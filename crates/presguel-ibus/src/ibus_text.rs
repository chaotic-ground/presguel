//! IBusText / IBusAttrList / IBusAttribute 의 D-Bus(zvariant) 직렬화.
//!
//! IBus 직렬화 타입은 모두 `(s a{sv} ...)` 헤더로 시작한다(첫 `s`=GType 이름,
//! `a{sv}`=부속물 dict, 보통 비어 있음). 정확한 시그니처:
//! - `IBusText      : (sa{sv}sv)`  이름, 부속물, 본문 s, 속성목록 variant
//! - `IBusAttrList  : (sa{sv}av)`  이름, 부속물, 속성 variant 배열(**av**, `a(uuuu)` 아님)
//! - `IBusAttribute : (sa{sv}uuuu)` 이름, 부속물, type, value, start, end
//!
//! 속성 배열을 `a(uuuu)` 로 보내면 ibus-daemon 이 역직렬화에 실패해 죽을 수 있다
//! (ibus/ibus#2611). 참고: `research/03-ibus-zbus.md` §3.

use std::collections::HashMap;

use zbus::zvariant::{Structure, Value};

/// IBus 속성 타입.
pub const IBUS_ATTR_TYPE_UNDERLINE: u32 = 1;
#[allow(dead_code)] // 프로토콜 상수(전경/배경색); 추후 사용.
pub const IBUS_ATTR_TYPE_FOREGROUND: u32 = 2;
#[allow(dead_code)]
pub const IBUS_ATTR_TYPE_BACKGROUND: u32 = 3;
/// 밑줄 스타일.
pub const IBUS_ATTR_UNDERLINE_SINGLE: u32 = 1;

fn empty_attachments() -> HashMap<String, Value<'static>> {
    HashMap::new()
}

/// 속성 없는 IBusText. 시그니처 `(sa{sv}sv)`.
pub fn make_ibus_text(text: impl Into<String>) -> Value<'static> {
    let attr_list = Structure::from((
        "IBusAttrList",
        empty_attachments(),
        Vec::<Value<'static>>::new(), // av = []
    ));
    Value::new(Structure::from((
        "IBusText",
        empty_attachments(),
        text.into(),
        Value::new(attr_list),
    )))
}

/// 직렬화된 IBusAttribute 하나. 시그니처 `(sa{sv}uuuu)`.
fn ibus_attribute(attr_type: u32, value: u32, start: u32, end: u32) -> Value<'static> {
    Value::new(Structure::from((
        "IBusAttribute",
        empty_attachments(),
        attr_type,
        value,
        start,
        end,
    )))
}

/// 전체에 홑밑줄을 친 IBusText(전형적인 한글 preedit). 시그니처 `(sa{sv}sv)`.
pub fn make_preedit_text(text: impl Into<String>) -> Value<'static> {
    let text = text.into();
    let char_len = text.chars().count() as u32;

    let attrs: Vec<Value<'static>> = if char_len > 0 {
        vec![ibus_attribute(
            IBUS_ATTR_TYPE_UNDERLINE,
            IBUS_ATTR_UNDERLINE_SINGLE,
            0,
            char_len,
        )]
    } else {
        Vec::new()
    };

    let attr_list = Structure::from(("IBusAttrList", empty_attachments(), attrs));

    Value::new(Structure::from((
        "IBusText",
        empty_attachments(),
        text,
        Value::new(attr_list),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ibus_text_signature() {
        let v = make_ibus_text("test");
        assert_eq!(v.value_signature().to_string(), "(sa{sv}sv)");
    }

    #[test]
    fn preedit_text_signature() {
        // 속성이 있어도 바깥 시그니처는 동일해야 한다.
        let v = make_preedit_text("가");
        assert_eq!(v.value_signature().to_string(), "(sa{sv}sv)");
        let empty = make_preedit_text("");
        assert_eq!(empty.value_signature().to_string(), "(sa{sv}sv)");
    }
}
