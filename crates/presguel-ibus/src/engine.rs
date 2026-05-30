//! `org.freedesktop.IBus.Engine` 구현. presguel-core 의 조합 엔진을 감싼다.
//!
//! 키 이벤트(method)를 받아 조합하고, 결과를 CommitText / UpdatePreeditText
//! (signal)로 데몬에 돌려준다. 참고: `research/03-ibus-zbus.md` §2,§4.

use presguel_core::Engine as Core;
use presguel_core::Layout;
use zbus::object_server::SignalEmitter;
use zbus::{fdo, interface};
use zbus::zvariant::Value;

use crate::ibus_text::{make_ibus_text, make_preedit_text};

// 수식어/키 마스크 (research/03 §4, 실측).
const RELEASE_MASK: u32 = 1 << 30;
const LOCK_MASK: u32 = 1 << 1; // Caps Lock
const CONTROL_MASK: u32 = 1 << 2;
const MOD1_MASK: u32 = 1 << 3; // Alt
const SUPER_MASK: u32 = 1 << 26;
const META_MASK: u32 = 1 << 28;
const SPECIAL_MODS: u32 = CONTROL_MASK | MOD1_MASK | SUPER_MASK | META_MASK;

// 키심(keysym).
const KEY_BACKSPACE: u32 = 0xff08;
const KEY_HANGUL: u32 = 0xff31;

/// IBus 엔진 인스턴스 하나.
pub struct IBusEngine {
    core: Core,
    /// 한글 조합 모드(true) / 영문 패스스루(false).
    hangul: bool,
}

impl IBusEngine {
    pub fn new(layout: Layout) -> Self {
        Self { core: Core::new(layout), hangul: true }
    }

    /// 확정 문자열과 preedit 를 신호로 내보낸다.
    async fn emit(se: &SignalEmitter<'_>, commit: &str, preedit: &str) {
        if !commit.is_empty() {
            let _ = Self::commit_text(se, make_ibus_text(commit.to_string())).await;
        }
        let cursor = preedit.chars().count() as u32;
        let _ = Self::update_preedit_text(
            se,
            make_preedit_text(preedit.to_string()),
            cursor,
            !preedit.is_empty(),
            0, // IBusPreeditFocusMode::CLEAR
        )
        .await;
    }

    /// 현재 조합을 확정해 내보내고 비운다.
    async fn flush_commit(&mut self, se: &SignalEmitter<'_>) {
        let commit = self.core.flush();
        if !commit.is_empty() {
            Self::emit(se, &commit, "").await;
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl IBusEngine {
    async fn process_key_event(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        keyval: u32,
        _keycode: u32,
        state: u32,
    ) -> fdo::Result<bool> {
        // 눌림(press)만 처리, 뗌(release)은 무시.
        if state & RELEASE_MASK != 0 {
            return Ok(false);
        }

        // 한/영 전환 키.
        if keyval == KEY_HANGUL {
            self.flush_commit(&se).await;
            self.hangul = !self.hangul;
            return Ok(true);
        }

        // Ctrl/Alt/Super/Meta 조합은 단축키 → 조합 확정 후 응용에 넘김.
        if state & SPECIAL_MODS != 0 {
            self.flush_commit(&se).await;
            return Ok(false);
        }

        // 영문 패스스루 모드.
        if !self.hangul {
            return Ok(false);
        }

        // 백스페이스: 조합 중이면 낱자 단위로 되돌림.
        if keyval == KEY_BACKSPACE {
            if self.core.is_empty() {
                return Ok(false);
            }
            let out = self.core.backspace();
            Self::emit(&se, &out.commit, &out.preedit).await;
            return Ok(out.consumed);
        }

        // 인쇄 가능 ASCII(+ space 0x20): KeyTable 로 처리.
        if (0x20..=0x7e).contains(&keyval) {
            let caps = state & LOCK_MASK != 0;
            let out = self.core.press(keyval as u8, caps);
            Self::emit(&se, &out.commit, &out.preedit).await;
            return Ok(out.consumed);
        }

        // 그 밖의 기능키(Enter/Esc/화살표/Hanja 등): 조합 확정 후 통과.
        self.flush_commit(&se).await;
        Ok(false)
    }

    async fn focus_in(&mut self) -> fdo::Result<()> {
        Ok(())
    }

    async fn focus_out(&mut self, #[zbus(signal_emitter)] se: SignalEmitter<'_>) -> fdo::Result<()> {
        self.flush_commit(&se).await;
        Ok(())
    }

    async fn reset(&mut self, #[zbus(signal_emitter)] se: SignalEmitter<'_>) -> fdo::Result<()> {
        self.core.reset();
        Self::emit(&se, "", "").await;
        Ok(())
    }

    async fn enable(&mut self) -> fdo::Result<()> {
        Ok(())
    }

    async fn disable(&mut self, #[zbus(signal_emitter)] se: SignalEmitter<'_>) -> fdo::Result<()> {
        self.flush_commit(&se).await;
        Ok(())
    }

    fn set_capabilities(&mut self, _caps: u32) {}

    fn set_cursor_location(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    fn property_activate(&mut self, _name: String, _state: u32) {}

    fn page_up(&mut self) {}
    fn page_down(&mut self) {}
    fn cursor_up(&mut self) {}
    fn cursor_down(&mut self) {}
    fn candidate_clicked(&mut self, _index: u32, _button: u32, _state: u32) {}

    // ── 신호(engine → daemon) ────────────────────────────────────────────────

    #[zbus(signal)]
    async fn commit_text(se: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        se: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn forward_key_event(
        se: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;
}
