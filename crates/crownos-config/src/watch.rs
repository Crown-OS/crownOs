//! Realtime notification when a config file gets updated.
//!
//! There is exactly **one** [`notify`] watcher for the whole process. It is
//! created the first time anything subscribes, watches [`config_dir`]
//! non-recursively, and fans events out to per-section callbacks.
//!
//! ```ignore
//! use crownos_config::schema::Display;
//!
//! let sub = crownos_config::subscribe_typed::<Display, _>(Display::SECTION, |display| {
//!     apply_brightness(display.brightness);
//! });
//! // `sub` must be kept alive; dropping it unregisters the callback.
//! ```
//!
//! # What counts as a change
//!
//! Create, modify and rename events all count, because "save" means different
//! things to different editors — many write a temp file and rename it over the
//! target, which is a create+rename rather than a modify. Removals are
//! ignored: a deleted config is not a new value, and the create event for
//! whatever replaces it will arrive anyway.
//!
//! Every candidate event ends in the same place: read the file, hash it, and
//! deliver only if that hash differs from both the last hash delivered for the
//! section and the last hash [`save`](crate::save) wrote. That single check
//! collapses duplicate inotify events, no-op writes, and this process's own
//! saves.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::de::DeserializeOwned;

use crate::config_dir;
use crate::util::{hash_bytes, last_written, path_for};

type Callback = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

#[derive(Default)]
struct Registry {
    /// live callbacks, each tagged with its [`Subscription`] id.
    subscribers: HashMap<String, Vec<(u64, Callback)>>,
    /// hash of the contents most recently handed to callbacks.
    last_delivered: HashMap<String, u64>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| Mutex::new(Registry::default()));
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Holds the single process-wide watcher alive. `None` if it failed to start
/// (no inotify available, config dir unreadable, ...) — in that case
/// subscriptions are inert rather than fatal.
static WATCHER: LazyLock<Mutex<Option<RecommendedWatcher>>> =
    LazyLock::new(|| Mutex::new(start_watcher()));

fn lock_registry() -> std::sync::MutexGuard<'static, Registry> {
    REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn start_watcher() -> Option<RecommendedWatcher> {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);

    let mut watcher = notify::recommended_watcher(|res: notify::Result<Event>| {
        if let Ok(event) = res {
            handle_event(&event);
        }
    })
    .ok()?;
    watcher.watch(&dir, RecursiveMode::NonRecursive).ok()?;
    Some(watcher)
}

fn handle_event(event: &Event) {
    let interesting = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Any | EventKind::Other
    );
    if !interesting {
        return;
    }

    for path in &event.paths {
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }
        let Some(section) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        deliver(section);
    }
}

fn deliver(section: &str) {
    // Cheap pre-check so an unrelated update on a different section never causes a read
    if !lock_registry().subscribers.contains_key(section) {
        return;
    }

    let Ok(bytes) = std::fs::read(path_for(section)) else {
        // Mid-rename or removed; the follow-up event will carry the content.
        return;
    };
    let hash = hash_bytes(&bytes);

    // Take the callbacks and release the lock before invoking any of them —
    // a callback is free to call `save`, `subscribe`, or drop a Subscription.
    let callbacks = {
        let mut registry = lock_registry();

        // Echo: this is the file we just wrote ourselves.
        if last_written(section) == Some(hash) {
            registry.last_delivered.insert(section.to_owned(), hash);
            return;
        }
        // Duplicate event, or a write that produced identical bytes.
        if registry.last_delivered.get(section) == Some(&hash) {
            return;
        }
        registry.last_delivered.insert(section.to_owned(), hash);

        match registry.subscribers.get(section) {
            Some(subs) => subs
                .iter()
                .map(|(_, cb)| Arc::clone(cb))
                .collect::<Vec<_>>(),
            None => return,
        }
    };

    for callback in callbacks {
        callback(bytes.clone());
    }
}

/// A live registration. Dropping it unregisters the callback.
///
/// Returned by [`subscribe`] and [`subscribe_typed`]. It is `#[must_use]`
/// because a dropped `Subscription` stops firing immediately — binding it to
/// `_` is almost always a bug (bind to `_name` instead).
#[must_use = "dropping a Subscription unregisters the callback"]
#[derive(Debug)]
pub struct Subscription {
    section: String,
    id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut registry = lock_registry();
        if let Some(subs) = registry.subscribers.get_mut(&self.section) {
            subs.retain(|(id, _)| *id != self.id);
            if subs.is_empty() {
                registry.subscribers.remove(&self.section);
                registry.last_delivered.remove(&self.section);
            }
        }
    }
}

/// Call `callback` with the raw file contents whenever `<section>.ron` changes.
///
/// The callback runs on the watcher thread, so keep it short and do not block.
/// Multiple subscribers per section are fine; each gets its own
/// [`Subscription`] and they are invoked in registration order.
///
/// Writes made by this process through [`save`](crate::save) do not fire the
/// callback — see the [module docs](self).
pub fn subscribe(
    section: &str,
    callback: impl Fn(Vec<u8>) + Send + Sync + 'static,
) -> Subscription {
    // Force the watcher into existence on the first subscription.
    drop(
        WATCHER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    lock_registry()
        .subscribers
        .entry(section.to_owned())
        .or_default()
        .push((id, Arc::new(callback)));

    Subscription {
        section: section.to_owned(),
        id,
    }
}

/// [`subscribe`], but parsed into `T`.
///
/// Contents that fail to deserialize are dropped silently: an editor that
/// truncates before rewriting, or a user who has typed half a struct, must not
/// push a garbage value into a running app. The next successful write delivers
/// normally.
pub fn subscribe_typed<T, F>(section: &str, callback: F) -> Subscription
where
    T: DeserializeOwned + Send + 'static,
    F: Fn(T) + Send + Sync + 'static,
{
    subscribe(section, move |bytes| {
        let Ok(text) = std::str::from_utf8(&bytes) else {
            return;
        };
        if let Ok(value) = ron::from_str::<T>(text) {
            callback(value);
        }
    })
}
