use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(cwd_config: &str, explicit_config: Option<&str>) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-cwd-config-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated workspace");
        fs::write(path.join("proselint.json"), cwd_config).expect("write cwd config");
        if let Some(config) = explicit_config {
            fs::write(path.join("explicit.json"), config).expect("write explicit config");
        }
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(cwd_config: &str, explicit_config: Option<&str>) -> std::process::Output {
    let workspace = Workspace::new(cwd_config, explicit_config);
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint"));
    command
        .current_dir(&workspace.0)
        .args(["check", "--output-format", "json"]);
    if explicit_config.is_some() {
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

fn assert_json(output: &std::process::Output, status: i32, expected: Value) {
    assert_eq!(
        output.status.code(),
        Some(status),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

#[test]
fn omitted_config_discovers_cwd_finding_config() {
    let output = run(
        r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#,
        None,
    );
    assert_json(
        &output,
        1,
        json!({"result":{"<stdin>":{"diagnostics":[{
            "check_path":"misc.apologizing",
            "message":"Excessive apologizing.",
            "pos":[1,20],
            "replacements":null,
            "span":[20,43]
        }]}}}),
    );
}

#[test]
fn omitted_config_discovers_empty_cwd_checks() {
    let output = run(r#"{"checks":{}}"#, None);
    assert_json(&output, 0, json!({"result":{"<stdin>":{"diagnostics":[]}}}));
}

#[test]
fn explicit_config_takes_precedence_over_conflicting_cwd_config() {
    let output = run(
        r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#,
        Some(r#"{"checks":{}}"#),
    );
    assert_json(&output, 0, json!({"result":{"<stdin>":{"diagnostics":[]}}}));
}
