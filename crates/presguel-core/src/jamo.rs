//! 한글 자모 유니코드 사실(facts): 조합용 자모(첫가끝, U+1100), 완성형 음절(U+AC00)
//! 공식, 호환 자모(U+3130). 설정과 무관한 순수 유니코드 계층.
//!
//! 참고: `research/04-hangul-unicode.md`.

/// 완성형 음절 시작 (가, U+AC00).
pub const SBASE: u32 = 0xAC00;
/// 조합용 초성 시작 (U+1100).
pub const LBASE: u32 = 0x1100;
/// 조합용 중성 시작 (U+1161).
pub const VBASE: u32 = 0x1161;
/// 조합용 종성 기준 (U+11A7 = U+11A8 - 1, 그래서 종성 인덱스 0 = "받침 없음").
pub const TBASE: u32 = 0x11A7;

/// 현대 초성 개수.
pub const LCOUNT: u32 = 19;
/// 현대 중성 개수.
pub const VCOUNT: u32 = 21;
/// 종성 개수(받침 없음 1개 포함).
pub const TCOUNT: u32 = 28;
/// 완성형 음절 총 개수 (U+AC00..=U+D7A3).
pub const SCOUNT: u32 = LCOUNT * VCOUNT * TCOUNT; // 11172

/// 현대 초성(choIdx 0..=18)의 조합용 자모 코드포인트.
pub const CHO: [u32; 19] = [
    0x1100, 0x1101, 0x1102, 0x1103, 0x1104, 0x1105, 0x1106, 0x1107, 0x1108, 0x1109, 0x110A, 0x110B,
    0x110C, 0x110D, 0x110E, 0x110F, 0x1110, 0x1111, 0x1112,
];

/// 현대 중성(jungIdx 0..=20)의 조합용 자모 코드포인트.
pub const JUNG: [u32; 21] = [
    0x1161, 0x1162, 0x1163, 0x1164, 0x1165, 0x1166, 0x1167, 0x1168, 0x1169, 0x116A, 0x116B, 0x116C,
    0x116D, 0x116E, 0x116F, 0x1170, 0x1171, 0x1172, 0x1173, 0x1174, 0x1175,
];

/// 종성(jongIdx 0..=27)의 조합용 자모 코드포인트. 인덱스 0 = 받침 없음(코드포인트 없음).
pub const JONG: [u32; 28] = [
    0x0000, // 0 = 받침 없음
    0x11A8, 0x11A9, 0x11AA, 0x11AB, 0x11AC, 0x11AD, 0x11AE, 0x11AF, 0x11B0, 0x11B1, 0x11B2, 0x11B3,
    0x11B4, 0x11B5, 0x11B6, 0x11B7, 0x11B8, 0x11B9, 0x11BA, 0x11BB, 0x11BC, 0x11BD, 0x11BE, 0x11BF,
    0x11C0, 0x11C1, 0x11C2,
];

/// 조합용 초성 코드포인트 → 현대 초성 인덱스(0..=18).
pub fn cho_index(cp: u32) -> Option<u8> {
    (LBASE..=0x1112).contains(&cp).then(|| (cp - LBASE) as u8)
}

/// 조합용 중성 코드포인트 → 현대 중성 인덱스(0..=20).
pub fn jung_index(cp: u32) -> Option<u8> {
    (VBASE..=0x1175).contains(&cp).then(|| (cp - VBASE) as u8)
}

/// 조합용 종성 코드포인트 → 종성 인덱스(1..=27). "받침 없음"(0)은 코드포인트가 없으므로 제외.
pub fn jong_index(cp: u32) -> Option<u8> {
    (0x11A8..=0x11C2).contains(&cp).then(|| (cp - TBASE) as u8)
}

/// 현대 초/중/종성 인덱스로 완성형 음절 글자를 만든다. jong=0 이면 받침 없음.
pub fn compose_indices(cho: u8, jung: u8, jong: u8) -> Option<char> {
    if (cho as u32) >= LCOUNT || (jung as u32) >= VCOUNT || (jong as u32) >= TCOUNT {
        return None;
    }
    let s = SBASE + ((cho as u32 * VCOUNT) + jung as u32) * TCOUNT + jong as u32;
    char::from_u32(s)
}

