//! IBus 데몬 버스 주소 탐색.
//!
//! IBus 는 세션 버스가 아니라 자체 사설 버스를 쓴다. 탐색 순서(librush/ibusshare.c):
//! 1. `$IBUS_ADDRESS`
//! 2. `$IBUS_ADDRESS_FILE` 의 내용
//! 3. `$XDG_CONFIG_HOME/ibus/bus/<machine-id>-<host>-<display>` 파일
//!    - Wayland: host=`unix`, display=`$WAYLAND_DISPLAY` → `...-unix-wayland-0`
//!    - X11: `$DISPLAY` 파싱(`host:display.screen`), host 비면 `unix` → `...-unix-0`
//!
//! 파일에서 `IBUS_ADDRESS=` 줄의 값을 읽는다(`#` 주석 무시).
//!
//! 참고: `research/03-ibus-zbus.md` §5, `research/00-environment.md`.

use std::path::PathBuf;

/// IBus 버스 주소 문자열(`unix:path=...,guid=...`)을 찾는다.
pub fn find_ibus_address() -> Result<String, String> {
    if let Ok(addr) = std::env::var("IBUS_ADDRESS") {
        if !addr.is_empty() {
            return Ok(addr);
        }
    }
    let file = if let Ok(f) = std::env::var("IBUS_ADDRESS_FILE") {
        PathBuf::from(f)
    } else {
        address_file_path()?
    };
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("주소 파일 {} 읽기 실패: {e}", file.display()))?;
    parse_address_file(&content)
        .ok_or_else(|| format!("{} 에서 IBUS_ADDRESS 를 찾지 못함", file.display()))
}

/// 주소 파일 본문에서 `IBUS_ADDRESS=` 값을 뽑는다.
pub fn parse_address_file(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(v) = line.strip_prefix("IBUS_ADDRESS=") {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// 주소 파일 경로 `$XDG_CONFIG_HOME/ibus/bus/<machine-id>-<host>-<display>` 를 만든다.
pub fn address_file_path() -> Result<PathBuf, String> {
    let config_home = std::env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()).map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
        .ok_or_else(|| "XDG_CONFIG_HOME/HOME 둘 다 없음".to_string())?;

    let machine_id = read_machine_id()?;
    let (host, display) = host_and_display();
    let name = format!("{machine_id}-{host}-{display}");
    Ok(config_home.join("ibus").join("bus").join(name))
}

fn read_machine_id() -> Result<String, String> {
    for p in ["/var/lib/dbus/machine-id", "/etc/machine-id"] {
        if let Ok(s) = std::fs::read_to_string(p) {
            let s = s.trim().to_string();
            if !s.is_empty() {
                return Ok(s);
            }
        }
    }
    Err("machine-id 를 찾지 못함".to_string())
}

/// (host, display) 를 결정한다. Wayland 우선, 아니면 X11 `$DISPLAY`.
pub fn host_and_display() -> (String, String) {
    if let Ok(wl) = std::env::var("WAYLAND_DISPLAY") {
        if !wl.is_empty() {
            return ("unix".to_string(), wl);
        }
    }
    // X11: DISPLAY = "host:display.screen" (host 비면 unix)
    let disp = std::env::var("DISPLAY").unwrap_or_default();
    parse_x_display(&disp)
}

/// `host:display.screen` 을 (host_or_unix, display_number) 로 파싱.
pub fn parse_x_display(disp: &str) -> (String, String) {
    // 예: ":0" → ("unix","0");  "localhost:10.0" → ("localhost","10")
    let (host, rest) = match disp.split_once(':') {
        Some((h, r)) => (h, r),
        None => ("", disp),
    };
    let display_num = rest.split('.').next().unwrap_or("0");
    let host = if host.is_empty() { "unix" } else { host };
    (host.to_string(), display_num.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file() {
        let body = "# comment\n# another\nIBUS_ADDRESS=unix:path=/run/user/1000/ibus/dbus-AbC,guid=deadbeef\nIBUS_DAEMON_PID=123\n";
        assert_eq!(
            parse_address_file(body).as_deref(),
            Some("unix:path=/run/user/1000/ibus/dbus-AbC,guid=deadbeef")
        );
        assert_eq!(parse_address_file("# nothing here\n"), None);
    }

    #[test]
    fn x_display_parsing() {
        assert_eq!(parse_x_display(":0"), ("unix".to_string(), "0".to_string()));
        assert_eq!(parse_x_display(":10.0"), ("unix".to_string(), "10".to_string()));
        assert_eq!(
            parse_x_display("localhost:10.0"),
            ("localhost".to_string(), "10".to_string())
        );
    }
}
