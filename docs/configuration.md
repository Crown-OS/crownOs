# CrownOS configuration

Every setting lives in a RON file under one directory, one file per section, and
the schema in `crates/crownos-config/src/schema/` is the only definition of it.

## Where the files are

The directory is `$CROWN_CONFIG_DIR` when that is set and non-empty, otherwise
`dirs::config_dir()/crownos` — on Linux `$XDG_CONFIG_HOME/crownos`, normally
`~/.config/crownos` — and `./crownos` if there is no config directory at all
(`config.rs:6`). A section named `wifi` is the file `<dir>/wifi.ron`
(`util.rs:22`).

| File | Type | Read by |
| --- | --- | --- |
| `appearance.ron` | `Appearance` | crownbar, crownpositor |
| `compositor.ron` | `Compositor` | crownpositor |
| `keybinds.ron` | `Keybinds` | crownpositor |
| `input.ron` | `Input` | crowndictator |
| `display.ron` | `Display` | crownpositor |
| `power.ron` | `Power` | — |
| `sound.ron` | `Sound` | — |
| `wifi.ron` | `Wifi` | — |
| `bluetooth.ron` | `Bluetooth` | — |
| `notifications.ron` | `Notifications` | — |

The last five are schema-only today: no crate in this repository reads them yet.

## A file that fails to parse silently reverts the whole section

Read this before hand-editing anything.

`load` is `options().from_str(&text).unwrap_or_default()` (`parser.rs:27`): no
error, no log line, no message anywhere. One bad value in `compositor.ron` means
**every** field in that file goes back to its default — your window rules, your
outputs, your startup list. The file on disk is left untouched, so the next read
fails the same way. The live watcher drops unparseable contents too and the
running app keeps what it already had (`watch.rs:216-223`), so saving a broken
file looks like your edit did nothing.

Breaks the section: a misspelled enum variant (`accent: Teal`), a wrong type
(`bar_height: "44"`), malformed RON, or an unparseable shortcut string
(`"Supper+Space"` — `Keybind` deserializes through `FromStr`, `keybind.rs:416`).

Does not break it: a missing field (every generated struct carries
`#[serde(default)]`, `key.rs:118`, so writing only the fields you want to change
is the correct way to use these files), an unknown field, a trailing comma, or
the struct name in front of the parens (`Appearance(dark_mode: false)`).

There is no validator. To recover, delete the file: the next `load` writes the
defaults back out (`parser.rs:29-31`).

## RON in these files

- A section is a parenthesised struct: `( field: value, ... )`.
- `Option` fields are written as the bare value — `network: "home"`, not
  `Some("home")`. `parser.rs:11` enables `IMPLICIT_SOME`. Explicit `Some(..)`,
  which is what `save` emits, still parses. Omit the field, or write `None`, for
  unset.
- Enums are bare variant names: `accent: Blue`, `transform: R90`.
- Tuples are parenthesised: `position: (0, 0)`.
- An integer literal is accepted where an `f64` is expected.
- `save` writes pretty RON to `<section>.ron.tmp` and renames it over the target
  (`parser.rs:44-60`), so a reader never sees a half-written file.

## appearance.ron

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `dark_mode` | bool | `true` | Light-on-dark theme. |
| `accent` | `Purple` \| `Blue` \| `Green` \| `Orange` \| `Pink` | `Purple` | Accent colour. |
| `transparency` | f64 | `0.0` | How see-through windows are. crownpositor clamps to `0.0..=1.0` and uses `1.0 - transparency` as opacity. |
| `wallpaper` | String | `""` | Wallpaper path. |
| `bar_height` | u32 | `32` | crownbar height, px. |
| `gaps_inner` | u16 | `8` | Px between tiled windows. |
| `gaps_outer` | u16 | `8` | Px between the tiled area and the output edge. |
| `border_width` | u16 | `2` | Px. |
| `border_radius` | u16 | `8` | Px. |
| `animations` | `None` \| `Snappy` \| `Standard` \| `Smooth` | `Standard` | Animation profile. |
| `titlebar_height` | u16 | `36` | Px, on floating windows. |
| `snap` | bool | `true` | Edge snapping. |
| `appmenu` | bool | `true` | Application menu. |
| `blur` | bool | `true` | Background blur. |
| `blur_passes` | u16 | `3` | |
| `blur_size` | f64 | `1.5` | |
| `blur_noise` | f64 | `0.01` | |

Gaps, borders and animation are here, not in `compositor.ron`: theme, not window
management.

## compositor.ron

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `layout` | `WorkspaceMode` | `Tiling` | Default arrangement for new workspaces. |
| `focus_follows_mouse` | bool | `false` | |
| `window_rules` | `[WindowRule]` | `[]` | Matched at a window's first buffer commit. |
| `outputs` | `[OutputSetting]` | `[]` | |
| `startup` | `[String]` | `["crownbar", "crowndock", "crownotify"]` | Command lines spawned at session start. |

