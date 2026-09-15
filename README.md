# CrownOS

A Wayland desktop written from scratch in Rust — compositor, shell framework,
bar, dock, notifications, settings schema and a push-to-talk dictation daemon.

![CrownOS running nested: crownbar along the top, a terminal tiled beneath it](assets/screenshot.png)

*crownpositor with crownbar, running nested inside an existing session. The bar
reserves its exclusive zone; the terminal tiles below it.*

> **CrownOS is early.** It builds, it runs, and it is not something to put on
> your only laptop yet. The
> [project status page](https://github.com/Crown-OS/crownos-documentations/blob/main/docs/00-overview/project-status.md)
> says what works today, component by component.

## Build it

Any Linux distribution. The one command that matters:

```bash
git clone https://github.com/Crown-OS/crownOs && cd crownOs
cargo build --workspace
```

There is no overlay to write, no sibling layout to reproduce and no dependency
to publish first — the crates resolve each other by path. If `cargo build` fails,
it is a missing system library, and
[crownos-setup](https://github.com/Crown-OS/crownOs-setup) installs those on any
distro:

```bash
./bootstrap.sh --check     # what does this machine already have?
./bootstrap.sh --dev       # install it, and clone the rest of the org
```

It falls through three layers — your package manager, then Nix, then a
container — so "it only builds on Arch" is not a thing that can happen. The Nix
path is the one to reach for if your distro is unusual:

```bash
nix develop github:Crown-OS/crownOs-setup --command cargo build --workspace
```

## Try it

Three ways, costing progressively more and testing progressively more. Start at
the top.

### Nested — the daily loop

Runs inside your current session, in a window. Cannot affect your machine.

```bash
./session/install.sh
CROWN_BACKEND=winit crownos-session
```

That is the whole desktop: compositor, bar, dock and notifications. It exercises
layout, rendering, input and IPC — everything except the parts that only exist on
real hardware.

### A VM — before you trust it with a login

```bash
./contrib/run-vm.sh
```

Builds the workspace, boots a NixOS guest on virtio-gpu, and autologins into
CrownOS. This is the tier that tests what nesting cannot: **seat acquisition,
DRM/KMS mode setting, and the session launcher a display manager would use** —
the three things that decide whether CrownOS works on real hardware.

Your `target/` is mounted read-only in the guest, so a rebuild on the host is
picked up by the next boot; nothing is installed into the image. Needs `nix`,
and `/dev/kvm` if you want it to be fast rather than merely correct.

### Real hardware

```bash
sudo ./session/install.sh --system
```

Then pick CrownOS at your display manager. Have a second TTY available the first
time, and read
[the compositor's notes](crates/crownpositor/README.md) on seats first.

## The crates

| Crate | What it is |
|---|---|
| `crownpositor` | Tiling Wayland compositor, built on Smithay. It *is* the session — it owns the socket and spawns everything else. |
| `crownshell` | The layer-shell + Vello framework every surface is built on |
| `crownos-config` | Shared settings schema, live-reloading, RON on disk |
| `crownbar` `crowndock` `crownotify` | Bar, dock, notification daemon |
| `crowndictator` | Push-to-talk voice dictation |
| `crownuikit` | Widget kit on xilem, for the settings panel |
| `crownpositor-config` | The compositor's compiled configuration types |

They are one Cargo workspace but publish to crates.io as independent crates with
independent versions — `cargo add crownshell` works exactly as before.

## Configuration

`~/.config/crownos/*.ron`, watched live. A rebind takes effect without a restart.
`session/compositor.example.ron` is a working starting point;
[the schema reference](https://github.com/Crown-OS/crownos-documentations/blob/main/docs/50-reference/config-schema.md)
documents every field.

One thing worth knowing up front: a keybind that does not parse silently reverts
its whole section to defaults, with no error. Key names are labels — `A`, `Left`,
`Space`, `F5` — not the W3C `code` spellings (`KeyA`, `ArrowLeft`).

## Contributing

```bash
cargo build --workspace --all-targets
cargo test --workspace --exclude crownotify
dbus-run-session -- cargo test -p crownotify -- --test-threads=1
cargo fmt --all
```

rustfmt blocks in CI; clippy is advisory while a lint backlog is cleared. The
toolchain is pinned to 1.88.0 in `rust-toolchain.toml` — rustup honours that over
whatever you have installed, so everyone compiles with the same rustc.

The [contribution guide](https://github.com/Crown-OS/crownos-documentations/blob/main/CONTRIBUTING.md)
has the rest.

## Elsewhere in the organisation

[crownos-documentations](https://github.com/Crown-OS/crownos-documentations) ·
[crownOs-setup](https://github.com/Crown-OS/crownOs-setup) ·
[crownos-iso](https://github.com/Crown-OS/crownos-iso) ·
[crownos-website](https://github.com/Crown-OS/crownos-website) ·
[crowncrate-linux](https://github.com/Crown-OS/crowncrate-linux) ·
[crowncrate-android](https://github.com/Crown-OS/crowncrate-android) ·
[lls-protocol](https://github.com/Crown-OS/lls-protocol)

## Licence

MIT — see [LICENSE](LICENSE). `crownuikit` bundles the Inter typeface, which is
SIL OFL 1.1; its licence travels with it at
`crates/crownuikit/resources/fonts/LICENSE-Inter.txt`.
