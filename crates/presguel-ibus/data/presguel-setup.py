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
    return os.environ.get("PRESGUEL_CONFIG_INI") or os.path.join(config_dir(), "config.ini")


def xml_path():
    return os.environ.get("PRESGUEL_CONFIG") or os.path.join(config_dir(), "nalgaeset.xml")


def load_ini():
    """key=value 설정을 dict 로. 없으면 기본값."""
    cfg = {
        "simple_mode": "false",
        "hangul_entry": "0",
        "latin_entry": "1",
        "base_layout": "us",
        "base_variant": "",
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


def save_ini(simple, hangul_idx, latin_idx, base_layout, base_variant):
    os.makedirs(config_dir(), exist_ok=True)
    body = (
        "# presguel 설정 (presguel-setup 가 생성). key=value 형식.\n"
        "# simple_mode: 켜면 아래 두 항목만 써서 단순 동작. 끄면 모든 InputEntry(날개셋 동일).\n"
        f"simple_mode = {'true' if simple else 'false'}\n"
        "# 간단 모드에서 쓸 한글 InputEntry 인덱스.\n"
        f"hangul_entry = {hangul_idx}\n"
        "# 간단 모드에서 한/영 전환 시 쓸 영문 InputEntry 인덱스.\n"
        f"latin_entry = {latin_idx}\n"
        "# 단축키·keysym 변환 기준 XKB 레이아웃(컴포넌트 XML 에 반영, 적용은 관리자 권한 필요).\n"
        f"base_layout = {base_layout}\n"
        f"base_variant = {base_variant}\n"
    )
    with open(ini_path(), "w", encoding="utf-8") as f:
        f.write(body)


# 베이스 키보드 레이아웃 선택지: (표시 이름, xkb layout, xkb variant)
BASE_LAYOUTS = [
    ("미국 QWERTY", "us", ""),
    ("Dvorak", "us", "dvorak"),
    ("Dvorak (국제, 죽은키)", "us", "dvorak-intl"),
    ("Colemak", "us", "colemak"),
    ("Colemak-DH", "us", "colemak_dh"),
    ("Workman", "us", "workman"),
]


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
            title="입력 동작",
            description="끄면 설정의 모든 입력 항목을 읽어 날개셋과 똑같이 동작합니다. "
            "켜면 아래에서 고른 한글 항목과 영문 항목만 한/영 전환에 사용합니다.",
        )
        page.add(group)

        # 간단 모드 스위치 행.
        self.simple_row = Adw.SwitchRow(
            title="간단 모드",
            subtitle="한글 / 영문 배치 항목을 직접 지정",
        )
        self.simple_row.set_active(_to_bool(cfg.get("simple_mode", "false")))
        self.simple_row.connect("notify::active", self.on_change)
        group.add(self.simple_row)

        # 한글 항목 콤보.
        self.hangul_row = Adw.ComboRow(
            title="한글 입력 항목",
            subtitle="실제로 쓸 한글 자판",
            model=Gtk.StringList.new(labels),
        )
        self._set_combo(self.hangul_row, _to_int(cfg.get("hangul_entry", "0")))
        self.hangul_row.connect("notify::selected", self.on_change)
        group.add(self.hangul_row)

        # 영문 항목 콤보(한/영 전환 시 영문으로 쓸 항목).
        self.latin_row = Adw.ComboRow(
            title="영문 입력 항목",
            subtitle="한/영 전환 시 쓸 영문 항목",
            model=Gtk.StringList.new(labels),
        )
        self._set_combo(self.latin_row, _to_int(cfg.get("latin_entry", "1")))
        self.latin_row.connect("notify::selected", self.on_change)
        group.add(self.latin_row)

        # ── 베이스 키보드 레이아웃(단축키·keysym 변환 기준) ──────────────────────
        kbd_group = Adw.PreferencesGroup(
            title="단축키 키보드 배열",
            description="한글 글자 자판은 물리 위치로 고정되지만, 단축키(Ctrl/Alt+키)와 "
            "영문 입력은 이 XKB 레이아웃을 따릅니다. 드보락 등을 쓰면 여기서 고르세요.",
        )
        page.add(kbd_group)

        # 현재 config 의 base_layout/base_variant 에 맞는 콤보 항목을 찾는다.
        self._base_labels = [lbl for lbl, _, _ in BASE_LAYOUTS]
        cur_layout = cfg.get("base_layout", "us") or "us"
        cur_variant = cfg.get("base_variant", "")
        self._base_index = 0
        for i, (_, lay, var) in enumerate(BASE_LAYOUTS):
            if lay == cur_layout and var == cur_variant:
                self._base_index = i
                break
        else:
            # 목록에 없는 사용자 지정값이면 항목을 추가해 잃지 않는다.
            custom = f"사용자 지정: {cur_layout}" + (f"+{cur_variant}" if cur_variant else "")
            BASE_LAYOUTS.append((custom, cur_layout, cur_variant))
            self._base_labels.append(custom)
            self._base_index = len(BASE_LAYOUTS) - 1

        self.base_row = Adw.ComboRow(
            title="베이스 레이아웃",
            subtitle="단축키가 따를 키보드 배열",
            model=Gtk.StringList.new(self._base_labels),
        )
        self.base_row.set_selected(self._base_index)
        self.base_row.connect("notify::selected", self.on_change)
        kbd_group.add(self.base_row)

        # 시스템 적용 버튼(컴포넌트 XML 은 root 소유 → pkexec 로 권한 상승).
        apply_row = Adw.ActionRow(
            title="시스템에 적용",
            subtitle="관리자 권한이 필요하며, 적용 후 다시 로그인해야 반영됩니다.",
        )
        self.apply_btn = Gtk.Button(label="적용…")
        self.apply_btn.set_valign(Gtk.Align.CENTER)
        self.apply_btn.connect("clicked", self.on_apply_layout)
        apply_row.add_suffix(self.apply_btn)
        apply_row.set_activatable_widget(self.apply_btn)
        kbd_group.add(apply_row)

        self._apply_status = Gtk.Label(xalign=0, wrap=True)
        self._apply_status.add_css_class("dim-label")
        kbd_group.add(self._apply_status)

        # 안내 행.
        note = Adw.PreferencesGroup()
        lbl = Gtk.Label(
            label="입력 동작 설정은 즉시 적용됩니다(입력창을 다시 누르면 반영). "
            "베이스 레이아웃은 '시스템에 적용' 후 재로그인이 필요합니다.",
            xalign=0,
            wrap=True,
        )
        lbl.add_css_class("dim-label")
        note.add(lbl)
        page.add(note)

        self._sync_sensitivity()
        self._loading = False

    def _set_combo(self, row, idx):
        n = max(1, len(self.entries))
        row.set_selected(idx if 0 <= idx < n else 0)
        if not self.entries:
            row.set_sensitive(False)

    def _sync_sensitivity(self):
        on = self.simple_row.get_active() and bool(self.entries)
        self.hangul_row.set_sensitive(on)
        self.latin_row.set_sensitive(on)

    def _base_layout_variant(self):
        """현재 콤보 선택의 (layout, variant)."""
        idx = self.base_row.get_selected()
        if 0 <= idx < len(BASE_LAYOUTS):
            _, lay, var = BASE_LAYOUTS[idx]
            return lay, var
        return "us", ""

    def on_change(self, *_):
        """위젯이 바뀔 때마다 즉시 config.ini 저장(GNOME instant-apply)."""
        self._sync_sensitivity()
        if self._loading:
            return
        simple = self.simple_row.get_active()
        h = self.hangul_row.get_selected() if self.entries else 0
        l = self.latin_row.get_selected() if self.entries else 1
        lay, var = self._base_layout_variant()
        save_ini(simple, h, l, lay, var)

    def on_apply_layout(self, *_):
        """베이스 레이아웃을 시스템 컴포넌트 XML 에 적용(pkexec → 권한 헬퍼)."""
        lay, var = self._base_layout_variant()
        argv = ["pkexec", "/usr/local/bin/presguel-apply-layout", lay]
        if var:
            argv.append(var)
        try:
            proc = Gio.Subprocess.new(
                argv, Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_MERGE
            )
        except Exception as e:  # noqa: BLE001
            self._apply_status.set_text(f"적용 실행 실패: {e}")
            return
        self.apply_btn.set_sensitive(False)
        self._apply_status.set_text("적용 중…")

        def done(p, res):
            self.apply_btn.set_sensitive(True)
            try:
                ok, _out, _err = p.communicate_utf8_finish(res)
            except Exception as e:  # noqa: BLE001
                self._apply_status.set_text(f"적용 실패: {e}")
                return
            if p.get_successful():
                tag = lay + (f"+{var}" if var else "")
                self._apply_status.set_text(
                    f"적용됨: {tag}. 다시 로그인하면 단축키가 이 배열을 따릅니다."
                )
            else:
                self._apply_status.set_text("적용이 취소되었거나 실패했습니다.")

        proc.communicate_utf8_async(None, None, done)


class SetupApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="org.freedesktop.IBus.Presguel.Setup",
                         flags=Gio.ApplicationFlags.FLAGS_NONE)

    def do_activate(self):
        win = self.props.active_window
        if not win:
            win = SetupWindow(self)
        win.present()


def main():
    return SetupApp().run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
