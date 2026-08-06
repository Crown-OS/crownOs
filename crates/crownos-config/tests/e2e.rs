use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use crownos_config::*;

/// How long to wait for a notification that should arrive.
const DELIVERED: Duration = Duration::from_secs(5);
/// How long to wait before concluding a notification will never arrive.
const SILENT: Duration = Duration::from_millis(400);

/// One test function, because `CROWN_CONFIG_DIR` is process-global and
/// cargo runs test functions on parallel threads.
#[test]
fn e2e() {
    let dir = setup_environment();

    check_paths(&dir);
    check_load_materialises_default(&dir);
    check_save_load_round_trip();
    check_save_records_its_own_hash();
    check_external_edit_breaks_the_hash();
    check_unparseable_file_is_not_clobbered();
    check_watcher();
    check_key_watcher();

    clear_environment(&dir);
}

fn setup_environment() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("crownos-config-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp config dir");
    // SAFETY: single-threaded section of a single test function; no other
    // test in this binary touches the environment.
    unsafe { std::env::set_var(CONFIG_DIR_ENV, &dir) };

    dir
}

fn clear_environment(dir: &Path) {
    unsafe { std::env::remove_var(CONFIG_DIR_ENV) };
    let _ = std::fs::remove_dir_all(dir);
}

/// `CROWN_CONFIG_DIR` wins over the platform config dir, and a section maps
/// to `<dir>/<section>.ron`.
fn check_paths(dir: &Path) {
    assert_eq!(config_dir(), dir);
    assert_eq!(path_for(Wifi::SECTION), dir.join("wifi.ron"));
}

/// A load of a missing file writes the default out.
fn check_load_materialises_default(dir: &Path) {
    let loaded: Wifi = load(Wifi::SECTION);
    assert_eq!(loaded, Wifi::default());
    assert!(
        dir.join("wifi.ron").exists(),
        "load() should materialise the default file"
    );
}

fn check_save_load_round_trip() {
    let appearance = Appearance {
        dark_mode: false,
        accent: AccentColor::Orange,
        transparency: 0.42,
        wallpaper: "/usr/share/backgrounds/crown.png".to_owned(),
        bar_height: 40,
    };
    save(Appearance::SECTION, &appearance).expect("save appearance");

    let round_tripped: Appearance = load(Appearance::SECTION);
    assert_eq!(round_tripped, appearance);
}

/// `save` records the hash of exactly what it put on disk, so a watcher
/// comparing on-disk bytes against it sees a match and stays quiet.
fn check_save_records_its_own_hash() {
    let on_disk = read(Appearance::SECTION);
    assert_eq!(
        last_written(Appearance::SECTION),
        Some(hash_bytes(&on_disk)),
        "save() must record the hash of the bytes it wrote"
    );
}

/// An external edit changes the bytes, so the recorded hash no longer matches
/// and the change is *not* suppressed.
fn check_external_edit_breaks_the_hash() {
    write(Appearance::SECTION, b"(dark_mode: true)");

    let on_disk = read(Appearance::SECTION);
    assert_ne!(
        last_written(Appearance::SECTION),
        Some(hash_bytes(&on_disk))
    );
}

/// An unparseable file falls back to the default without clobbering it.
fn check_unparseable_file_is_not_clobbered() {
    let garbage = b"this is not RON at all";
    write(Wifi::SECTION, garbage);

    let broken: Wifi = load(Wifi::SECTION);
    assert_eq!(broken, Wifi::default());
    assert_eq!(
        read(Wifi::SECTION),
        garbage,
        "a parse failure must not overwrite the user's file"
    );
}

/// The watcher delivers external edits, but not our own writes.
fn check_watcher() {
    let (tx, rx) = channel();
    let subscription = subscribe_typed::<Sound, _>(Sound::SECTION, move |sound| {
        let _ = tx.send(sound);
    });

    check_own_save_is_suppressed(&rx);
    check_partial_write_is_dropped(&rx);
    check_external_write_is_delivered(&rx);
    check_dropped_subscription_stops_delivering(subscription, &rx);
}

fn check_own_save_is_suppressed(rx: &Receiver<Sound>) {
    let mine = Sound {
        output_volume: 11.0,
        ..Sound::default()
    };
    save(Sound::SECTION, &mine).expect("save sound");

    assert!(
        rx.recv_timeout(SILENT).is_err(),
        "save() should not notify this process's own subscribers"
    );
}

/// A half-written file must not fire the typed callback.
fn check_partial_write_is_dropped(rx: &Receiver<Sound>) {
    write(Sound::SECTION, b"(output_volume: 9");

    assert!(
        rx.recv_timeout(SILENT).is_err(),
        "unparseable contents must not reach a typed subscriber"
    );
}

/// A write that did not go through `save` must be delivered.
fn check_external_write_is_delivered(rx: &Receiver<Sound>) {
    let theirs = Sound {
        output_volume: 77.0,
        ..Sound::default()
    };
    write(Sound::SECTION, to_ron(&theirs).as_bytes());

    let received = rx
        .recv_timeout(DELIVERED)
        .expect("watcher should deliver an external edit");
    assert_eq!(received, theirs);
}

fn check_dropped_subscription_stops_delivering(subscription: Subscription, rx: &Receiver<Sound>) {
    drop(subscription);
    write(Sound::SECTION, b"(output_volume: 1.0)");

    assert!(
        rx.recv_timeout(SILENT).is_err(),
        "a dropped Subscription must stop delivering"
    );
}

/// A key subscription fires for its own key only, not for its neighbours in
/// the same section.
fn check_key_watcher() {
    let (tx, rx) = channel();
    let subscription = subscribe_key(schema::appearance::BarHeight, move |bar_height| {
        let _ = tx.send(bar_height);
    });

    let mut appearance = Appearance {
        bar_height: 40,
        ..Appearance::default()
    };
    write(Appearance::SECTION, to_ron(&appearance).as_bytes());
    assert_eq!(
        rx.recv_timeout(DELIVERED)
            .expect("the first change should always be delivered"),
        40
    );

    // Same key, different section: a neighbour changed, so the file's bytes
    // changed, but this subscriber's value did not.
    appearance.transparency = 0.75;
    write(Appearance::SECTION, to_ron(&appearance).as_bytes());
    assert!(
        rx.recv_timeout(SILENT).is_err(),
        "a change to another key must not reach a key subscriber"
    );

    appearance.bar_height = 48;
    write(Appearance::SECTION, to_ron(&appearance).as_bytes());
    assert_eq!(
        rx.recv_timeout(DELIVERED)
            .expect("a change to the watched key should be delivered"),
        48
    );

    drop(subscription);
    appearance.bar_height = 24;
    write(Appearance::SECTION, to_ron(&appearance).as_bytes());
    assert!(
        rx.recv_timeout(SILENT).is_err(),
        "a dropped Subscription must stop delivering"
    );
}

/// Write a section's file directly, bypassing `save` — i.e. what another app
/// or `$EDITOR` would do.
fn write(section: &str, bytes: &[u8]) {
    std::fs::write(path_for(section), bytes).unwrap_or_else(|e| panic!("write {section}: {e}"));
}

fn read(section: &str) -> Vec<u8> {
    std::fs::read(path_for(section)).unwrap_or_else(|e| panic!("read {section}: {e}"))
}

fn to_ron<T: serde::Serialize>(value: &T) -> String {
    ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default()).expect("serialise")
}
