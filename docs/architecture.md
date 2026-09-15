# How CrownOS fits together

Nine crates in one workspace. This is what each is for and, more usefully, where
the boundaries are — because most of the design is in which crate is *not*
allowed to know a thing.

## The shape

```
                         crownpositor
                    the Wayland server. It owns
                    the socket, the seat and the
                    outputs, and it spawns the rest.
                              │
                    wlr-layer-shell
      ┌───────────────┬───────┴───────┬────────────────┐
   crownbar       crowndock       crownotify      crowndictator
      └───────────────┴───────────────┴────────────────┘
                              │ all built on
                        crownshell
              layer-shell surfaces + Vello painting

   crownos-config   the settings schema. Read by the compositor,
                    the bar and the dictation daemon, written by
                    whatever edits settings. Nobody's private state.

   crownuikit       widget kit on xilem, for the settings panel.
                    Currently has no consumer.
```

## crownpositor — the session

Not a window manager that runs *in* a session; it **is** the session. It creates
the Wayland socket, exports `WAYLAND_DISPLAY`, and spawns everything in
`compositor.startup` with that variable already set. Kill it and the desktop is
gone, because there was never anything else holding it up.

Two backends. `winit` renders into a window inside an existing session — the
development loop. `kms` drives DRM directly from a TTY and needs a seat, from
either seatd or logind. `CROWN_BACKEND` picks one; unset autodetects.

Its own configuration is *compiled*: `crownpositor-config` turns the on-disk
strings into regexes, parsed chords and pixel geometry once, so the hot paths
never parse anything. That crate is also the adapter between the shared schema
and the compositor's internal vocabulary — when the schema says
`WorkspaceMode::Tiling`, it decides which layout algorithm that means.

## crownshell — the surface framework

Every panel-like thing in CrownOS is a layer-shell surface painted with Vello,
and this crate is the only place that knows how either of those works. It owns
the Wayland protocol objects, the wgpu device, the swapchain, text shaping
through Parley, and one `calloop` event loop that can drive several windows.

A consumer implements `SurfaceHandler`, pushes into a `Scene`, and does not
mention Wayland. **When a component reaches past crownshell to
`smithay-client-toolkit` directly, that is a missing crownshell API, not a
shortcut** — it means two crates now have to agree about protocol details that
only one of them should know.

## crownos-config — the contract, not a library

The interesting property is that it has no opinions. It is a schema plus RON
serialisation plus a file watcher; it knows nothing about bars, compositors or
GPUs. Every consumer reads the same section definitions, so "what is
`bar_height`" has exactly one answer.

Configuration is also the IPC. There is no D-Bus interface for "set the accent
colour" — the settings panel writes `appearance.ron`, every running component is
watching it, and they all update. That is why the schema is shared rather than
each component having its own config file: the file *is* the message.

One consequence worth internalising: **a change to a section is a breaking
change to every consumer**, and the only thing that catches it is that they all
compile together. They did not always live in one repository, and a schema
change did break the compositor for eight days without anything noticing.

`xilem` is an optional, off-by-default feature. It exists for the settings
panel's view helpers, and defaulting it on dragged an entire GPU stack into
crates that only wanted to read a `u32`.

## The shell components

`crownbar`, `crowndock`, `crownotify` and `crowndictator` are separate processes
on purpose. A panic in the dock does not take the notifications with it, and
each can be restarted without touching the session. They share nothing but
crownshell and the config schema.

They are also independently optional. Removing one from `compositor.startup`
removes it from the desktop, and nothing else notices.

## crownuikit — the odd one out

A widget kit built on xilem/Masonry rather than crownshell, because a settings
panel is a conventional application window and not a layer-shell surface. It has
no consumer today: the settings panel it exists for has not been written.

That also means it sits on a different graphics stack from everything else —
xilem pulls its own wgpu and Vello. Two stacks coexisting is a real cost and a
known open question, not an accident.

## Boundaries, stated as rules

These are the ones worth keeping:

- Only **crownshell** talks Wayland client protocol. Two components currently
  break this, both for the same missing API: `crowndock/src/dock_handler.rs:5`
  and `crowndictator/src/main.rs:31` import
  `smithay_client_toolkit::{compositor::Region, shell::WaylandSurface}` to set
  an input region. One `SurfaceCtx::set_input_region` on crownshell removes both.
- Only **crownpositor** talks Wayland server protocol.
- **crownos-config** depends on nothing in CrownOS and never will. Everything
  depends on it, which is exactly why it must stay small.
- A component may not read another component's files. `crowndock` keeping its
  own `items.toml` is the one current violation, and it is why `dirs` appears
  twice in the tree at two major versions.

## Where to look

| You want to change | Start in |
|---|---|
| How windows are arranged | `crates/crownpositor/src/layout/` |
| A keyboard shortcut's behaviour | `crates/crownpositor/src/input/shortcuts/` |
| How a surface is painted | `crates/crownshell/src/renderer.rs`, `text.rs` |
| A setting's name, type or default | `crates/crownos-config/src/schema/` |
| What the bar shows | `crates/crownbar/src/widgets/` |
| What starts with the session | `compositor.startup`, spawned in `crates/crownpositor/src/state/actions.rs` |
