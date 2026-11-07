use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const PINNED_HELP: &[u8] = include_bytes!("fixtures/top_level_help.bin");

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("proselint-rs-help-{}-{serial}", std::process::id()));
        fs::create_dir(&path).expect("create isolated help workspace");
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated HOME and XDG config");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn long_help_matches_pinned_bytes() {
    assert_help(&["--help"], 0);
}

#[test]
fn short_help_matches_pinned_bytes() {
    assert_help(&["-h"], 0);
}

#[test]
fn no_arguments_prints_help_then_returns_usage_status() {
    assert_help(&[], 2);
}

fn assert_help(args: &[&str], status: i32) {
    let workspace = Workspace::new();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .env("HOME", workspace.0.join("home"))
        .env("XDG_CONFIG_HOME", workspace.0.join("home/xdg"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI");

    assert_eq!(output.status.code(), Some(status));
    assert_eq!(output.stdout, PINNED_HELP);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
