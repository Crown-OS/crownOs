# When something does not work

Symptom first. Every entry here is a failure someone actually hit, not one we
imagined.

## It will not build

**`pkg-config` not found, or a `*-sys` crate panics in its build script**

A system library is missing, not a Rust one. `cargo` cannot install these.

```bash
./bootstrap.sh --check      # names exactly what is absent
./bootstrap.sh --deps-only  # installs it
```

If `--check` says everything is present and the build still fails, that is a bug
in `deps.toml` — report it rather than working around it locally.

**`failed to select a version for the requirement crownshell = "^0.3"`**

You are building an old checkout from before the workspace merge. The nine
crates live in one repository now and resolve each other by path; there is no
version to satisfy. `git pull` and delete any `.cargo/config.toml` left above
your checkouts from the old setup — it can only confuse cargo now.

**`cargo build` works but `cargo build --locked` fails**

Your `Cargo.lock` disagrees with the manifests. Commit the lockfile change, or
if you did not mean to change it, `git checkout Cargo.lock`.

## The compositor will not start

**Nothing on screen, no error, and it exits**

Run it with logging. It writes to stderr, not the journal — despite
`tracing-journald` being declared, nothing installs that subscriber:

```bash
RUST_LOG=crownpositor=debug crownpositor 2>&1 | tee /tmp/crownpositor.log
```

**`EGLDisplay::new` panics, or `make_sure_egl_is_loaded` fails**

libEGL is not on the loader path. This bites when the binary was built somewhere
other than where it runs — a container, a VM sharing the host's build, a Nix
shell whose environment is gone. Either run it in the same environment you built
it in, or set `LD_LIBRARY_PATH` to include your GL stack.

**DRM errors, or it opens `/dev/dri/card0` and then dies**

The KMS backend needs DRM master, which means a seat:

```bash
sudo systemctl start seatd     # or use a logind session
groups | grep -q seat || sudo usermod -aG seat "$USER"   # then log out and back in
```

To rule the seat out entirely, run nested — `CROWN_BACKEND=winit` needs neither
seat nor DRM.

**It starts, but the screen is black and you cannot get back**

Switch TTY: `Ctrl+Alt+F1` through `F6`. Have a TTY logged in *before* you start
the compositor, so returning to it is not something that also has to work.
`Super+Shift+E` quits from inside.

## The desktop is missing pieces

**The compositor runs but there is no bar, dock or notifications**

Check what it tried to spawn:

```bash
grep startup /tmp/crownpositor.log
```

`compositor.startup` defaults to `["crownbar","crowndock","crownotify"]`. If you
have a `compositor.ron` from before that default existed, it probably has
`startup: []` — which is a valid way to ask for a bare compositor, so nothing
warns. Add the entries, or delete the file and let it regenerate.

If the log says it spawned them and they are not there, they are not on `PATH`.
The compositor execs them by name.

**A component starts and immediately disappears**

You will not see why. `crownpositor`'s `spawn` sets the child's stdout and stderr
to `/dev/null`, so anything it prints is lost. Run it by hand against the running
compositor instead:

```bash
grep 'socket' /tmp/crownpositor.log        # find the socket name, e.g. wayland-2
WAYLAND_DISPLAY=wayland-2 RUST_LOG=debug crownbar
```

**The dock is empty**

Expected. `crowndock` draws pinned items from its own `items.toml` and has no
foreign-toplevel handling, so it never shows running windows. With nothing
pinned there is nothing to draw. It also initialises no logger, so it cannot
tell you that.

**Notifications do not appear**

Something else already owns `org.freedesktop.Notifications` — dunst, mako, a
desktop environment's own daemon. Only one process can hold it:

```bash
busctl --user status org.freedesktop.Notifications
```

## Configuration does not do anything

**A setting has no effect**

Most likely the file failed to parse. **A file that does not parse silently
reverts that whole section to defaults, with no error anywhere.** One bad line
loses the entire file's worth of settings.

```bash
# does it parse at all?
python3 -c "print(open('$HOME/.config/crownos/compositor.ron').read())" >/dev/null
RUST_LOG=crownos_config=debug crownpositor 2>&1 | grep -i 'pars\|skip'
```

**A keybinding does nothing**

Almost always the key name. Key names are **labels**, not W3C code spellings:

| Write | Not |
|---|---|
| `A` | `KeyA` |
| `1` | `Digit1` |
| `Left` | `ArrowLeft` |
| `Space` | `Space` ✓ |

A chord that does not parse is skipped with a warning; a *file* that does not
parse takes everything with it.

Note also that the two config sections are parsed by **different** parsers with
different accepted spellings — see [keybindings.md](keybindings.md).

**`Super+Space` does two things at once**

Known conflict. `keybinds.launcher` defaults to `Super+Space` and the
compositor's own default binds the same chord to `cycle-layout`. Rebind one.

**The accent colour changes nothing**

`AccentColor` is carried through the schema but never converted to an RGB value
anywhere, so nothing renders it yet.

## Getting help

Include the output of `./bootstrap.sh --check`, your distribution, and the
compositor log. The first two answer most questions before anyone has to ask.
