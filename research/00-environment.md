# Local environment facts (verified on this machine)

Captured directly from the target machine so the ibus frontend matches reality.

## Toolchain
- `rustc` 1.93.1, `cargo` 1.93.1
- IBus **1.5.33** running
- No `ibus-1.0` pkg-config / dev headers installed -> confirms pure-Rust (zbus) path; do NOT depend on libibus.

## Session
- Wayland session (`WAYLAND_DISPLAY`-based bus file suffix `unix-wayland-0`).
- `XMODIFIERS=@im=ibus`, `QT_IM_MODULE=ibus`, `QT_IM_MODULES=wayland;ibus`.
- machine-id: `e8602052bcae45b4bc57b5bfb255c5e3`

## IBus bus address discovery (how our engine connects)
Order:
1. `$IBUS_ADDRESS` env var if set.
2. Else read file `~/.config/ibus/bus/<machine-id>-unix-<display-tag>`
   - On this machine: `~/.config/ibus/bus/e8602052bcae45b4bc57b5bfb255c5e3-unix-wayland-0`
   - display-tag is `wayland-N` on Wayland, `:N` (or similar) on X11.
3. File contents (key=value lines, ignore `#` comments):
   ```
   IBUS_ADDRESS=unix:path=/home/nemo/.cache/ibus/dbus-YO2EVmpR,guid=e66a6058627e95ce69b068696a169262
   IBUS_DAEMON_PID=3092535
   ```
4. zbus connects to this with a custom address (NOT the standard session bus):
   `zbus::connection::Builder::address(addr)?.build().await`
   The address string `unix:path=...,guid=...` is a standard D-Bus address zbus understands.

## Component install location
- System: `/usr/share/ibus/component/` (needs sudo; contains anthy/chewing/dconf/gtk*/hangul/libpinyin/m17n/simple/typing-booster .xml)
- **CORRECTION (verified on this machine):** this ibus does NOT scan the per-user dir.
  `ibus write-cache` only scans `/usr/share/ibus/component`. A known-good component placed in
  `~/.local/share/ibus/component/` (XDG_DATA_HOME) or `~/.config/ibus/component/` was NOT picked
  up. So installation here REQUIRES the system dir + `sudo ibus write-cache --system`.
  (`scripts/install.sh` does this.) Activation confirmed: `ibus list-engine` shows
  `presguel - Presguel (날개셋 세벌식)`, and `ibus engine presguel` makes the daemon exec
  `/usr/local/bin/presguel-ibus --ibus`.

## Reference component template (ibus-hangul)
`/usr/share/ibus/component/hangul.xml`:
```xml
<?xml version="1.0" encoding="utf-8"?>
<component>
	<name>org.freedesktop.IBus.Hangul</name>
	<description>Korean Component</description>
	<exec>/usr/libexec/ibus-engine-hangul --ibus</exec>
	<version>1.5.5</version>
	<author>...</author>
	<license>GPL</license>
	<homepage>...</homepage>
	<textdomain>ibus-hangul</textdomain>
	<engines>
		<engine>
			<name>hangul</name>
			<language>ko</language>
			<license>GPL</license>
			<author>...</author>
			<icon>ibus-hangul</icon>
			<layout>kr</layout>
			<layout_variant>kr104</layout_variant>
			<longname>Hangul</longname>
			<description>Korean Input Method</description>
			<rank>99</rank>
			<symbol>&#xD55C;</symbol>
			<setup>/usr/libexec/ibus-setup-hangul</setup>
		</engine>
	</engines>
</component>
```
Notes for ours:
- `<exec>` should be `.../nalgaeset-ibus --ibus` (the `--ibus` flag tells the engine it was launched by ibus-daemon).
- `<name>` bus-name style, e.g. `org.freedesktop.IBus.Nalgaeset`.
- engine `<name>` is the short id used in `CreateEngine`, e.g. `nalgaeset`.
- `<layout>us` so the daemon does not pre-translate to a kr XKB layout (our KeyTable is indexed by US-ASCII positions). Verify: 세벌식 needs raw US positions, so `us` layout is correct, NOT `kr`.
