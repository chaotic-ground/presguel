//! 날개셋 낱자(단위) 모델과 니모닉/operand 해석.
//!
//! `H3|<operand>` 의 operand 는 니모닉(`_GG`, `O_`, `RS`)이거나 숫자(`0x1F4`,
//! `0x810000`)다. 니모닉은 위치(초/중/종)와 글자 정체를, 숫자는 갈마들이 토글(500)이나
//! 가상 단위(`id<<16`)를 나타낸다. 참고: `research/01-nalgaeset-format.md` §2,§7.

use crate::jamo;

/// 자모의 위치(낱자 갈래).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Category {
    /// 초성.
    Cho,
    /// 중성.
    Jung,
    /// 종성(받침).
    Jong,
}

/// 해결된 한글 자모 단위: 위치 + 조합용 자모 코드포인트.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Jamo {
    pub category: Category,
    /// 조합용 자모 코드포인트(U+1100 영역, 또는 옛한글/확장 영역).
    pub cp: u32,
}

impl Jamo {
    pub fn new(category: Category, cp: u32) -> Self {
        Self { category, cp }
    }

    /// 현대 자모 집합에 속하면 그 호환 자모(U+31xx)를 돌려준다. 옛한글이면 `None`
    /// (이 경우 설정의 FinalConvTable 에 의존).
    pub fn default_compat(&self) -> Option<u32> {
        match self.category {
            Category::Cho => jamo::cho_index(self.cp).map(|i| CHO_COMPAT[i as usize]),
            Category::Jung => jamo::jung_index(self.cp).map(|i| JUNG_COMPAT[i as usize]),
            Category::Jong => jamo::jong_index(self.cp).map(|i| JONG_COMPAT[i as usize]),
        }
    }
}

/// `H3|<operand>` 가 가리키는 단위.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit {
    /// 보통의 한글 자모.
    Jamo(Jamo),
    /// 갈마들이 같은-키 토글 sentinel (500 = 0x1F4).
    Toggle,
    /// 미해결 가상 단위 id (VirtualUnitTable 로 해결해야 함). 예: 128/129/130.
    Virtual(u32),
}

/// 갈마들이 토글을 나타내는 내부 단위 값.
pub const TOGGLE: u32 = 500;

// ── 호환 자모(U+31xx) 표: 조합용 배열과 인덱스가 1:1 대응 ─────────────────────

/// 현대 초성 인덱스 → 호환 자모.
pub const CHO_COMPAT: [u32; 19] = [
    0x3131, 0x3132, 0x3134, 0x3137, 0x3138, 0x3139, 0x3141, 0x3142, 0x3143, 0x3145, 0x3146, 0x3147,
    0x3148, 0x3149, 0x314A, 0x314B, 0x314C, 0x314D, 0x314E,
];

/// 현대 중성 인덱스 → 호환 자모(U+314F..=U+3163, 연속).
pub const JUNG_COMPAT: [u32; 21] = [
    0x314F, 0x3150, 0x3151, 0x3152, 0x3153, 0x3154, 0x3155, 0x3156, 0x3157, 0x3158, 0x3159, 0x315A,
    0x315B, 0x315C, 0x315D, 0x315E, 0x315F, 0x3160, 0x3161, 0x3162, 0x3163,
];

/// 종성 인덱스(0=없음) → 호환 자모. 0 자리는 0.
pub const JONG_COMPAT: [u32; 28] = [
    0x0000, 0x3131, 0x3132, 0x3133, 0x3134, 0x3135, 0x3136, 0x3137, 0x3139, 0x313A, 0x313B, 0x313C,
    0x313D, 0x313E, 0x313F, 0x3140, 0x3141, 0x3142, 0x3144, 0x3145, 0x3146, 0x3147, 0x3148, 0x314A,
    0x314B, 0x314C, 0x314D, 0x314E,
];

