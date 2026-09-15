# Keybindings

Every chord and gesture CrownOS ships, and the action vocabulary they resolve
to. Derived from the code at `1f8deee`.

## Where bindings come from

Three independent tables, with three different owners:

| Table | Owner | Configurable in |
|---|---|---|
| Compositor chords | `crownpositor`, `Bindings` | `keybinds.ron` → `custom_keybinds` |
| Trackpad swipes | `crownpositor`, `GestureBindings` | not configurable |
| Dictation push-to-talk | `crowndictator`, evdev grab | `input.ron` → `dictation_hotkey` |

`keybinds.ron` also holds `launcher` (default `Super+Space`) and `dictation`
(default `Super+Ctrl`). **Nothing reads them** — a repo-wide search finds no
consumer outside the config crate's own tests. The only field of the `keybinds`
section the compositor reads is `custom_keybinds`.

Config files live at `~/.config/crownos/<section>.ron` and are watched live — a
rebind takes effect without a restart.

## Default chords

32 rows, from `Bindings::defaults()` in
`crates/crownpositor/src/input/shortcuts/bindings.rs`. These apply when
`custom_keybinds` is empty.

| Chord | Action |
|---|---|
| `Super+Shift+E` | `quit` |
| `Super+Return` | `spawn foot` |
| `Super+Q` | `close-window` |
| `Super+H` | `focus left` |
| `Super+L` | `focus right` |
| `Super+K` | `focus up` |
| `Super+J` | `focus down` |
| `Super+Shift+H` | `move left` |
| `Super+Shift+L` | `move right` |
| `Super+Shift+K` | `move up` |
| `Super+Shift+J` | `move down` |
| `Super+Tab` | `workspace +1` |
| `Super+Shift+Tab` | `workspace -1` |
| `Super+1` | `workspace 0` |
| `Super+2` | `workspace 1` |
| `Super+3` | `workspace 2` |
| `Super+4` | `workspace 3` |
| `Super+Shift+1` | `move-to-workspace 0 follow` |
| `Super+Shift+2` | `move-to-workspace 1 follow` |
| `Super+Shift+3` | `move-to-workspace 2 follow` |
| `Super+Shift+4` | `move-to-workspace 3 follow` |
| `Super+V` | `toggle-float` |
| `Super+F` | `toggle-fullscreen` |
| `Super+M` | `toggle-maximize` |
| `Super+Space` | `cycle-layout` |
| `Super+Shift+Space` | `toggle-layout-mode` |
| `Super+Shift+C` | `reload-config` |
| `Super+Ctrl+L` | `resize-split 0.05` |
| `Super+Ctrl+H` | `resize-split -0.05` |
| `Super+P` | `promote` |
| `Super+R` | `cycle-size` |
| `Super+Shift+R` | `reset-size` |

Workspace numbers in actions are 0-based; the `Super+<n>` chords are offset by
one so the keycap matches the workspace a user counts from 1.

Lookup prefers the layout-independent symbol
(`raw_latin_sym_or_raw_current_sym`) and falls back to the modified symbol, so
`Super+Q` stays `Super+Q` on Dvorak or a Cyrillic keymap. Only four modifiers
participate — logo, ctrl, alt, shift. `caps_lock`, `num_lock` and
`iso_level3/5_shift` are deliberately excluded from the mask, so bindings still
match with CapsLock on or on an AltGr layout.

## Wired outside the table

**VT switching.** `Ctrl+Alt+F1`…`Ctrl+Alt+F12` switch virtual terminals. Matched
in the keyboard filter on the `XF86Switch_VT_1..12` modified symbols, before the
binding table and before any shortcut inhibitor — the compositor holds the evdev
devices, so nothing can take this away. A keymap without the
`srvr_ctrl(fkey2vt)` level never matches it.

**Shortcut inhibition.** A client holding a `keyboard-shortcuts-inhibit`
inhibitor *while its own surface has keyboard focus* bypasses the whole binding
table, including modifier-only chords. VT switching is not bypassed. A `TODO` in
the keyboard filter notes that session lock state is not checked yet, so a lock
screen cannot currently prevent `Super+Q`.

**Modifier-only chords** fire on the *release* edge, and only if no other key
was pressed while the modifier was held; changing or extending the modifier set
re-arms them. No default uses one — they exist only via `custom_keybinds`.

