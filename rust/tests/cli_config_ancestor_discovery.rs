use serde_json::{Value, json};
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
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-ancestor-config-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("child/grandchild")).expect("create isolated workspace");
        Self(path)
    }

    fn cwd(&self) -> PathBuf {
        self.0.join("child/grandchild")
    }

    fn run(&self, explicit: bool) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_proselint"));
        command
            .current_dir(self.cwd())
            .args(["check", "--output-format", "json"]);
        if explicit {
            command.args(["--config", "explicit.json"]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run native CLI");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"Our policy states: More research is needed.")
            .unwrap();
        child.wait_with_output().expect("wait for native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn disabled() -> &'static str {
    r#"{"checks":{}}"#
}

fn apologizing() -> &'static str {
    r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#
}

fn assert_success(output: &std::process::Output, expected: Value) {
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

#[test]
fn nested_cwd_discovers_ancestor_config() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("proselint.json"), apologizing()).unwrap();
    let output = workspace.run(false);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["result"]["<stdin>"]["diagnostics"][0]["check_path"],
        "misc.apologizing"
    );
}

#[test]
fn nearest_ancestor_config_wins() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("proselint.json"), apologizing()).unwrap();
    fs::write(workspace.0.join("child/proselint.json"), disabled()).unwrap();
    assert_success(
        &workspace.run(false),
        json!({"result":{"<stdin>":{"diagnostics":[]}}}),
    );
}

#[test]
fn explicit_config_overrides_ancestors() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("proselint.json"), apologizing()).unwrap();
    fs::write(workspace.cwd().join("explicit.json"), disabled()).unwrap();
    assert_success(
        &workspace.run(true),
        json!({"result":{"<stdin>":{"diagnostics":[]}}}),
    );
}

#[test]
fn malformed_nearest_ancestor_errors_without_falling_through() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("proselint.json"), disabled()).unwrap();
    fs::write(workspace.0.join("child/proselint.json"), "{").unwrap();
    let output = workspace.run(false);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["error"]["data"],
        "JSONDecodeError"
    );
}

#[test]
fn directory_named_config_is_skipped_for_ancestor_file() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("proselint.json"), apologizing()).unwrap();
    fs::create_dir(workspace.0.join("child/proselint.json")).unwrap();
    let output = workspace.run(false);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["result"]["<stdin>"]["diagnostics"][0]["check_path"],
        "misc.apologizing"
    );
}
