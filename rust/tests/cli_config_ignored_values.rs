use serde_json::{Value, json};
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
            "proselint-rs-ignored-config-{}-{serial}",
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

fn assert_unused_value_is_ignored(config: &[u8]) {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("config.json"), config).expect("write config");
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"result":{"<stdin>":{"diagnostics":[]}}})
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn ignores_python_nan_in_unused_top_level_field() {
    assert_unused_value_is_ignored(br#"{"checks":{},"x":NaN}"#);
}

#[test]
fn ignores_python_overflow_float_in_unused_top_level_field() {
    assert_unused_value_is_ignored(br#"{"checks":{},"x":1e999}"#);
}

#[test]
fn ignores_lone_surrogate_in_unused_top_level_field() {
    assert_unused_value_is_ignored(br#"{"checks":{},"x":"\ud800"}"#);
}

#[test]
fn ignores_nested_special_values_in_unused_top_level_field() {
    assert_unused_value_is_ignored(br#"{"checks":{},"x":{"values":[NaN,1e999,"\ud800"]}}"#);
}

#[test]
fn recognizes_escaped_checks_key_while_ignoring_nan_field() {
    assert_unused_value_is_ignored(br#"{"\u0063hecks":{},"x":NaN}"#);
}
