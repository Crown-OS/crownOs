# crownuikit

A widget kit for [CrownOS](https://github.com/Crown-OS), built on
[xilem](https://github.com/linebender/xilem) and Masonry — sidebar, sliders,
toggles, selects, context menus and icons.

**Status: Early.** It builds and runs a widget gallery, but is not yet wired to
anything.

## Why it exists alongside crownshell

Two different jobs:

| | [`crownshell`](https://github.com/Crown-OS/crownshell) | `crownuikit` |
|---|---|---|
| For | Desktop surfaces — bars, docks, toasts | Ordinary application windows |
| Stack | Raw `wlr-layer-shell` + direct Vello painting | xilem / Masonry, reactive |
| You write | A paint callback | A view tree |

`crownuikit` is intended for the CrownOS settings panel. `crownos-config` ships a
`xilem` feature providing `watch`, `watched` and `watched_key` views that exist
to feed exactly this kind of app — though nothing connects the two yet.

## Prerequisites

`crownuikit` has no CrownOS dependencies and needs no dev overlay — but `winit`
and `xilem` do need a Wayland session, a Vulkan loader and fontconfig at build
and run time.

Native dependencies (Arch):

```bash
sudo pacman -S --needed base-devel pkgconf \
  wayland wayland-protocols libxkbcommon \
  vulkan-icd-loader mesa libglvnd fontconfig
```

Full list, including Debian/Ubuntu:
[Prerequisites](https://github.com/Crown-OS/crownos-documentations/blob/main/docs/10-getting-started/prerequisites.md),
or run [`crownos-setup`](https://github.com/Crown-OS/crownos-setup)'s
`./bootstrap.sh --check` to have them installed for you.

Rust **1.88+** (pinned in `rust-toolchain.toml`).

## Build and run

```bash
cargo build
cargo run
```

Opens a desktop window with the widget gallery: sidebar, sliders, toggles and a
`select`.

Three dependencies only — `xilem`, `winit`, `blinc_icons` (Lucide icon bodies).
No CrownOS dependencies.

## API

```rust
pub mod animation;
pub mod config;    // global Theme behind an RwLock: theme() / set_theme()
pub mod layouts;   // sidebar and its parts
pub mod util;      // lerp_color, Inter font, shadows, inflated_pill
pub mod widgets;   // context_menu, icon, select, slider, toggle
```

**Theming** is a process-global `Theme` behind an `RwLock`, read with `theme()`
and replaced with `set_theme()`. The default palette is a shadcn-derived dark
scheme with `GradientStops` and `PopoverColors`.

**The sidebar** is composed from `sidebar`, `sidebar_brand`, `sidebar_group`,
`sidebar_item`, `sidebar_subitem` and `sidebar_separator`, plus an
animated-height `collapse` container. `layouts/sidebar/mod.rs` opens with a
design note explaining the composition — read it before adding a part.

**Inter** is embedded with `include_bytes!` from `resources/fonts/`.

## Known limitations

- **The demo content is placeholder.** The gallery's sidebar shows fintech
  material from a design mock — "Untitled UI", "Bank accounts", "Upgrade to PRO".
  It is not CrownOS settings navigation.
- **It is wired to nothing, in both directions.** No `crownos-config`
  dependency, so it reads and writes no settings — and no repository in the
  organization depends on `crownuikit`, so nothing consumes it either.
  Connecting it to the settings panel is the obvious next step.
- **Dead and empty files**: `widgets/status.rs` and `layouts/header/mod.rs` are
  zero bytes and undeclared; `widgets/search.rs` is zero bytes but *is* declared,
  so it is an empty live module.
- **No tests**, no examples.
- It carries its own spring implementation — the fifth in the project.

## Tests

There are none. `cargo test` compiles the crate and reports zero tests. There are
no examples either — the gallery in `main.rs` is the only exercise of the widgets.

## Contributing

See the organization-wide
[contribution guide](https://github.com/Crown-OS/crownos-documentations/blob/main/CONTRIBUTING.md).
Default branch here is **`main`**.

## License

Licensed under the [MIT License](LICENSE).
