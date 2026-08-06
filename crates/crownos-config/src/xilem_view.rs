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