fn cho_cp_for_compat(compat: u32) -> Option<u32> {
    CHO_COMPAT
        .iter()
        .position(|&c| c == compat)
        .map(|i| jamo::CHO[i])
}
fn jung_cp_for_compat(compat: u32) -> Option<u32> {
    JUNG_COMPAT
        .iter()
        .position(|&c| c == compat)
        .map(|i| jamo::JUNG[i])
}
fn jong_cp_for_compat(compat: u32) -> Option<u32> {
    // 인덱스 0 (받침 없음)은 제외.
    JONG_COMPAT
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, &c)| c == compat)
        .map(|(i, _)| jamo::JONG[i])
}

fn is_vowel_compat(compat: u32) -> bool {
    (0x314F..=0x3163).contains(&compat)
}

/// 초성 코드포인트를 같은 자음의 종성 코드포인트로 바꾼다(대응 없으면 None).
/// 예: ㄱ초성(U+1100) → ㄱ종성(U+11A8). C0 특수글쇠(초·종성 맞바꾸기 등)에서 쓴다.
pub fn cho_to_jong(cho_cp: u32) -> Option<u32> {
    let compat = jamo::cho_index(cho_cp).map(|i| CHO_COMPAT[i as usize])?;
    jong_cp_for_compat(compat)
}

/// 종성 코드포인트를 같은 자음의 초성 코드포인트로 바꾼다(대응 없으면 None).
/// 예: ㅇ종성(U+11BC) → ㅇ초성(U+110B). 도깨비불·초종성 맞바꾸기에서 쓴다.
pub fn jong_to_cho(jong_cp: u32) -> Option<u32> {
    let compat = jamo::jong_index(jong_cp).map(|i| JONG_COMPAT[i as usize])?;
    cho_cp_for_compat(compat)
}

/// 니모닉의 "핵심 토큰"(밑줄 제거) → 호환 자모(글자 정체). 위치 무관.
fn mnemonic_to_compat(core: &str) -> Option<u32> {
    Some(match core {
        // 자음 (단일)
        "G" => 0x3131,
        "N" => 0x3134,
        "D" => 0x3137,
        "R" | "L" => 0x3139,
        "M" => 0x3141,
        "B" => 0x3142,
        "S" => 0x3145,
        "Q" | "NG" => 0x3147,
        "J" => 0x3148,
        "C" => 0x314A,
        "K" => 0x314B,
        "T" => 0x314C,
        "P" => 0x314D,
        "H" => 0x314E,
        // 자음 (쌍/겹)
        "GG" => 0x3132,
        "GS" => 0x3133,
        "NJ" => 0x3135,
        "NH" => 0x3136,
        "DD" => 0x3138,
        "RG" => 0x313A,
        "RM" => 0x313B,
        "RB" => 0x313C,
        "RS" => 0x313D,
        "RT" => 0x313E,
        "RP" => 0x313F,
        "RH" => 0x3140,
        "BB" => 0x3143,
        "BS" => 0x3144,
        "SS" => 0x3146,
        "JJ" => 0x3149,
        // 모음
        "A" => 0x314F,
        "AE" => 0x3150,
        "YA" => 0x3151,
        "YAE" => 0x3152,
        "EO" => 0x3153,
        "E" => 0x3154,
        "YEO" => 0x3155,
        "YE" => 0x3156,
        "O" => 0x3157,
        "WA" => 0x3158,
        "WAE" => 0x3159,
        "OI" => 0x315A,
        "YO" => 0x315B,
        "U" => 0x315C,
        "UEO" => 0x315D,
        "WE" => 0x315E,
        "WI" => 0x315F,
        "YU" => 0x3160,
        "EU" => 0x3161,
        "EUI" => 0x3162,
        "I" => 0x3163,
        _ => return None,
    })
}

