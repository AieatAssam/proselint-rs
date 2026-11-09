use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "proselint-rs-xdg-missing-{}-{serial}",
            std::process::id()
        ));
        let home = root.join("home");
        fs::create_dir_all(home.join(".config/proselint")).unwrap();
        fs::write(
            home.join(".config/proselint/config.json"),
            r#"{"max_errors":7,"checks":{"typography":{"symbols":{"curly_quotes":true}}}}"#,
        )
        .unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str], input: &[u8]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
        command
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("xdg-missing"))
            .args(args);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn check_does_not_load_home_config_when_nonempty_xdg_file_is_missing() {
    let output = Workspace::new().run(
        &["check", "--output-format", "json"],
        b"The phrase \"word\" is quoted.",
    );
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        value["result"]["<stdin>"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn dump_config_uses_default_when_nonempty_xdg_file_is_missing() {
    let output = Workspace::new().run(&["dump-config"], b"");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["max_errors"], 1000, "home config was loaded: {value}");
    assert_eq!(value["checks"]["typography"], false);
}
