//! `org.freedesktop.IBus.Engine` 구현. presguel-core 의 조합 엔진을 감싼다.
//!
//! 키 이벤트(method)를 받아 조합하고, 결과를 CommitText / UpdatePreeditText
//! (signal)로 데몬에 돌려준다. 참고: `research/03-ibus-zbus.md` §2,§4.

use std::collections::HashSet;

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

/// 수식어 키 자체(Shift/Ctrl/Caps/Meta/Alt/Super/Hyper, ISO_Level shifts, Mode_switch)인가.
/// 이런 키는 텍스트가 아니므로 조합에 영향을 주지 않고 그대로 통과시켜야 한다.
fn is_modifier_keysym(keyval: u32) -> bool {
    (0xffe1..=0xffee).contains(&keyval) // Shift_L..Hyper_R
        || (0xfe01..=0xfe0f).contains(&keyval) // ISO_Lock, ISO_LevelN_Shift 등
        || keyval == 0xff7e // Mode_switch (AltGr 류)
}

/// 날개셋 ShortcutTable 의 가상키(VK_*) 이름 → X11/ibus 키심.
fn vk_to_keysyms(vk: &str) -> &'static [u32] {
    match vk {
        "VK_HANGUL" => &[0xff31],        // Hangul (한/영)
        "VK_HANJA" => &[0xff34],         // Hangul_Hanja (한자)
        "VK_CAPITAL" => &[0xffe5],       // Caps_Lock
        "VK_SPACE" => &[0x20],
        "VK_RMENU" => &[0xffea],         // Alt_R (오른쪽 Alt, 한/영 대용)
        "VK_RCONTROL" => &[0xffe4],      // Control_R (한자 대용)
        _ => &[],
    }
}

/// 키 분류(순수 함수 결과). 라우팅 로직을 D-Bus 비동기와 분리해 단위 테스트한다.
#[derive(Debug, PartialEq, Eq)]
enum KeyClass {
    Release,
    ImeSwitch,
    Modifier,
    ShortcutCombo,
    Backspace,
    Printable(u8),
    FunctionKey,
}

/// IBus 엔진 인스턴스 하나.
pub struct IBusEngine {
    core: Core,
    /// 한글 조합 모드(true) / 영문 패스스루(false).
    hangul: bool,
    /// 한/영 전환(IME_SWITCH)을 일으키는 키심들(설정 ShortcutTable 에서 해석).
    ime_switch: HashSet<u32>,
}

