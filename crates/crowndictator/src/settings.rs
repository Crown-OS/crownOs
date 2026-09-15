//! What `~/.config/crownos/input.ron` says, and staying in step with it.
//!
//! Three of the four fields in that file are things this process is *doing*
//! right now — whether it is listening at all, which chord it is listening for,
//! and which microphone it will open — so none of them is read once at startup
//! and remembered. The settings panel writes the file, the watcher below
//! notices, and the running daemon changes what it is doing without anybody
//! restarting it.
//!
//! The whole module is one function and a type alias, because there is nothing
//! more to it than that: [`crownos_config`] does the watching, and this decides
//! only what a change is delivered *as*. It is delivered as the whole section
//! rather than as a per-key callback so the controller — which is the thing
//! that has to act on it — sees one consistent snapshot instead of four
//! independent edits arriving in whatever order the file's fields happen to be
//! parsed in.

use crownos_config::Subscription;
use crownos_config::schema::Input;

/// Read the section as it stands, materialising the file with defaults if this
/// is a machine where nothing has written it yet.
pub fn load() -> Input {
    crownos_config::load::<Input>(Input::SECTION)
}

/// Call `on_change` with every later version of the section.
///
/// The callback runs on the watcher's own thread, so it should do nothing but
/// hand the value on to whoever can act on it.
///
/// The returned [`Subscription`] is the registration: dropping it stops the
/// delivery, so the caller has to hold it for as long as it wants to keep
/// hearing about edits. That is why this hands one back rather than quietly
/// leaking it — a daemon that stopped following its own config after a while
/// would be a very confusing bug.
#[must_use = "dropping the subscription stops the daemon following input.ron"]
pub fn watch(on_change: impl Fn(Input) + Send + Sync + 'static) -> Subscription {
    crownos_config::subscribe_typed::<Input, _>(Input::SECTION, on_change)
}
