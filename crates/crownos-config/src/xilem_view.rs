//! Realtime config sync as a [xilem] view.
//!
//! [`watch`] returns a view that owns a subscription for as long as it is in
//! the tree. When the file changes, the parsed value arrives on the main
//! thread through the normal view-message path and your closure edits app
//! state — no locks, no `Arc<Mutex<AppState>>`, and a rebuild happens for free
//! because xilem rebuilds after every message.
//!
//! ```ignore
//! use crownos_config::schema::Appearance;
//! use crownos_config::xilem_view::watched;
//! use xilem::WidgetView;
//! use xilem::view::flex_col;
//!
//! fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
//!     watched(
//!         flex_col((/* the real UI */)),
//!         Appearance::SECTION,
//!         |state: &mut AppState, appearance: Appearance| state.appearance = appearance,
//!     )
//! }
//! ```
//!
//! # Watching one key
//!
//! [`watched`] hands you the whole section. When your state only mirrors one
//! field, [`watched_key`] takes a [`Key`] and delivers just that field's value —
//! and stays quiet when the write changed some *other* field of the same file,
//! so unrelated edits cost no rebuild:
//!
//! ```ignore
//! use crownos_config::schema::appearance;
//!
//! watched_key(
//!     flex_col((/* the real UI */)),
//!     appearance::BarHeight,
//!     |state: &mut AppState, bar_height: u32| state.bar_height = bar_height,
//! )
//! ```
//!
//! The closure's `u32` is not an annotation you chose — it is
//! `<appearance::BarHeight as Key>::Value`. Renaming or retyping the field in
//! the schema breaks this call site instead of quietly delivering nothing.
//!
//! # Why this is not a `WidgetView`
//!
//! The underlying [`task_raw`] view produces
//! [`NoElement`] — it draws nothing — while `xilem::WidgetView` is defined as a
//! view whose element is a `Pod<W>`. So a watcher cannot sit inside a
//! `flex_col((..))` tuple the way a label can. Xilem's answer for
//! element-less views is [`fork`], which pairs a real view
//! with side-running ones; that is what [`watched`] wraps for you, and it is
//! how the upstream `variable_clock` example embeds `task`. Use [`watch`]
//! directly if you want to compose the `fork` yourself (e.g. to attach several
//! watchers at once, since `fork`'s second argument is a sequence).
//!
//! [xilem]: https://docs.rs/xilem
//! [`NoElement`]: xilem::core::NoElement

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use xilem::core::{MessageProxy, NoElement, View, fork};
use xilem::tokio::sync::mpsc;
use xilem::view::task_raw;
use xilem::{ViewCtx, WidgetView};

use crate::Key;

/// A view that keeps `section` in sync with your app state, drawing nothing.
///
/// The returned view is element-less, so it cannot go straight into a flex
/// tuple — pair it with a real view using [`fork`], or use [`watched`] which
/// does exactly that. It must stay in the view tree for the subscription to
/// stay alive; xilem tears the task down (and drops the subscription) when the
/// view leaves the tree.
///
/// `on_change` runs on the main thread with `&mut State`, once per successful
/// parse. Malformed or unchanged files never reach it.
pub fn watch<T, State, F>(
    section: &'static str,
    on_change: F,
) -> impl View<State, (), ViewCtx, Element = NoElement> + Send + Sync
where
    T: DeserializeOwned + Debug + Send + 'static,
    F: Fn(&mut State, T) + Send + Sync + 'static,
    State: 'static,
{
    task_raw(
        move |proxy: MessageProxy<T>| {
            // The notify callback is synchronous and must not block, so it
            // hands values to the async side through an unbounded channel.
            let (tx, mut rx) = mpsc::unbounded_channel::<T>();
            async move {
                // Held by the future: dropped when xilem aborts the task,
                // which unregisters the callback.
                let _subscription = crate::subscribe_typed::<T, _>(section, move |value| {
                    let _ = tx.send(value);
                });
                while let Some(value) = rx.recv().await {
                    if proxy.message(value).is_err() {
                        // Event loop is gone; nothing left to update.
                        break;
                    }
                }
            }
        },
        move |state: &mut State, value: T| on_change(state, value),
    )
}