## Trackpad gestures

Hardcoded in `GestureBindings::defaults()`. **Not configurable** — there is no
`from_config`, and `State` constructs it from `defaults()` unconditionally.

| Gesture | Action |
|---|---|
| 3-finger swipe left→right | `workspace -1` |
| 3-finger swipe right→left | `workspace +1` |
| 4-finger swipe bottom→top | `open-workspace-view` (not implemented) |
| 4-finger swipe top→bottom | `close-workspace-view` (not implemented) |

A **4-finger horizontal** swipe never reaches this table: it is intercepted
during the update stream and drives the workspace viewport interactively,
committing on release. ~5 cm of finger travel is one workspace.

Thresholds (unaccelerated, libinput-normalised to 1000 dpi): a swipe locks to an
axis after ~2 mm, and commits on release after ~25 mm of travel or at ~10 cm/s;
below both it snaps back. Two- and five-finger swipes are recognised but nothing
is bound to them.

## Action vocabulary

From `crates/crownpositor/src/input/shortcuts/action.rs`. An action string is
split on whitespace: the first token names the action, the rest are arguments.

| Action | Aliases | Arguments | Notes |
|---|---|---|---|
| `none` | | | No-op. |
| `quit` | `exit` | | |
| `reload-config` | | | |
| `spawn` | | `<program> [args…]` | Split on whitespace; no quoting or escaping. |
| `switch-vt` | | `<n>` | For putting a VT on some other chord; `Ctrl+Alt+F<n>` is already wired. |
| `close-window` | `close` | | |
| `focus` | | `<direction>` | |
| `move` | `move-window` | `<direction>` | |
| `focus-output` | | `<direction>` | |
| `move-to-output` | | `<direction>` | |
| `move-workspace-to-output` | | `<direction>` | **Not implemented** — logs a warning. |
| `workspace` | | `<workspace>` | |
| `move-to-workspace` | | `<workspace> [follow]` | `follow` is opt-in. |
| `toggle-float` | `toggle-floating` | | |
| `toggle-fullscreen` | | | |
| `toggle-maximize` | | | |
| `toggle-layout-mode` | | | Flips the compositor-wide default; per-workspace overrides stay. |
| `cycle-layout` | | | This workspace's override: none → master → scrolling → none. |
| `set-layout` | | `<layout>` | |
| `open-workspace-view` | | | **Not implemented** — logs a warning. |
| `close-workspace-view` | | | **Not implemented** — logs a warning. |
| `resize-split` | | `<fraction>` | Signed `f64`; grows or shrinks the primary split. |
| `promote` | `demote` | | Into or out of the master area; full width in a scrolling layout. |
| `cycle-size` | | | Cycles the focused window through the layout's presets. |
| `reset-size` | | | |

Argument grammars:

- `<direction>` — `left`, `right`, `up`, `down`.
- `<workspace>` — a bare number is an absolute 0-based index, clamped to the
  list; `+N` / `-N` is relative and does **not** wrap; `prev` or `previous` is
  the last workspace visited.
- `<layout>` — `master-stack` or `master`, `scrolling-columns` or `scrolling`,
  `floating`.

**Action arguments are case-sensitive**, unlike chords. `focus Left` is the
error `invalid direction \`Left\``, and `move-to-workspace 0 FOLLOW` silently
does not follow.

## Configuring chords

`~/.config/crownos/keybinds.ron`:

```ron
(
    launcher: "Super+Space",
    dictation: "Super+Ctrl",
    custom_keybinds: [
        (keys: "Super+Return", action: "spawn foot"),
        (keys: "Super+D",      action: "spawn crownlauncher"),
    ],
)
```

Custom chords moved here from the `compositor` section. A `keybinds:` list left
behind in `compositor.ron` is ignored — `Compositor` has no such field.
`session/compositor.example.ron` still ships that stale example.

Semantics of `custom_keybinds`:

- **Empty means defaults.** `Bindings::from_config` returns
  `Bindings::defaults()` when the list is empty. It is not "no bindings".
- **Non-empty replaces the whole table.** The defaults are *not* merged in. One
  custom row means one binding total; the other 31 defaults are gone, including
  `quit`.
- **One `(keys: "None", …)` row is how you ask for an empty table.** `"None"`
  parses to a chord with no modifiers and no key, which binds nothing.
