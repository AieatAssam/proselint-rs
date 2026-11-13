#![cfg(unix)]

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn assert_signal_matches_proselint(signal: libc::c_int, name: &str) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .arg("check")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start CLI");
    let mut stdin = child.stdin.take().expect("keep stdin open");
    stdin.flush().unwrap();
    thread::sleep(Duration::from_millis(300));
    assert_eq!(unsafe { libc::kill(child.id() as libc::pid_t, signal) }, 0);

    let output = child.wait_with_output().expect("wait for signaled CLI");
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        format!("\nExiting (received {name} {signal}).\n").as_bytes()
    );
}

#[test]
fn sigint_exits_with_proselint_warning_and_status() {
    assert_signal_matches_proselint(libc::SIGINT, "SIGINT");
}

#[test]
fn sigterm_exits_with_proselint_warning_and_status() {
    assert_signal_matches_proselint(libc::SIGTERM, "SIGTERM");
}
