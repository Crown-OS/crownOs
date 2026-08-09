//! Type transcribed text into the focused window.
//!
//! Preference order:
//!   1. `wtype`   — wlr virtual-keyboard protocol, best unicode fidelity
//!   2. `ydotool` — uinput daemon, works everywhere ydotoold runs
//!   3. `wl-copy` — clipboard fallback (with a desktop notification)

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};

fn run_with_stdin(cmd: &str, args: &[&str], text: &str) -> std::io::Result<bool> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(text.as_bytes())?;
    Ok(child.wait()?.success())
}

pub fn type_text(text: &str) -> Result<()> {
    // wtype reads from stdin with `-`; ydotool with `--file -`.
    match run_with_stdin("wtype", &["-"], text) {
        Ok(true) => return Ok(()),
        Ok(false) => log::warn!("inject: wtype exited nonzero, trying ydotool"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("inject: wtype failed ({e}), trying ydotool"),
    }
    match run_with_stdin("ydotool", &["type", "--file", "-"], text) {
        Ok(true) => return Ok(()),
        Ok(false) => log::warn!("inject: ydotool exited nonzero (is ydotoold running?)"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("inject: ydotool failed ({e})"),
    }
    if run_with_stdin("wl-copy", &[], text).unwrap_or(false) {
        let _ = Command::new("notify-send")
            .args(["Crown Dictator", "Typing tools unavailable — text copied to clipboard"])
            .status();
        return Ok(());
    }
    Err(anyhow!(
        "no way to deliver text: install wtype or ydotool (with ydotoold), or wl-clipboard"
    ))
}
