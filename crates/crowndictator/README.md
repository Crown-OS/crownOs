# crowndictator

Push-to-talk voice dictation for Wayland.

Hold the shortcut, speak, release: the recording is transcribed **locally** with
NVIDIA Parakeet TDT 0.6B v2 (ONNX Runtime, CUDA when available) and typed into
the focused window. A small waveform pill sits at the bottom of the screen while
it listens and thinks.

Nothing is sent to a server.

**Status: Early.** v0, but internally coherent and usable. The prerequisites are
substantial — read them before starting.

## How it works

```
hold hotkey ──► evdev reads /dev/input/event*   (bypasses the compositor)
                       │
                       ▼
               cpal captures 16 kHz mono f32
                       │
                       ▼
     ONNX Runtime: mel → conformer encoder → greedy TDT decode → detokenise
                       │
                       ▼
     wtype ──fallback──► ydotool ──fallback──► wl-copy + notify-send
```

The ASR engine loads on demand and **drops the model after 300 s idle** to free
VRAM, so the first transcription after a pause is slower.

Both the hotkey and the text injection deliberately bypass the compositor, so it
works under any Wayland compositor rather than only `crownpositor`.

## Prerequisites

**System packages** (Arch):

```bash
sudo pacman -S --needed base-devel pkgconf alsa-lib libevdev \
  wayland wayland-protocols libxkbcommon \
  vulkan-icd-loader mesa libglvnd fontconfig dbus \
  wtype wl-clipboard libnotify
```

Debian/Ubuntu equivalents:
[Prerequisites](https://github.com/Crown-OS/crownos-documentations/blob/main/docs/10-getting-started/prerequisites.md#crowndictator).

**`/dev/input` read access.** The global hotkey is detected by reading
`/dev/input/event*` directly, so your user must be in the `input` group:

```bash
sudo usermod -aG input "$USER"    # log out and back in
```

**ONNX Runtime with CUDA.** `ort` is pinned exactly (`= "2.0.0-rc.12"`) and the
`cuda` feature is not optional, so it is a hard build dependency even without
NVIDIA hardware. The runtime falls back to CPU.

**A large first-run download.** Model weights come from Hugging Face:
roughly **700 MB** for the int8 CPU model, **2.5 GB** for fp32 on GPU.

**The dev overlay.** `crownshell = "0.3"` and `crownos-config = "0.2"` are
ordinary crates.io dependencies, but **neither version is published** — crates.io
has only `crownshell` 0.1.0 and 0.2.0, and no `crownos-config` at all. A fresh
clone fails at `cargo metadata` until Cargo is pointed at local checkouts.
`crownos-setup`'s `./bootstrap.sh --dev` clones the repos side by side and writes
a `[patch.crates-io]` overlay into a `.cargo/config.toml` one directory **above**
them:

```
~/crownos/
├── .cargo/config.toml   # [patch.crates-io] crownshell, crownos-config
├── crowndictator/
├── crownshell/
└── crownos-config/
```

Cargo walks up from the working directory to find that file, and the paths in it
are relative to the file's own directory. No particular layout *inside* a repo is
required.

**A `wlr-layer-shell` compositor** for the overlay.

## Build and run

```bash
cargo run -- --demo               # cycle the overlay states with fake audio.
                                  # No model download, no microphone, no input
                                  # group needed. Use this for UI work.
cargo run                         # the daemon
cargo run -- --transcribe f.wav   # one-shot, 16 kHz wav
cargo run -- --cpu                # skip CUDA
```

`--demo` is the contributor-friendly path — it exercises every visual state
without touching a model or `/dev/input`.

## Configuration

`~/.config/crownos/input.ron`, followed **live**: turning dictation off releases
the keyboard grab, and changing the shortcut re-arms it without a restart.

```ron
(
    dictation_enabled: true,
    dictation_microphone: "Blue Yeti Analog Stereo",
    dictation_hotkey: "Super+Alt+D",
    dictation_gpu: true,
)
```

| Field | Default | Effect |
|---|---|---|
| `dictation_enabled` | `true` | Off keeps the daemon resident but idle |
| `dictation_microphone` | `None` | `None` follows the system default |
| `dictation_hotkey` | `"Super+Space"` | Held, not struck |
| `dictation_gpu` | `true` | The same switch as `--cpu` |

A microphone name that no longer matches any device falls back to the default
rather than failing to record.

> **The default hotkey collides with the compositor.** `crownpositor` binds
> `Super+Space` to `cycle-layout`. Because `crowndictator` reads the keyboard
> directly, **both fire**. Rebind one of them.

## Design note

`controller.rs` is worth reading. Hotkey edges and settings changes are both
`Event`s on **one channel**, which is what makes "the user switched dictation off
while holding the chord" an ordinary sequence of messages rather than a race.

`settings.rs` is 40 lines and is the model to copy when bringing another CrownOS
component onto the shared config convention.

## Tests

```bash
cargo test
```

Eleven unit tests: ten in `src/hotkey.rs` covering chord parsing and matching,
one in `src/controller.rs`. They need no model, no microphone and no
`/dev/input` access. There are no integration tests, examples or benches.

## Known limitations

- **CUDA is not optional at build time**, and `ort` is declared with
  `download-binaries`, so `cargo build` fetches a prebuilt ONNX Runtime over the
  network. That also means the docs.rs build fails permanently — docs.rs blocks
  network access — so this crate will not have rendered documentation until the
  ASR backend sits behind an optional feature.
- `ort` is declared `default-features = false` with `tls-rustls` rather than the
  default `tls-native`. That is deliberate: the default drags in native-tls and
  therefore system OpenSSL, which no other CrownOS crate needs, and it was the
  only reason this crate would not compile. Do not restore the defaults.
- It pulls `crownos-config` with default features, which includes `xilem` — a
  headless daemon dragging in a GUI toolkit. `default-features = false` is
  probably correct.
- First run needs network access and several gigabytes of disk.

## Contributing

See the organization-wide
[contribution guide](https://github.com/Crown-OS/crownos-documentations/blob/main/CONTRIBUTING.md).
Default branch here is **`main`**.

## License

Licensed under the [MIT License](LICENSE).