impl IBusEngine {
    pub fn new(layout: Layout) -> Self {
        // 설정의 단축글쇠 중 usage=IME_SWITCH 인 것의 키심을 모은다. 한/영 키(0xff31)는
        // 설정에 없어도 항상 포함한다.
        let mut ime_switch: HashSet<u32> = HashSet::new();
        ime_switch.insert(KEY_HANGUL);
        for sc in &layout.shortcuts {
            if sc.usage == "IME_SWITCH" {
                ime_switch.extend(vk_to_keysyms(&sc.key).iter().copied());
            }
        }
        Self { core: Core::new(layout), hangul: true, ime_switch }
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

    /// 키 이벤트를 분류한다(순수 함수). `process_key_event` 가 이 결과로 분기한다.
    /// IME_SWITCH 는 release/수식어보다 먼저 본다 — CapsLock 은 수식어 키심이기도 하므로.
    fn classify(&self, keyval: u32, state: u32) -> KeyClass {
        if self.ime_switch.contains(&keyval) {
            return KeyClass::ImeSwitch;
        }
        if state & RELEASE_MASK != 0 {
            return KeyClass::Release;
        }
        if is_modifier_keysym(keyval) {
            return KeyClass::Modifier;
        }
        if state & SPECIAL_MODS != 0 {
            return KeyClass::ShortcutCombo;
        }
        if keyval == KEY_BACKSPACE {
            return KeyClass::Backspace;
        }
        if (0x20..=0x7e).contains(&keyval) {
            return KeyClass::Printable(keyval as u8);
        }
        KeyClass::FunctionKey
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
        let release = state & RELEASE_MASK != 0;
        match self.classify(keyval, state) {
            // IME_SWITCH(한/영·CapsLock 등): 눌림/뗌 모두 소비, 눌림에서만 토글.
            KeyClass::ImeSwitch => {
                if !release {
                    self.flush_commit(&se).await;
                    self.hangul = !self.hangul;
                }
                Ok(true)
            }
            // 뗌·수식어 키 자체: 조합에 영향 없이 통과.
            KeyClass::Release | KeyClass::Modifier => Ok(false),
            // Ctrl/Alt/Super/Meta 조합(단축키): 조합 확정 후 응용에 넘김.
            KeyClass::ShortcutCombo => {
                self.flush_commit(&se).await;
                Ok(false)
            }
            // 영문 패스스루 모드면 아래 한글 처리들을 모두 통과.
            _ if !self.hangul => Ok(false),
            // 백스페이스: 조합 중이면 낱자 단위로 되돌림, 아니면 응용에 넘김.
            KeyClass::Backspace => {
                if self.core.is_empty() {
                    return Ok(false);
                }
                let out = self.core.backspace();
                Self::emit(&se, &out.commit, &out.preedit).await;
                Ok(out.consumed)
            }
            // 인쇄 가능 ASCII(+ space): KeyTable 로 처리.
            KeyClass::Printable(ascii) => {
                let caps = state & LOCK_MASK != 0;
                let out = self.core.press(ascii, caps);
                Self::emit(&se, &out.commit, &out.preedit).await;
                Ok(out.consumed)
            }
            // 그 밖의 기능키(Enter/Esc/화살표 등): 조합 확정 후 통과.
            KeyClass::FunctionKey => {
                self.flush_commit(&se).await;
                Ok(false)
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use presguel_core::Config;

    // VK_HANGUL 과 VK_CAPITAL 을 IME_SWITCH 로 둔 최소 설정.
    const MINI: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<EditContextSetting version="0x500">
  <EditorLayer flag="0">
    <ShortcutTable>
      <Shortcut key="VK_HANGUL" usage="IME_SWITCH" value="!A"/>
      <Shortcut key="VK_CAPITAL" modifier="DONT_EAT|KEEP_LAMP" usage="IME_SWITCH" value="!A"/>
      <Shortcut key="VK_HANJA" usage="KEYCHAR" value="C0|0x82"/>
    </ShortcutTable>
    <FinalConvTable><FinalConv from="0x1100" to="0x3131"/></FinalConvTable>
  </EditorLayer>
  <InputLayer default="0" current="0">
    <InputEntry>
      <InputSchemeSetting object="CBasicInputScheme">
        <KeyTable name="mini" flag="0" from="33" to="126">
          <Key at="0x6B" value="H3|G_"/>
          <Key at="0x40" value="T ? H3|_RG : 0x40"/>
        </KeyTable>
      </InputSchemeSetting>
      <GeneratorSetting object="CNgsImeEx">
        <UnitMixTable/><VirtualUnitTable/><AutomataTable default="0"/>
      </GeneratorSetting>
    </InputEntry>
  </InputLayer>
</EditContextSetting>"#;

    fn engine() -> IBusEngine {
        let cfg = Config::parse(MINI).unwrap();
        IBusEngine::new(cfg.compile(0).unwrap())
    }

    #[test]
    fn capslock_in_switch_set() {
        let e = engine();
        // 설정의 VK_CAPITAL → Caps_Lock(0xffe5), VK_HANGUL → 0xff31
        assert!(e.ime_switch.contains(&0xffe5));
        assert!(e.ime_switch.contains(&0xff31));
        // VK_HANJA 는 KEYCHAR 라 전환 집합에 없어야 한다.
        assert!(!e.ime_switch.contains(&0xff34));
    }

    #[test]
    fn shift_is_modifier_not_function_key() {
        let e = engine();
        // 버그 재현 방지: Shift 는 Modifier(통과)여야지, FunctionKey(조합 확정)면 안 된다.
        assert_eq!(e.classify(0xffe1, 0), KeyClass::Modifier); // Shift_L
        assert_eq!(e.classify(0xffe2, 0), KeyClass::Modifier); // Shift_R
    }

    #[test]
    fn capslock_classifies_as_ime_switch_even_on_release() {
        let e = engine();
        assert_eq!(e.classify(0xffe5, 0), KeyClass::ImeSwitch);
        assert_eq!(e.classify(0xffe5, RELEASE_MASK), KeyClass::ImeSwitch);
    }

    #[test]
    fn hangul_key_is_ime_switch() {
        assert_eq!(engine().classify(0xff31, 0), KeyClass::ImeSwitch);
    }

    #[test]
    fn at_key_with_shift_is_printable() {
        // 실키 Shift+2 는 keyval 0x40('@') + SHIFT 상태로 도착 → 인쇄키로 분류되어 ㄺ 조합 가능.
        assert_eq!(engine().classify(0x40, 1 /*SHIFT*/), KeyClass::Printable(0x40));
    }

    #[test]
    fn ctrl_combo_is_shortcut() {
        assert_eq!(engine().classify(b'c' as u32, CONTROL_MASK), KeyClass::ShortcutCombo);
    }

    #[test]
    fn release_of_normal_key_ignored() {
        assert_eq!(engine().classify(b'k' as u32, RELEASE_MASK), KeyClass::Release);
    }

    #[test]
    fn backspace_classified() {
        assert_eq!(engine().classify(KEY_BACKSPACE, 0), KeyClass::Backspace);
    }
}