/// 니모닉을 단위로 해석. `ctx` 가 주어지면(UnitMix 처럼) 그 위치를 강제하고, 없으면
/// 밑줄 위치(`_X`=종성, `X_`=초성/모음)와 글자 정체로 추론한다.
pub fn resolve_mnemonic(s: &str, ctx: Option<Category>) -> Option<Unit> {
    let (core, pos_category): (&str, Option<Category>) = if let Some(rest) = s.strip_prefix('_') {
        (rest, Some(Category::Jong))
    } else if let Some(rest) = s.strip_suffix('_') {
        (rest, None) // 초성 또는 모음 (글자 정체로 결정)
    } else {
        (s, None)
    };
    let compat = mnemonic_to_compat(core)?;
    let category = ctx.or(pos_category).unwrap_or_else(|| {
        if is_vowel_compat(compat) {
            Category::Jung
        } else {
            Category::Cho
        }
    });
    let cp = match category {
        Category::Cho => cho_cp_for_compat(compat)?,
        Category::Jung => jung_cp_for_compat(compat)?,
        Category::Jong => jong_cp_for_compat(compat)?,
    };
    Some(Unit::Jamo(Jamo::new(category, cp)))
}

/// 숫자 operand 를 단위로 해석.
/// - 500 → 갈마들이 토글.
/// - `id<<16` (하위 16비트 0, 상위 비0) → 가상 단위 id.
/// - 그 외 → 조합용 자모 코드포인트로 보고 영역으로 위치 추론(옛한글 포함).
pub fn resolve_numeric(n: u32) -> Option<Unit> {
    if n == TOGGLE {
        return Some(Unit::Toggle);
    }
    if n & 0xFFFF == 0 && n >> 16 != 0 {
        return Some(Unit::Virtual(n >> 16));
    }
    category_of_codepoint(n).map(|cat| Unit::Jamo(Jamo::new(cat, n)))
}

/// 조합용 자모 코드포인트의 위치를 블록 범위로 추론(옛한글/확장 포함).
pub fn category_of_codepoint(cp: u32) -> Option<Category> {
    match cp {
        0x1100..=0x115F => Some(Category::Cho), // 초성(현대+옛) + 초성 채움
        0x1160..=0x11A7 => Some(Category::Jung), // 중성 채움 + 중성(현대+옛)
        0x11A8..=0x11FF => Some(Category::Jong), // 종성(현대+옛)
        0xA960..=0xA97F => Some(Category::Cho), // 확장-A: 옛 초성
        0xD7B0..=0xD7CA => Some(Category::Jung), // 확장-B: 옛 중성
        0xD7CB..=0xD7FF => Some(Category::Jong), // 확장-B: 옛 종성
        _ => None,
    }
}

/// `H3|<operand>` operand 문자열(니모닉 또는 숫자)을 단위로 해석.
pub fn resolve_operand(s: &str, ctx: Option<Category>) -> Option<Unit> {
    let s = s.trim();
    if let Some(n) = parse_int(s) {
        resolve_numeric(n)
    } else {
        resolve_mnemonic(s, ctx)
    }
}

