//! 자판 설정 XML 을 날개셋 reverse-spec 스키마(no-namespace XSD)로 검증하는
//! 개발/CI 린트. 런타임 의존성이 아니라 테스트 전용이며, `xmllint`(libxml2)가
//! PATH 에 있을 때만 실행한다.
//!
//! 검증 대상:
//!  1. `tests/fixtures/*.xml` — 저장소 동봉 clean-room 예제(.set/.ist/.key). 항상 검사.
//!  2. 사용자 실제 설정 — `PRESGUEL_TEST_CONFIG` 또는 provision `layout.xml` 가
//!     있으면 추가 검사(없으면 skip).
//!
//! xmllint 가 없으면 기본적으로 skip 하되, 그 사실을 항상 stderr 로 알린다.
//! CI 에서 "린트가 조용히 사라지는" 것을 막으려면 `PRESGUEL_REQUIRE_XMLLINT=1` 로
//! 강제하면 xmllint 부재 시 테스트가 실패한다.
//!
//! 스키마 출처: chaotic-ground/nalgaeset-reverse-spec (CC BY 4.0). `schema/` 참조.

use std::path::{Path, PathBuf};
use std::process::Command;

fn xmllint_available() -> bool {
    Command::new("xmllint")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// xmllint 가 있으면 true. 없으면 skip 신호(false)를 돌려주되, `PRESGUEL_REQUIRE_XMLLINT`
/// 가 설정돼 있으면 패닉(CI 강제). "조용히 통과"가 아니라 항상 가시적으로 알린다.
fn xmllint_or_skip(test: &str) -> bool {
    if xmllint_available() {
        return true;
    }
    assert!(
        std::env::var_os("PRESGUEL_REQUIRE_XMLLINT").is_none(),
        "{test}: xmllint(libxml2) 가 없는데 PRESGUEL_REQUIRE_XMLLINT 가 설정됨. \
         CI 에서 스키마 린트가 사라지지 않도록 libxml2-utils(xmllint) 를 설치하라."
    );
    eprintln!("skip({test}): xmllint(libxml2) 없음. PRESGUEL_REQUIRE_XMLLINT=1 로 강제 가능");
    false
}

/// 저장소 루트의 vendored no-namespace XSD 경로.
fn schema_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/nalgaeset-no-namespace.xsd")
}

/// 스키마 경로를 돌려주되, 없으면 즉시 명확히 실패시킨다.
fn require_schema() -> PathBuf {
    let p = schema_path();
    assert!(p.exists(), "vendored 스키마 없음: {}", p.display());
    p
}

fn validate(xsd: &Path, xml: &Path) -> Result<(), String> {
    let out = Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(xsd)
        .arg(xml)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[test]
fn fixtures_validate_against_schema() {
    if !xmllint_or_skip("fixtures") {
        return;
    }
    let xsd = require_schema();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut checked = 0;
    let mut fails = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("fixtures 디렉터리") {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("xml") {
            continue;
        }
        checked += 1;
        if let Err(e) = validate(&xsd, &p) {
            fails.push(format!(
                "  {}:\n{}",
                p.file_name().unwrap().to_string_lossy(),
                e
            ));
        }
    }
    assert!(checked > 0, "검사할 픽스처가 없음: {}", dir.display());
    assert!(
        fails.is_empty(),
        "{} 개 픽스처가 스키마 위반:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

#[test]
fn real_config_validates_against_schema() {
    if !xmllint_or_skip("real_config") {
        return;
    }
    let xsd = require_schema();
    let path = std::env::var("PRESGUEL_TEST_CONFIG")
        .unwrap_or_else(|_| "/home/nemo/git/lens/provision/config/layout.xml".to_string());
    let path = PathBuf::from(path);
    if !path.exists() {
        eprintln!("skip(real_config): 실제 설정 없음 ({})", path.display());
        return;
    }
    if let Err(e) = validate(&xsd, &path) {
        panic!("실제 설정이 스키마 위반:\n{e}");
    }
}
