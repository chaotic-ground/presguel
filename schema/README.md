# schema/

`nalgaeset-no-namespace.xsd` 는 presguel 이 읽는 자판 설정(`layout.xml`, 날개셋
종합 설정 `.set` 형식)을 기계 검증하기 위한 XML Schema 다. 런타임이 아니라
개발/CI 린트에서만 쓴다(`crates/presguel-core/tests/schema.rs`).

- 출처: [chaotic-ground/nalgaeset-reverse-spec](https://github.com/chaotic-ground/nalgaeset-reverse-spec)
- 라이선스: CC BY 4.0 (© 2026 lens0021). 본 파일은 그 저장소의 정본
  `nalgaeset.xsd` 에서 네임스페이스 헤더만 제거한 동기화본으로, 네임스페이스가
  없는 실제 날개셋 출력 파일을 그대로 검증한다.
- 갱신: 상위 사양이 바뀌면 위 저장소에서 다시 받아 이 파일을 교체한다.

검증 린트(`crates/presguel-core/tests/schema.rs`)는 `xmllint`(libxml2)가 있을 때만
돈다. 없으면 조용히 skip 하되 stderr 로 알리며, CI 에서 강제하려면
`PRESGUEL_REQUIRE_XMLLINT=1` 로 돌려 xmllint 부재 시 실패하게 한다.
