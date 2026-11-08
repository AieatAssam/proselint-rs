use proselint_rs::file_uri;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-directory-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir_all(path.join("nested/deeper")).expect("create test tree");
        std::fs::write(
            path.join("config.json"),
            br#"{"max_errors":1,"checks":{"misc":{"apologizing":true}}}"#,
        )
        .expect("write config");
        std::fs::write(path.join("nested/first.txt"), two_findings()).unwrap();
        std::fs::write(path.join("nested/deeper/second.md"), two_findings()).unwrap();
        std::fs::write(path.join("nested/ignored.rs"), two_findings()).unwrap();
        Self(path.canonicalize().expect("canonicalize test workspace"))
    }

    fn uri(&self, relative: &str) -> String {
        file_uri(&self.0.join(relative)).expect("source file URI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn two_findings() -> &'static [u8] {
    b"Our policy states: More research is needed. Our policy states: More research is needed."
}

fn run(workspace: &Workspace, path: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
            path,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI")
}

fn diagnostic() -> Value {
    json!({
        "check_path": "misc.apologizing",
        "message": "Excessive apologizing.",
        "pos": [1, 20],
        "replacements": null,
        "span": [20, 43],
    })
}

#[test]
fn directory_recursively_lints_accepted_extensions_with_per_file_limits() {
    let workspace = Workspace::new();
    let output = run(&workspace, "nested");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {
            workspace.uri("nested/first.txt"): {"diagnostics": [diagnostic()]},
            workspace.uri("nested/deeper/second.md"): {"diagnostics": [diagnostic()]},
        }})
    );
}
