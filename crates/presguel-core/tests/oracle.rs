//! 실제 `nalgaeset.xml`(저장소 바깥, 사용자 환경)에 대한 통합 검증.
//!
//! 설정 경로는 `PRESGUEL_TEST_CONFIG` 환경변수로 지정하거나, 없으면 이 머신의
//! provision 경로를 기본값으로 쓴다. 파일이 없으면 테스트를 건너뛴다(저장소에는
//! 사용자 설정을 포함하지 않으므로 CI 에서는 자연히 skip).

use std::path::PathBuf;

use presguel_core::config::Config;

fn config_path() -> Option<PathBuf> {
    let p = std::env::var("PRESGUEL_TEST_CONFIG")
        .unwrap_or_else(|_| "/home/nemo/git/lens/provision/config/nalgaeset.xml".to_string());
    let p = PathBuf::from(p);
    p.exists().then_some(p)
}

fn load() -> Option<Config> {
    let path = config_path()?;
    let xml = std::fs::read_to_string(&path).expect("read config");
    Some(Config::parse(&xml).expect("parse config"))
}

#[test]
fn real_config_parses() {
    let Some(cfg) = load() else {
        eprintln!("skip: nalgaeset.xml 없음");
        return;
    };
    assert_eq!(cfg.version, "0x500");
    assert_eq!(cfg.default_entry, 0);
    assert_eq!(cfg.entries.len(), 3);

    // 항목 0 = 세벌식-맞춤
    let e0 = &cfg.entries[0];
    assert_eq!(e0.scheme_object, "CBasicInputScheme");
    assert_eq!(e0.generator_object, "CNgsImeEx");
    let kt = e0.key_table.as_ref().expect("키 테이블");
    assert_eq!(kt.name, "세벌식-맞춤");
    assert_eq!(kt.from, 33);
    assert_eq!(kt.to, 126);
    // 0x21..=0x7E = 94 키 전부
    assert_eq!(kt.keys.len(), 94, "키 개수");

    // 항목 1 = 로마자 드보락 (식 `^(P&1)<<5` 들이 전부 파싱되어야 함)
    assert_eq!(cfg.entries[1].scheme_object, "CAdvancedScheme");
    assert!(cfg.entries[1].key_table.is_some());

    // 항목 2 = 패스스루
    assert_eq!(cfg.entries[2].scheme_object, "CInputScheme");
    assert!(cfg.entries[2].key_table.is_none());

    // FinalConvTable 전체
    assert!(cfg.editor.final_conv.len() > 150, "FinalConv 항목 수");
    assert_eq!(cfg.editor.final_conv.get(&0x1100), Some(&0x3131));
    assert_eq!(cfg.editor.final_conv.get(&0x11A8), Some(&0x3131));

    // 단축글쇠
    assert!(cfg.editor.shortcuts.iter().any(|s| s.key == "VK_HANGUL"));

    assert_eq!(cfg.first_hangul_entry(), Some(0));
}

#[test]
fn real_config_compiles() {
    let Some(cfg) = load() else {
        eprintln!("skip: nalgaeset.xml 없음");
        return;
    };
    let layout = cfg.compile(0).unwrap();
    assert_eq!(layout.name, "세벌식-맞춤");
    assert_eq!(layout.keys.len(), 94);

    // 갈마들이 5쌍: ㄱ↔ㄲ ㄷ↔ㄸ ㅂ↔ㅃ ㅅ↔ㅆ ㅈ↔ㅉ (토글 경로)
    use presguel_core::unit::TOGGLE;
    use presguel_core::Category::*;
    assert_eq!(layout.combine(Cho, 0x1100, TOGGLE), Some(0x1101)); // ㄱ→ㄲ
    assert_eq!(layout.combine(Cho, 0x1101, TOGGLE), Some(0x1100)); // ㄲ→ㄱ
    // 겹모음 6개
    assert_eq!(layout.combine(Jung, 0x1169, 0x1161), Some(0x116A)); // ㅗ+ㅏ→ㅘ
    assert_eq!(layout.combine(Jung, 0x116E, 0x1165), Some(0x116F)); // ㅜ+ㅓ→ㅝ
    // 겹받침 3개 (RS/RT/RP)
    assert_eq!(layout.combine(Jong, 0x11AF, 0x11BA), Some(0x11B3)); // ㄹ+ㅅ→ㄽ

    // 가상 단위 128/129/130 = ㅗ/ㅜ/ㅡ
    use presguel_core::Jamo;
    assert_eq!(layout.virtual_units.get(&128), Some(&Jamo::new(Jung, 0x1169)));
    assert_eq!(layout.virtual_units.get(&129), Some(&Jamo::new(Jung, 0x116E)));
    assert_eq!(layout.virtual_units.get(&130), Some(&Jamo::new(Jung, 0x1173)));
}