/// 조합용 자모 코드포인트들로 완성형 음절을 만든다. 모두 현대 집합일 때만 성공.
/// `jong_cp`가 `None` 또는 0 이면 받침 없음.
pub fn compose(cho_cp: u32, jung_cp: u32, jong_cp: Option<u32>) -> Option<char> {
    let cho = cho_index(cho_cp)?;
    let jung = jung_index(jung_cp)?;
    let jong = match jong_cp {
        None | Some(0) => 0,
        Some(cp) => jong_index(cp)?,
    };
    compose_indices(cho, jung, jong)
}

/// 완성형 음절을 (초성, 중성, 종성) 조합용 코드포인트로 분해. 종성 없으면 마지막이 None.
pub fn decompose(syllable: char) -> Option<(u32, u32, Option<u32>)> {
    let s = syllable as u32;
    if !(SBASE..SBASE + SCOUNT).contains(&s) {
        return None;
    }
    let idx = s - SBASE;
    let cho = idx / (VCOUNT * TCOUNT);
    let jung = (idx % (VCOUNT * TCOUNT)) / TCOUNT;
    let jong = idx % TCOUNT;
    Some((
        CHO[cho as usize],
        JUNG[jung as usize],
        (jong != 0).then(|| JONG[jong as usize]),
    ))
}

/// 코드포인트가 조합용 자모 블록(첫가끝 + 확장 A/B)에 속하는지.
pub fn is_conjoining_jamo(cp: u32) -> bool {
    (0x1100..=0x11FF).contains(&cp) // Hangul Jamo
        || (0xA960..=0xA97F).contains(&cp) // Extended-A (옛 초성)
        || (0xD7B0..=0xD7FF).contains(&cp) // Extended-B (옛 중성/종성)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precompose_basic() {
        // 가 = ㄱ(U+1100) + ㅏ(U+1161), 받침 없음
        assert_eq!(compose(0x1100, 0x1161, None), Some('가'));
        // 각 = ㄱ + ㅏ + ㄱ(종성 U+11A8)
        assert_eq!(compose(0x1100, 0x1161, Some(0x11A8)), Some('각'));
        // 한 = ㅎ(U+1112) + ㅏ + ㄴ(U+11AB)
        assert_eq!(compose(0x1112, 0x1161, Some(0x11AB)), Some('한'));
        // 과 = ㄱ + ㅘ(U+116A)
        assert_eq!(compose(0x1100, 0x116A, None), Some('과'));
        // 까 = ㄲ(U+1101) + ㅏ
        assert_eq!(compose(0x1101, 0x1161, None), Some('까'));
    }

    #[test]
    fn precompose_indices_match_formula() {
        // 가 = index (0,0,0)
        assert_eq!(compose_indices(0, 0, 0), Some('가'));
        // 힣 = 마지막 음절 (18,20,27)
        assert_eq!(compose_indices(18, 20, 27), Some('힣'));
        assert_eq!(SBASE + SCOUNT - 1, '힣' as u32);
    }

    #[test]
    fn roundtrip_decompose() {
        for ch in ['가', '각', '한', '뷁', '힣', '꿈', '워', '의'] {
            let (c, j, t) = decompose(ch).unwrap();
            assert_eq!(compose(c, j, t), Some(ch), "roundtrip {ch}");
        }
    }

    #[test]
    fn index_lookups() {
        assert_eq!(cho_index(0x1100), Some(0));
        assert_eq!(cho_index(0x1112), Some(18));
        assert_eq!(cho_index(0x1113), None);
        assert_eq!(jung_index(0x1161), Some(0));
        assert_eq!(jung_index(0x1175), Some(20));
        assert_eq!(jong_index(0x11A8), Some(1));
        assert_eq!(jong_index(0x11C2), Some(27));
        assert_eq!(jong_index(0x1100), None);
    }

    #[test]
    fn non_modern_has_no_precompose() {
        // 옛이응 초성 U+114C 는 현대 집합 밖 → 완성형 불가
        assert_eq!(compose(0x114C, 0x1161, None), None);
    }
}
