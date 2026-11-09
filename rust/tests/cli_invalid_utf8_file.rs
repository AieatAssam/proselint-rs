use proselint_rs::file_uri;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-invalid-utf8-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create workspace");
        fs::write(path.join("config.json"), br#"{"checks":{}}"#).expect("write config");
        fs::write(path.join("broken.txt"), [0xff]).expect("write invalid UTF-8");
        Self(path.canonicalize().expect("canonicalize workspace"))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(workspace: &Workspace, format: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            format,
            "broken.txt",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI")
}

#[test]
fn full_mode_reports_invalid_utf8_as_a_per_file_lint_error() {
    let workspace = Workspace::new();
    let output = run(&workspace, "full");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        format!(
            "Error encountered while linting {}: 'utf-8' codec can't decode byte 0xff in position 0: invalid start byte\n",
            workspace.0.join("broken.txt").display()
        )
        .as_bytes()
    );
}

#[test]
fn json_mode_reports_invalid_utf8_as_a_structured_per_file_error() {
    let workspace = Workspace::new();
    let output = run(&workspace, "json");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).expect("JSON output"),
        json!({
            "result": {
                file_uri(&workspace.0.join("broken.txt")).expect("file URI"): {
                    "error": {
                        "code": -31999,
                        "data": "UnicodeDecodeError",
                        "message": "'utf-8' codec can't decode byte 0xff in position 0: invalid start byte"
                    }
                }
            }
        })
    );
}
