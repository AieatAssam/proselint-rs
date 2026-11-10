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
            "proselint-rs-surrogate-config-{}-{serial}",
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

fn run_cli(config: &[u8]) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("config.json"), config).expect("write config");
    Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
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
        .expect("run CLI")
}

fn assert_clean(config: &[u8]) {
    let output = run_cli(config);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"result": {"<stdin>": {"diagnostics": []}}})
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn false_check_with_high_lone_surrogate_key_is_ignored() {
    assert_clean(br#"{"checks":{"annotations":{"\ud800":false}}}"#);
}

#[test]
fn false_check_with_low_lone_surrogate_key_is_ignored() {
    assert_clean(br#"{"checks":{"annotations":{"\udfff":false}}}"#);
}

#[test]
fn high_lone_surrogate_duplicate_key_uses_last_false_value() {
    assert_clean(br#"{"checks":{"annotations":{"\ud800":"invalid","\uD800":false}}}"#);
}

#[test]
fn false_then_invalid_duplicate_key_matches_python_attribute_error() {
    let output = run_cli(br#"{"checks":{"annotations":{"\ud800":false,"\uD800":"invalid"}}}"#);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "error": {
                "code": -31999,
                "data": "AttributeError",
                "message": "'str' object has no attribute 'items'"
            }
        })
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn escaped_surrogate_pair_and_literal_unicode_duplicate_key_uses_last_false() {
    assert_clean(r#"{"checks":{"annotations":{"\ud83d\ude00":"invalid","😀":false}}}"#.as_bytes());
}
