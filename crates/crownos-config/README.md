# crownos-config

On-disk configuration for [CrownOS](https://github.com/Crown-OS) desktop apps —
and, because there is no IPC daemon, the mechanism by which CrownOS components
coordinate with each other.

**Status: Stable.** Builds, tested, API settling.

## The idea

Every settings *section* is one [RON](https://github.com/ron-rs/ron) file in the
CrownOS config directory:

```
~/.config/crownos/
├── appearance.ron
├── compositor.ron
├── display.ron
├── input.ron
└── …
```

That flat `<section>.ron` layout is a convention: the settings menu page name and
the file name are the same string, so a user who wants to hand-edit "Display"
knows to open `display.ron`.

Components `load()`, `save()` and `subscribe()`. Changes propagate live over
inotify — **there is no message bus**. Change `appearance.ron` and every process
that cares picks it up without a restart.

## Usage

```toml
[dependencies]
crownos-config = "0.2"

# Headless consumers should opt out of the xilem views:
# crownos-config = { version = "0.2", default-features = false }
```

> **0.2 is not published.** Nothing in the CrownOS organization is on crates.io
> except `crownshell` 0.1.0 and 0.2.0, so that dependency line resolves only if
> Cargo is pointed at a local checkout. `crownos-setup`'s `./bootstrap.sh --dev`
> clones the repos side by side and writes a `[patch.crates-io]` overlay into a
> `.cargo/config.toml` one directory **above** them; Cargo walks up from the
> working directory to find it, and the paths in it are relative to that file's
> own directory.

Every call is addressed by section name. Each schema struct carries its own
`SECTION` constant, so you pass `Appearance::SECTION` rather than a string
literal:

```rust
use crownos_config::{load, save, subscribe_key, subscribe_typed};
use crownos_config::schema::{appearance, Appearance};

let mut appearance: Appearance = load(Appearance::SECTION);
appearance.dark_mode = false;
save(Appearance::SECTION, &appearance).unwrap();

// Keep the Subscription alive — dropping it unregisters.
let _sub = subscribe_typed::<Appearance, _>(Appearance::SECTION, |new| {
    // Called on every external change to appearance.ron.
});

// Or watch a single field. `bar_height` is a u32 because the key says it is.
let _sub = subscribe_key(appearance::BarHeight, |bar_height| {
    println!("bar is now {bar_height}px");
});
```

| Function | Delivers |
|---|---|
| `load::<T>(section: &str) -> T` | Parsed section; writes defaults if the file is missing |
| `save::<T>(section: &str, &T) -> io::Result<()>` | Atomic write, and records a hash for echo suppression |
| `subscribe(section: &str, cb)` | Raw `Vec<u8>` contents on change |
| `subscribe_typed::<T, _>(section: &str, cb)` | Parsed `T` on change |
| `subscribe_key(key, cb)` | Only when one specific field changes; the key carries its own section |
| `config_dir()` | `$CROWN_CONFIG_DIR`, else `dirs::config_dir()/crownos` |

## Behaviour worth knowing

**A missing file materialises defaults.** A fresh install ends up with a
complete, readable config rather than nothing.

**Saves are atomic** — serialise, write `<file>.ron.tmp`, rename. A reader never
sees a half-written file.

**A parse failure does not clobber.** `load()` returns the default and leaves the
file alone; clobbering a config someone is halfway through editing would be
worse.

**Omitted fields fall back.** Every section derives `#[serde(default)]`, so a
hand-written file with three of ten fields parses fine.

**Implicit `Some`.** Optional fields are written `network: "home"`, not
`network: Some("home")`, so hand-editing stays natural.

**Echo suppression.** `save()` records a hash of what it wrote, and the watcher
drops events matching it. Without this, an app that saves on every slider tick
would immediately get its own write back as an external change and fight itself.

## Whole section or single key?

`subscribe_key` is right for a widget bound to one toggle. `subscribe_typed` is
right when several fields together describe a state you must apply atomically —
`crowndictator` subscribes to the whole `Input` section so the controller sees
one consistent snapshot instead of four independent edits.

## Sections

`appearance` · `bluetooth` · `compositor` · `display` · `input` · `keybinds` ·
`notifications` · `power` · `sound` · `wifi`

Full field reference:
[Configuration schema](https://github.com/Crown-OS/crownOs/blob/main/docs/configuration.md).

Sections are declared with a `section!` macro that generates the struct, a
`SECTION` constant, a `Default` impl, a **zero-sized unit key type per field**,
and a `<Name>Key` enum. The unit key types are what make `subscribe_key`
type-safe — you pass `DarkMode`, not `"dark_mode"`, so a typo is a compile error.

## Threading

Watcher callbacks are `Send + Sync` and run on the notify thread — they cannot
touch UI state directly. `crownpositor` posts to a calloop channel;
`crowndictator` turns config changes and hotkey edges into the same `Event` on
one channel.

For xilem apps, the default-on `xilem` feature provides `watch`, `watched` and
`watched_key` views that plumb changes through xilem's message path instead.

## Build and test

```bash
cargo build
cargo test
```

Rust **1.88+** (pinned in `rust-toolchain.toml`). No native dependencies beyond
what `notify` needs.

`tests/e2e.rs` is deliberately **a single `#[test]` function** that calls eight
sub-checks in sequence, because `CROWN_CONFIG_DIR` is process-global and cargo
runs test functions on parallel threads. Add a sub-check and call it from
`e2e()`; do not split it up.

## Environment

| Variable | Effect |
|---|---|
| `CROWN_CONFIG_DIR` | Overrides the config directory. Use it while developing so a work-in-progress build cannot damage your real settings. |

## Adding a section

1. Add `src/schema/<name>.rs` using the `section!` macro.
2. Declare it in `src/schema/mod.rs`.
3. Write a module doc that names **who the consumer is** — every existing section
   does, and it is the only place that contract is recorded.
4. Add a test that parses the RON shape you documented through
   `crate::parser::options()`, so it goes down the same path as `load()`.
   `schema/compositor.rs` is the model.

## Known limitations

- **Five sections have no reader at all**: `sound`, `wifi`, `bluetooth`,
  `power`, `keybinds`.
- **The `xilem` feature is on by default**, so a headless consumer pulls in a GUI
  toolkit unless it opts out.

## Contributing

See the organization-wide
[contribution guide](https://github.com/Crown-OS/crownOs/blob/main/CONTRIBUTING.md).
Default branch here is **`main`**.

## License

Licensed under the [MIT License](LICENSE).