`WorkspaceMode` is `Tiling` (the compositor owns every window's geometry) or
`Floating` (windows keep their given size and position and wear a titlebar).

`startup` entries are command lines, not argv: split on whitespace with `"..."`
and `'...'` honoured, and no shell — no globbing, no `$VAR`, no `&&`
(`crates/crownpositor-config/src/startup.rs`). A missing binary is logged and
skipped. Write `startup: []` for a bare compositor with no bar, dock or
notifications. crowndictator is absent from the default on purpose: it downloads
700 MB-2.5 GB of model weights on first run.

### WindowRule

Every field is optional: omitted means "do not care" (matchers) or "leave alone"
(effects).

| Field | Type | Meaning |
| --- | --- | --- |
| `app_id` | String | Regex, unanchored: `"blender"` matches `"org.blender.Blender"`. |
| `title` | String | Regex, unanchored. |
| `floating` | bool | |
| `fullscreen` | bool | |
| `maximized` | bool | |
| `workspace` | u16 | Zero-based workspace index on the target output. |
| `output` | String | Connector name, or `"MAKE MODEL SERIAL"`. |
| `focus` | bool | `false` opens the window without stealing focus. |
| `opacity` | f32 | |
| `corner_radius` | u16 | |

### OutputSetting

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | String | Required. Connector name (`"eDP-1"`) or `"MAKE MODEL SERIAL"`. Defaults to `""`, which matches nothing. |
| `enabled` | bool | |
| `mode` | String | `"2560x1440@144.000"`. |
| `scale` | f64 | |
| `transform` | `Normal` \| `R90` \| `R180` \| `R270` \| `Flipped` \| `Flipped90` \| `Flipped180` \| `Flipped270` | |
| `position` | (i32, i32) | Layout position in the global space. |
| `vrr` | bool | |
| `layout` | `WorkspaceMode` | Overrides `layout` for workspaces created on this output. |

## keybinds.ron

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `launcher` | `Keybind` | `"Super+Space"` | Opens the launchpad. |
| `dictation` | `Keybind` | `"Super+Ctrl"` | Modifier-only, so it cannot collide with an application's own bindings. |
| `custom_keybinds` | `[Binding]` | `[]` | Compositor shortcuts. |

`Binding` is `(keys: "<chord>", action: "<action>")`; a row that fails to parse
is logged and skipped rather than failing the section.

**A non-empty `custom_keybinds` replaces the built-in bindings entirely** — it
does not merge with them (`crates/crownpositor/src/input/shortcuts/bindings.rs:213-218`).
Listing one chord leaves you with exactly one chord.

## input.ron

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `dictation_enabled` | bool | `true` | Off leaves the daemon resident but the shortcut inert and no microphone ever opened. |
| `dictation_microphone` | Option\<String\> | `None` | Device name from the audio host; `None` follows the system default. An unmatched name falls back to the default. |
| `dictation_hotkey` | `Keybind` | `"Super+Space"` | Push-to-talk: held to record, released to transcribe. |
| `dictation_gpu` | bool | `true` | Falls back to CPU automatically when there is no usable GPU. |
| `keyboard_repeat_rate` | u16 | `200` | |

`dictation_hotkey` defaults to the same chord as `keybinds.launcher`; rebind one
of them if you use both.

## display.ron

| Field | Type | Default |
| --- | --- | --- |
| `brightness` | f64 | `80.0` |
| `night_light` | bool | `false` |
| `night_light_warmth` | f64 | `50.0` |
| `scale` | `S100` \| `S125` \| `S150` \| `S200` | `S100` (factors 1.0, 1.25, 1.5, 2.0) |

## power.ron, sound.ron, wifi.ron, bluetooth.ron, notifications.ron

| File | Field | Type | Default |
| --- | --- | --- | --- |
| `power.ron` | `screen_off_minutes` | u32 | `10` |
| | `sleep_minutes` | u32 | `30` |
| | `power_profile` | `PowerSaver` \| `Balanced` \| `Performance` | `Balanced` |
| `sound.ron` | `output_volume` | f64 | `50.0` |
| | `input_volume` | f64 | `50.0` |
| | `muted` | bool | `false` |
| | `output_device` | Option\<String\> | `None` |
| `wifi.ron` | `enabled` | bool | `true` |
| | `network` | Option\<String\> | `None` |
| `bluetooth.ron` | `enabled` | bool | `false` |
| `notifications.ron` | `enabled` | bool | `true` |
| | `do_not_disturb` | bool | `false` |
| | `show_previews` | bool | `true` |

## Writing a shortcut

`keybinds.launcher`, `keybinds.dictation` and `input.dictation_hotkey` are
`Keybind` values, written as a string: modifiers and at most one ordinary key,
joined by `+`.

Parsing is case-insensitive and order-insensitive, and tolerates spaces around
the `+`, so `ctrl + alt + d` and `Alt+Ctrl+D` are the same chord
(`keybind.rs:369`). Printing is canonical: `Super`, `Ctrl`, `Alt`, `Shift`, then
the key.

Modifier spellings: `Super` / `Meta` / `Cmd` / `Win`, `Ctrl` / `Control`, `Alt` /
`Option`, `Shift`.

Keys are written as **labels**, not W3C code names: `A`, `1`, `Left`, `Space`.
`KeyA`, `Digit1` and `ArrowLeft` do not parse — `Keybind::from_str` goes through
`KeyCode::from_label` (`keybind.rs:387`). The complete list (`keybind.rs:175-256`):

- `A`–`Z`
- `0`–`9`
- `F1`–`F12`
- `Space`, `Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `Insert`, `Home`,
  `End`, `PageUp`, `PageDown`, `CapsLock`
- `Up`, `Down`, `Left`, `Right`
- `Minus`, `Equal`, `LeftBracket`, `RightBracket`, `Backslash`, `Semicolon`,
  `Quote`, `Backquote`, `Comma`, `Period`, `Slash`

Nothing else; a key outside this list is a parse error, which reverts the
section. Modifier-only is valid (`"Super"`, `"Super+Ctrl"`) and fires on release,
which is why it suits push-to-talk. Two ordinary keys is an error
(`"Super+Space+Enter"`). Unbound is a value, not an error: `""`, `"  "` and
`"None"` all read as nothing bound, and `save` writes `"None"`.

### Chords in custom_keybinds

`custom_keybinds[].keys` is parsed by the compositor, not by `Keybind`
(`crates/crownpositor/src/input/shortcuts/bindings.rs:67-152`), and accepts a
wider vocabulary: any single ASCII printable character as the key, the modifier
aliases `logo` / `mod4` / `mod1`, and `Return`, `Esc`, `Del`, `Prior`, `Next`.
Anything `Keybind` accepts, this accepts too.

### Actions

`custom_keybinds[].action` is a verb plus arguments, space-separated
(`crates/crownpositor/src/input/shortcuts/action.rs`).

Actions taking no argument: `none`, `quit` (`exit`), `reload-config`,
`close-window` (`close`), `toggle-float` (`toggle-floating`),
`toggle-fullscreen`, `toggle-maximize`, `toggle-layout-mode`, `cycle-layout`,
`open-workspace-view`, `close-workspace-view`, `promote` (`demote`),
`cycle-size`, `reset-size`.

| Action | Argument |
| --- | --- |
| `spawn <program> [args...]` | Program and argv, no shell. |
| `switch-vt <n>` | VT number. |
| `focus <dir>` | `left` \| `right` \| `up` \| `down` |
| `move` / `move-window <dir>` | Same directions. |
| `focus-output <dir>` | Same directions. |
| `move-to-output <dir>` | Same directions. |
| `move-workspace-to-output <dir>` | Same directions. |
| `workspace <ref>` | Index (`0`), relative (`+1`, `-1`), or `prev` / `previous`. |
| `move-to-workspace <ref> [follow]` | `follow` also switches to it. |
| `set-layout <name>` | `master-stack` \| `master` \| `scrolling-columns` \| `scrolling` \| `floating` |
| `resize-split <fraction>` | e.g. `0.05`, `-0.05` |

`set-layout` picks a tiling algorithm inside the compositor; it is not
`compositor.layout`, which is a `WorkspaceMode`. The built-in bindings a
non-empty `custom_keybinds` replaces are in `bindings.rs:141-197`.

## A worked compositor.ron

Verified against the real parser.

```ron
(
    layout: Tiling,
    focus_follows_mouse: true,

    window_rules: [
        (app_id: "blender", floating: true),
        (title: "^(Open|Save)", floating: true, focus: false),
        (app_id: "steam", workspace: 3, output: "DP-1"),
        (app_id: "foot", opacity: 0.95, corner_radius: 12),
    ],

    outputs: [
        (name: "eDP-1", mode: "2560x1440@144.000", scale: 2.0, position: (0, 0)),
        (name: "DP-1", position: (2560, 0), transform: R90, vrr: true, layout: Floating),
        (name: "HDMI-A-1", enabled: false),
    ],

    startup: [
        "crownbar",
        "crowndock",
        "crownotify",
        "swaybg -i \"/home/me/My Pictures/wall.png\"",
    ],
)
```

## Live reload

One filesystem watcher per process watches the config directory non-recursively
and fans events out per section (`watch.rs`). Create, modify and rename count;
removals do not. Each event reads the file, hashes it, and delivers only if the
hash differs from both the last hash delivered for that section and the last
hash this process wrote itself — which collapses duplicate inotify events, no-op
writes, and a process hearing its own `save` back as an external change.

crownpositor and crowndictator subscribe, so editing `compositor.ron`,
`keybinds.ron` or `input.ron` in `$EDITOR` takes effect without a restart.

Apps read with `crownos_config::load(Section::SECTION)`, write with `save`, and
follow changes with `subscribe_typed::<Section, _>` for a whole section or
`subscribe_key` for one field; a `Subscription` unregisters when dropped. Every
field declared through `section!` (`key.rs:103`) also generates a typed key —
`appearance::BarHeight` carries its section, field name and value type — and an
enum listing them all, `AppearanceKey::ALL`. Adding a field there is what adds
it to the file format.