- **A bad row is skipped, not fatal.** An unparseable chord or action logs a
  `warn` naming the row and the rest of the list is kept.

By contrast, a bad `launcher` or `dictation` spelling is a *deserialize* error,
which `load` turns into `unwrap_or_default()` — the entire `keybinds` section
silently reverts to defaults, taking `custom_keybinds` with it.

## Two chord parsers, two spellings

This is the sharpest edge in the whole system. `custom_keybinds` rows are parsed
by `crownpositor`'s own `Chord`/`keysym_from_name`; `launcher`, `dictation` and
`input.ron`'s `dictation_hotkey` are parsed by `crownos-config`'s `Keybind` via
`KeyCode::from_label`. Both are case-insensitive, order-insensitive, tolerate
spaces around `+`, and accept `"None"` as unbound. They accept different names.

Modifiers:

| Modifier | `custom_keybinds` | `launcher` / `dictation` / `dictation_hotkey` |
|---|---|---|
| Logo | `super`, `logo`, `meta`, `mod4`, `cmd` | `Super`, `Meta`, `Cmd`, `Win` |
| Control | `ctrl`, `control` | `Ctrl`, `Control` |
| Alt | `alt`, `mod1`, `option` | `Alt`, `Option` |
| Shift | `shift` | `Shift` |

So `logo`, `mod4` and `mod1` work only in `custom_keybinds`, and `Win` only in
the config-crate fields.

Keys:

- `custom_keybinds` accepts **any single ASCII graphic character** as itself
  (`q`, `1`, `-`, `[`, `/`), plus the named keys `return`/`enter`, `space`,
  `tab`, `escape`/`esc`, `backspace`, `delete`/`del`, `home`, `end`,
  `pageup`/`prior`, `pagedown`/`next`, `insert`, `left`, `right`, `up`, `down`,
  `f1`–`f12`.
- `Keybind` accepts a **closed list of labels**: `A`–`Z`, `0`–`9`, `F1`–`F12`,
  `Space`, `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Insert`, `Home`,
  `End`, `PageUp`, `PageDown`, `CapsLock`, `Up`, `Down`, `Left`, `Right`,
  `Minus`, `Equal`, `LeftBracket`, `RightBracket`, `Backslash`, `Semicolon`,
  `Quote`, `Backquote`, `Comma`, `Period`, `Slash`.

Where they disagree:

| Spelling | `custom_keybinds` | `launcher` / `dictation` |
|---|---|---|
| `Enter` | ok | ok |
| `Return` | ok | **error** |
| `Esc`, `Del`, `Prior`, `Next` | ok | **error** |
| `-` `=` `[` `]` `\` `;` `'` `` ` `` `,` `.` `/` | ok, as the literal character | **error** |
| `Minus`, `Equal`, `LeftBracket`, `Comma`, `Period`, `Slash`, … | **error** | ok |
| `CapsLock` | **error** | ok |
| `""` (empty string) | **error** | parses as unbound |
| `"Super+"` | ok — trailing empty token dropped, yields bare `Super` | **error** |

Neither parser accepts W3C `code` spellings (`KeyA`, `Digit1`, `ArrowLeft`).
Neither accepts more than one ordinary key in a chord. Shift lives in the
modifier mask, never in the key: `Super+Shift+Q` is `{shift} + q`, not `Q`.

## Known conflicts

- **`Super+Space` is claimed three times.** The compositor's default
  `cycle-layout` binding, `keybinds.launcher`, and `input.ron`'s
  `dictation_hotkey` all default to it. Today the compositor wins the chord and
  `crowndictator` — which reads `/dev/input/event*` directly rather than going
  through the compositor — also sees it, so holding `Super+Space` both cycles
  the layout and starts recording. `keybinds.launcher` is unread, so it is
  inert; if it is ever wired up, this becomes a three-way conflict.
- **`keybinds.dictation` (`Super+Ctrl`) is not the chord dictation uses.**
  `crowndictator` reads `input.dictation_hotkey`, which is a separate field
  defaulting to `Super+Space`.
- The doc comments on `Keybind::SUPER_SPACE` and `Keybind::SUPER_CTRL` are
  swapped relative to what the schema does with them — `SUPER_SPACE` is
  described as the push-to-talk default and `SUPER_CTRL` as the launchpad
  default, while `Keybinds` uses them the other way round.
