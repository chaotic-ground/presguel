#!/usr/bin/env python3
"""presguel 설정창 (GTK4 + libadwaita).

GNOME 최신(49) 정석 스타일: AdwApplicationWindow + AdwHeaderBar +
AdwPreferencesGroup + AdwSwitchRow + AdwComboRow.

다루는 설정:
  - 간단 모드 on/off (기본 off = 모든 InputEntry 를 읽어 날개셋과 동일 동작)
  - 간단 모드 on 일 때: 한글 InputEntry / 영문 배치 InputEntry 를 드롭다운으로 지정

설정은 ~/.config/presguel/config.ini (key=value) 에 저장한다(엔진과 같은 형식).
드롭다운 항목은 ~/.config/presguel/nalgaeset.xml 의 InputEntry 들에서 읽는다.
"""

import os
import sys
import xml.etree.ElementTree as ET

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gio


def config_dir():
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    return os.path.join(base, "presguel")


def ini_path():
    return os.environ.get("PRESGUEL_CONFIG_INI") or os.path.join(
        config_dir(), "config.ini"
    )


def xml_path():
    return os.environ.get("PRESGUEL_CONFIG") or os.path.join(
        config_dir(), "nalgaeset.xml"
    )


def load_ini():
    """key=value 설정을 dict 로. 없으면 기본값."""
    cfg = {
        "pick_entry": "false",
        "entry": "0",
        "shortcuts_enabled": "true",
    }
    try:
        with open(ini_path(), encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#") or "=" not in line:
                    continue
                k, v = line.split("=", 1)
                cfg[k.strip()] = v.strip()
    except FileNotFoundError:
        pass
    return cfg


def save_ini(pick, entry_idx, shortcuts):
    os.makedirs(config_dir(), exist_ok=True)
    body = (
        "# presguel 설정 (presguel-setup 가 생성). key=value 형식.\n"
        "# pick_entry: 켜면 아래에서 고른 항목 하나만 사용(항목 전환 단축글쇠 없음).\n"
        "#            끄면 날개셋 설정의 모든 InputEntry 를 쓰고, 항목 전환은 ShortcutTable 의\n"
        "#            IME_SWITCH 단축글쇠가 등록됐을 때만 동작.\n"
        f"pick_entry = {'true' if pick else 'false'}\n"
        "# pick_entry 가 켜졌을 때 쓸 InputEntry 인덱스.\n"
        f"entry = {entry_idx}\n"
        "# 단축글쇠(한/영 키 등)로 입력 항목 전환. 끄면 그 키를 통과시켜 직접 바인딩 가능.\n"
        f"shortcuts_enabled = {'true' if shortcuts else 'false'}\n"
    )
    with open(ini_path(), "w", encoding="utf-8") as f:
        f.write(body)


def load_entries():
    """nalgaeset.xml 에서 (인덱스, 표시이름) 목록을 읽는다."""
    out = []
    try:
        root = ET.parse(xml_path()).getroot()
    except (FileNotFoundError, ET.ParseError):
        return out
    layer = root.find("InputLayer")
    if layer is None:
        return out
    for i, entry in enumerate(layer.findall("InputEntry")):
        name = None
        kt = entry.find(".//KeyTable")
        if kt is not None:
            name = kt.get("name")
        if not name:
            scheme = entry.find("InputSchemeSetting")
            obj = scheme.get("object") if scheme is not None else None
            if obj == "CInputScheme":
                name = "(직접 입력 / 영문 패스스루)"
            else:
                name = obj or "(이름 없음)"
        out.append((i, f"{i}: {name}"))
    return out


def _to_bool(s):
    return str(s).lower() in ("true", "1", "yes", "on")


def _to_int(s, default=0):
    try:
        return int(s)
    except (TypeError, ValueError):
        return default


class SetupWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Presguel 설정")
        self.set_default_size(460, -1)

        cfg = load_ini()
        self.entries = load_entries()
        labels = [lbl for _, lbl in self.entries] or ["(nalgaeset.xml 을 찾을 수 없음)"]
        # 초기값 세팅 중에는 notify 핸들러가 저장하지 않도록 막는다(불필요한 쓰기 방지).
        self._loading = True

        # 헤더바 + 본문을 담는 ToolbarView (Adw 표준 레이아웃). 즉시 적용이라 저장 버튼 없음.
        toolbar = Adw.ToolbarView()
        toolbar.add_top_bar(Adw.HeaderBar())

        page = Adw.PreferencesPage()
        toolbar.set_content(page)
        self.set_content(toolbar)

        group = Adw.PreferencesGroup(
            title="입력 항목",
            description="끄면(기본) 날개셋 설정의 모든 입력 항목을 그대로 쓰고, 항목 전환은 "
            "설정에 등록된 전환 단축글쇠로 합니다. 켜면 아래에서 고른 항목 하나만 사용합니다.",
        )
        page.add(group)

        # 입력 항목 직접 지정 스위치 행.
        self.pick_row = Adw.SwitchRow(
            title="입력 항목 직접 지정",
            subtitle="켜면 고른 항목 하나로 고정됩니다 (항목 전환 단축글쇠는 사용할 수 없게 됩니다)",
        )
        self.pick_row.set_active(_to_bool(cfg.get("pick_entry", "false")))
        self.pick_row.connect("notify::active", self.on_change)
        group.add(self.pick_row)

        # 사용할 항목 콤보.
        self.entry_row = Adw.ComboRow(
            title="사용할 입력 항목",
            subtitle="직접 지정을 켰을 때 쓸 항목",
            model=Gtk.StringList.new(labels),
        )
        self._set_combo(self.entry_row, _to_int(cfg.get("entry", "0")))
        self.entry_row.connect("notify::selected", self.on_change)
        group.add(self.entry_row)

        # 한/영 키로 입력 항목을 전환할지. 기본 켜짐.
        sc_group = Adw.PreferencesGroup(
            title="단축글쇠",
            description="한/영 키로 입력 항목을 전환합니다. 끄면 한/영 키를 직접 다른 용도로 "
            "쓸 수 있습니다.",
        )
        page.add(sc_group)

        self.shortcuts_row = Adw.SwitchRow(
            title="한/영 키로 전환",
        )
        self.shortcuts_row.set_active(_to_bool(cfg.get("shortcuts_enabled", "true")))
        self.shortcuts_row.connect("notify::active", self.on_change)
        sc_group.add(self.shortcuts_row)

        # 키보드 배열 안내(단축키·영문은 GNOME 입력 소스에서 배열별 엔진을 골라 정한다).
        kbd_group = Adw.PreferencesGroup(
            title="키보드 배열",
            description="단축키(Ctrl/Alt+키)와 영문은 GNOME 설정 → 키보드 → 입력 소스에서 "
            "'Presguel (Dvorak)' 처럼 원하는 배열을 고르면 됩니다. 한글 자판은 어느 배열에서도 "
            "같은 자리입니다.",
        )
        page.add(kbd_group)

        self._sync_sensitivity()
        self._loading = False

    def _set_combo(self, row, idx):
        n = max(1, len(self.entries))
        row.set_selected(idx if 0 <= idx < n else 0)
        if not self.entries:
            row.set_sensitive(False)

    def _sync_sensitivity(self):
        on = self.pick_row.get_active() and bool(self.entries)
        self.entry_row.set_sensitive(on)
        # 직접 지정을 켜면 항목 전환이 없으므로 단축글쇠 토글은 무의미해진다.
        self.shortcuts_row.set_sensitive(not self.pick_row.get_active())

    def on_change(self, *_):
        """위젯이 바뀔 때마다 즉시 config.ini 저장(GNOME instant-apply)."""
        self._sync_sensitivity()
        if self._loading:
            return
        pick = self.pick_row.get_active()
        e = self.entry_row.get_selected() if self.entries else 0
        shortcuts = self.shortcuts_row.get_active()
        save_ini(pick, e, shortcuts)


class SetupApp(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id="org.freedesktop.IBus.Presguel.Setup",
            flags=Gio.ApplicationFlags.FLAGS_NONE,
        )

    def do_activate(self):
        win = self.props.active_window
        if not win:
            win = SetupWindow(self)
        win.present()


def main():
    return SetupApp().run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
