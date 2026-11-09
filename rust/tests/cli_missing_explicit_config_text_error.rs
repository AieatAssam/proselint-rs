use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-missing-config-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated config workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_explicit_config_default_full_format_reports_canonical_error_on_stderr() {
    assert_missing_config_stderr(false);
}

#[test]
fn missing_explicit_config_compact_format_reports_canonical_error_on_stderr() {
    assert_missing_config_stderr(true);
}

fn assert_missing_config_stderr(compact: bool) {
    let workspace = Workspace::new();
    let home = workspace.0.join("home");
    let xdg = workspace.0.join("xdg");
    fs::create_dir_all(&home).expect("create isolated home");
    fs::create_dir_all(&xdg).expect("create isolated XDG config");
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .current_dir(&workspace.0)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &xdg)
        .args(["check", "--config", "missing config.json"]);
    if compact {
        command.args(["--output-format", "compact"]);
    }
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(
        output.stderr,
        b"Unrecoverable error encountered while running proselint: Configuration file 'missing config.json' could not be read.\n"
    );
}
