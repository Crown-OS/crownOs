# crownbar

The status bar for [CrownOS](https://github.com/Crown-OS). A top-anchored
[layer-shell](https://github.com/Crown-OS/crownshell) surface showing clock,
battery, wifi, bluetooth, brightness and volume.

**Status: Partial.** It builds and runs, but reads no CrownOS configuration and
its layout widget is a no-op. See [Known limitations](#known-limitations).

## Surface

| | |
|---|---|
| Layer | `Top` |
| Anchor | TOP, LEFT, RIGHT |
| Size | full width, 40 px |
| Exclusive zone | 40 |
| Namespace | `crownbar` |

## Prerequisites

Any Wayland compositor supporting `wlr-layer-shell` — `crownpositor`, Hyprland,
Sway, river, KWin — so you can develop this inside your existing desktop.

Native dependencies (Arch):

```bash
sudo pacman -S --needed base-devel pkgconf binutils \
  wayland wayland-protocols libxkbcommon \
  vulkan-icd-loader mesa libglvnd fontconfig dbus
```

Full list, including Debian/Ubuntu:
[Prerequisites](https://github.com/Crown-OS/crownos-documentations/blob/main/docs/10-getting-started/prerequisites.md).

> **This crate forces the BFD linker** via a committed `.cargo/config.toml`
> (`-C link-arg=-fuse-ld=bfd`). You need `ld.bfd` from `binutils`. If you use
> `mold` or `lld` globally, it is overridden here.

## Build and run

> **You need the dev overlay first.** `crownbar` depends on `crownshell = "0.3"`,
> and 0.3 is **not published** — crates.io has only `crownshell` 0.1.0 and 0.2.0.
> A fresh clone fails at `cargo metadata` until Cargo is pointed at a local
> `crownshell` checkout.
>
> `crownos-setup`'s `./bootstrap.sh --dev` clones the repos side by side and
> writes a `[patch.crates-io]` overlay into a `.cargo/config.toml` one directory
> **above** them:
>
> ```
> ~/crownos/
> ├── .cargo/config.toml   # [patch.crates-io] crownshell = { path = "crownshell" }
> ├── crownbar/
> └── crownshell/
> ```
>
> Cargo walks up from the working directory to find that file, and the paths in
> it are relative to the file's own directory. No particular layout *inside* a
> repo is required.

```bash
cargo run
RUST_LOG=debug cargo run
```

A widget whose hardware is absent returns `None` from `try_new()` and is silently
skipped — on a desktop with no battery you simply get no battery indicator.

## How it reads the system

Deliberately, `crownbar` talks to the kernel and to command-line tools rather
than to daemons, so the bar stays self-sufficient and works pre-login.

| Widget | Source |
|---|---|
| Wi-Fi | `/sys/class/net/*/wireless`, `/proc/net/wireless` — no NetworkManager |
| Bluetooth | `/sys/class/rfkill/*` — no BlueZ, no D-Bus |
| Brightness | `/sys/class/backlight/` |
| Volume | `wpctl`, falling back to `pactl` |
| Battery | the `battery` crate |

## Adding a widget

Implement `BarWidget` and register it in the `WidgetRegistry` in `lib.rs`.
Return `None` from `try_new()` when the hardware is not present. Use
`util::poll::PollGate` if you need something slower than the 1 Hz tick.

Icons are drawn procedurally with Vello in `ui/icons/` rather than loaded from
files.

## Known limitations

- **It reads no CrownOS configuration.** There is no `crownos-config`
  dependency. Bar height is hardcoded at 40 while `appearance.bar_height`
  defaults to 32, and accent colour, transparency and animation profile are all
  ignored. Wiring this up is a well-scoped first contribution — copy
  `crowndictator/src/settings.rs`, which is 40 lines.
- **The layout widget does nothing.** It owns local UI state and animates an
  icon; the compositor switch is not implemented.
- **Blur is requested but does not happen** under `crownpositor`, which never
  advertises `ext-background-effect-v1`.
- **Widgets are hardcoded** in `lib.rs`; nothing is configurable at runtime.
- **No tests.**
- **It does not build from a fresh clone on its own.** `crownshell = "0.3"` is an
  unpublished version; you need the `[patch.crates-io]` overlay described under
  [Build and run](#build-and-run).
- `bluer` and `tracing` are declared dependencies and unused.
- It carries its own spring implementation rather than using `crownshell`'s.

## Tests

There are none. `cargo test` compiles the crate and reports zero tests. Adding
coverage for the `/sys` and `/proc` parsers — they take file contents, so they
are testable without hardware — is a well-scoped contribution.

## Contributing

See the organization-wide
[contribution guide](https://github.com/Crown-OS/crownos-documentations/blob/main/CONTRIBUTING.md).
Default branch here is **`main`**.

## License

Licensed under the [MIT License](LICENSE).