/// [`watch`], narrowed to a single [`Key`] of the section.
///
/// `on_change` runs on the main thread with `&mut State` and the key's own
/// value type — `u32` for `appearance::BarHeight`, `bool` for
/// `appearance::DarkMode`. A write that leaves the key alone — some other field
/// of the same file — never reaches `on_change`, so it costs no rebuild.
///
/// ```ignore
/// use crownos_config::schema::appearance;
///
/// watch_key(appearance::BarHeight, |state: &mut AppState, bar_height: u32| {
///     state.bar_height = bar_height;
/// })
/// ```
///
/// Like [`watch`], the returned view draws nothing and must stay in the tree —
/// see [`watched_key`] for the paired-with-a-real-view version.
pub fn watch_key<K, State, F>(
    key: K,
    on_change: F,
) -> impl View<State, (), ViewCtx, Element = NoElement> + Send + Sync
where
    K: Key,
    K::Value: Debug,
    F: Fn(&mut State, K::Value) + Send + Sync + 'static,
    State: 'static,
{
    task_raw(
        // `task_raw` takes an `Fn` — it may run again on a re-subscribe — so the
        // key is copied in rather than moved. Keys are `Copy` and zero-sized.
        move |proxy: MessageProxy<K::Value>| {
            let (tx, mut rx) = mpsc::unbounded_channel::<K::Value>();
            async move {
                let _subscription = crate::subscribe_key(key, move |value| {
                    let _ = tx.send(value);
                });
                while let Some(value) = rx.recv().await {
                    if proxy.message(value).is_err() {
                        break;
                    }
                }
            }
        },
        move |state: &mut State, value: K::Value| on_change(state, value),
    )
}

/// [`view`](WidgetView) with a config watcher running alongside it.
///
/// Equivalent to `fork(view, watch(section, on_change))`, and the ergonomic
/// entry point: wrap your top-level view once per section you care about.
///
/// ```ignore
/// watched(my_ui(state), Display::SECTION, |s: &mut AppState, d: Display| s.display = d)
/// ```
pub fn watched<V, T, State, F>(
    view: V,
    section: &'static str,
    on_change: F,
) -> impl WidgetView<State, ()>
where
    V: WidgetView<State, ()>,
    T: DeserializeOwned + Debug + Send + 'static,
    F: Fn(&mut State, T) + Send + Sync + 'static,
    State: 'static,
{
    fork(view, watch::<T, State, F>(section, on_change))
}

/// [`view`](WidgetView) with a single-key watcher running alongside it.
///
/// Equivalent to `fork(view, watch_key(key, on_change))`. This is the one to
/// reach for when your state only mirrors part of a section.
///
/// ```ignore
/// watched_key(
///     my_ui(state),
///     appearance::BarHeight,
///     |state: &mut AppState, bar_height: u32| state.bar_height = bar_height,
/// )
/// ```
///
/// For several keys at once, `fork` takes a sequence — pass a tuple of
/// [`watch_key`]s rather than nesting `watched_key` calls:
///
/// ```ignore
/// fork(
///     my_ui(state),
///     (
///         watch_key(appearance::DarkMode, set_dark),
///         watch_key(display::Brightness, set_brightness),
///     ),
/// )
/// ```
pub fn watched_key<V, K, State, F>(view: V, key: K, on_change: F) -> impl WidgetView<State, ()>
where
    V: WidgetView<State, ()>,
    K: Key,
    K::Value: Debug,
    F: Fn(&mut State, K::Value) + Send + Sync + 'static,
    State: 'static,
{
    fork(view, watch_key::<K, State, F>(key, on_change))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{appearance, display, Appearance};

    #[derive(Default)]
    struct AppState {
        bar_height: u32,
        brightness: f64,
    }

    /// Builds the views a real app would build. Nothing is driven — the point
    /// is that the generic bounds are satisfiable from the outside, which the
    /// `ignore`d doc examples cannot check.
    #[test]
    fn views_are_constructible() {
        let state = AppState::default();

        // No turbofish and no type annotations: the key decides both.
        let _key = watch_key(appearance::BarHeight, |state: &mut AppState, bar_height| {
            state.bar_height = bar_height;
        });
        let _paired = watched_key(
            xilem::view::label("ui"),
            appearance::BarHeight,
            |state: &mut AppState, bar_height| state.bar_height = bar_height,
        );

        // Several watchers at once, the `fork`-with-a-sequence shape the docs
        // point at — including keys from two different sections.
        let _many = fork(
            xilem::view::label("ui"),
            (
                watch_key(appearance::DarkMode, |_state: &mut AppState, _on: bool| {}),
                watch_key(display::Brightness, |state: &mut AppState, brightness| {
                    state.brightness = brightness;
                }),
                watch::<Appearance, AppState, _>(Appearance::SECTION, |state: &mut AppState, a| {
                    state.bar_height = a.bar_height
                }),
            ),
        );

        assert_eq!(state.bar_height, 0);
        assert_eq!(state.brightness, 0.0);
    }

    /// The value type comes from the key, so this is a real compile-time check
    /// that the schema and the call site agree.
    #[test]
    fn key_value_types_come_from_the_schema() {
        fn assert_value<K: Key<Value = V>, V>(_key: K) {}

        assert_value::<appearance::BarHeight, u32>(appearance::BarHeight);
        assert_value::<appearance::DarkMode, bool>(appearance::DarkMode);
        assert_value::<display::Brightness, f64>(display::Brightness);
    }
}