/// `0x..` 16진 또는 10진 정수 파싱.
pub fn parse_int(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jamo_of(u: Unit) -> Jamo {
        match u {
            Unit::Jamo(j) => j,
            other => panic!("expected jamo, got {other:?}"),
        }
    }

    #[test]
    fn keytable_mnemonics() {
        // 초성: k=G_ → ㄱ 초성 U+1100
        assert_eq!(
            jamo_of(resolve_mnemonic("G_", None).unwrap()),
            Jamo::new(Category::Cho, 0x1100)
        );
        // 종성: x=_G → ㄱ 종성 U+11A8
        assert_eq!(
            jamo_of(resolve_mnemonic("_G", None).unwrap()),
            Jamo::new(Category::Jong, 0x11A8)
        );
        // 중성: f=A_ → ㅏ U+1161, / = O_ → ㅗ U+1169
        assert_eq!(
            jamo_of(resolve_mnemonic("A_", None).unwrap()),
            Jamo::new(Category::Jung, 0x1161)
        );
        assert_eq!(
            jamo_of(resolve_mnemonic("O_", None).unwrap()),
            Jamo::new(Category::Jung, 0x1169)
        );
        // 바른 중성 bare: 8=EUI → ㅢ U+1174
        assert_eq!(
            jamo_of(resolve_mnemonic("EUI", None).unwrap()),
            Jamo::new(Category::Jung, 0x1174)
        );
        // 겹받침 종성: @=_RG → ㄺ U+11B0
        assert_eq!(
            jamo_of(resolve_mnemonic("_RG", None).unwrap()),
            Jamo::new(Category::Jong, 0x11B0)
        );
        // 초성 ㅇ: j=Q_ → U+110B
        assert_eq!(
            jamo_of(resolve_mnemonic("Q_", None).unwrap()),
            Jamo::new(Category::Cho, 0x110B)
        );
        // 종성 ㅇ: a=_Q → U+11BC
        assert_eq!(
            jamo_of(resolve_mnemonic("_Q", None).unwrap()),
            Jamo::new(Category::Jong, 0x11BC)
        );
    }

    #[test]
    fn unitmix_context_mnemonics() {
        // UnitMix JONG: R_ + S_ → RS, 모두 종성 ctx
        assert_eq!(
            jamo_of(resolve_mnemonic("R_", Some(Category::Jong)).unwrap()),
            Jamo::new(Category::Jong, 0x11AF) // ㄹ 종성
        );
        assert_eq!(
            jamo_of(resolve_mnemonic("S_", Some(Category::Jong)).unwrap()),
            Jamo::new(Category::Jong, 0x11BA) // ㅅ 종성
        );
        assert_eq!(
            jamo_of(resolve_mnemonic("RS", Some(Category::Jong)).unwrap()),
            Jamo::new(Category::Jong, 0x11B3) // ㄽ
        );
        // UnitMix CHO: GG → ㄲ 초성 U+1101
        assert_eq!(
            jamo_of(resolve_mnemonic("GG", Some(Category::Cho)).unwrap()),
            Jamo::new(Category::Cho, 0x1101)
        );
        // UnitMix JUNG: WA → ㅘ U+116A
        assert_eq!(
            jamo_of(resolve_mnemonic("WA", Some(Category::Jung)).unwrap()),
            Jamo::new(Category::Jung, 0x116A)
        );
    }

    #[test]
    fn numeric_operands() {
        // 0x1F4 = 500 = 갈마들이 토글
        assert_eq!(resolve_operand("0x1F4", None), Some(Unit::Toggle));
        assert_eq!(resolve_operand("500", None), Some(Unit::Toggle));
        // 0x800000 = 128<<16 = 가상 단위 128, 0x810000=129, 0x820000=130
        assert_eq!(resolve_operand("0x800000", None), Some(Unit::Virtual(128)));
        assert_eq!(resolve_operand("0x810000", None), Some(Unit::Virtual(129)));
        assert_eq!(resolve_operand("0x820000", None), Some(Unit::Virtual(130)));
    }

    #[test]
    fn raw_old_hangul_codepoint() {
        // 옛이응 초성 U+114C → 초성으로 분류
        assert_eq!(
            resolve_operand("0x114C", None),
            Some(Unit::Jamo(Jamo::new(Category::Cho, 0x114C)))
        );
        // 아래아 중성 U+119E → 중성
        assert_eq!(
            resolve_operand("0x119E", None),
            Some(Unit::Jamo(Jamo::new(Category::Jung, 0x119E)))
        );
    }

    #[test]
    fn default_compat_roundtrip() {
        assert_eq!(
            Jamo::new(Category::Cho, 0x1100).default_compat(),
            Some(0x3131)
        );
        assert_eq!(
            Jamo::new(Category::Jong, 0x11A8).default_compat(),
            Some(0x3131)
        );
        assert_eq!(
            Jamo::new(Category::Jung, 0x1161).default_compat(),
            Some(0x314F)
        );
        assert_eq!(
            Jamo::new(Category::Jong, 0x11B0).default_compat(),
            Some(0x313A)
        );
    }
}
