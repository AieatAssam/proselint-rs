use proselint_rs::file_uri;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-multi-file-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("config.json"),
            br#"{"max_errors":1,"checks":{"misc":{"apologizing":true}}}"#,
        )
        .expect("write config with one error per file");
        std::fs::write(path.join("z.txt"), two_findings()).expect("write z source");
        std::fs::write(path.join("a.txt"), two_findings()).expect("write a source");
        Self(path.canonicalize().expect("canonicalize test workspace"))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn uri(&self, file: &str) -> String {
        file_uri(&self.path().join(file)).expect("source file URI")
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

fn run(workspace: &Workspace, format: &str, paths: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            format,
        ])
        .args(paths)
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
fn full_output_keeps_argv_order_and_resets_max_errors_per_file() {
    let workspace = Workspace::new();
    let output = run(&workspace, "full", &["z.txt", "a.txt"]);
    let expected = format!(
        "{}:1:20: misc.apologizing: Excessive apologizing.\n{}:1:20: misc.apologizing: Excessive apologizing.\n",
        workspace.path().join("z.txt").display(),
        workspace.path().join("a.txt").display(),
    );
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn compact_output_keeps_argv_order_and_resets_max_errors_per_file() {
    let workspace = Workspace::new();
    let output = run(&workspace, "compact", &["z.txt", "a.txt"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"z.txt:1:20: misc.apologizing: Excessive apologizing.\na.txt:1:20: misc.apologizing: Excessive apologizing.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn json_results_include_each_file_with_one_finding() {
    let workspace = Workspace::new();
    let output = run(&workspace, "json", &["z.txt", "a.txt"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({
            "result": {
                workspace.uri("z.txt"): {"diagnostics": [diagnostic()]},
                workspace.uri("a.txt"): {"diagnostics": [diagnostic()]},
            }
        })
    );
}

#[test]
fn json_continues_after_a_missing_file_and_keeps_the_finding() {
    let workspace = Workspace::new();
    let output = run(&workspace, "json", &["missing.txt", "a.txt"]);
    let missing = workspace.path().join("missing.txt");
    let missing_uri = format!("file://{}", missing.display());
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({
            "result": {
                missing_uri: {
                    "error": {
                        "code": -31998,
                        "message": format!(
                            "[Errno 2] No such file or directory: '{}'",
                            missing.display()
                        ),
                    }
                },
                workspace.uri("a.txt"): {"diagnostics": [diagnostic()]},
            }
        })
    );
}

#[test]
fn full_output_uses_resolved_path_for_missing_file_and_continues() {
    let workspace = Workspace::new();
    let output = run(&workspace, "full", &["missing.txt", "a.txt"]);
    let missing = workspace.path().join("missing.txt");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stderr,
        format!(
            "Error encountered while linting {}: [Errno 2] No such file or directory: '{}'\n",
            missing.display(),
            missing.display()
        )
        .as_bytes()
    );
    assert_eq!(
        output.stdout,
        format!(
            "{}:1:20: misc.apologizing: Excessive apologizing.\n",
            workspace.path().join("a.txt").display()
        )
        .as_bytes()
    );
}

#[cfg(unix)]
#[test]
fn json_continues_after_a_permission_denied_file() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = Workspace::new();
    let denied = workspace.path().join("denied.txt");
    std::fs::write(&denied, two_findings()).expect("write denied source");
    std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000))
        .expect("make source unreadable");

    let output = run(&workspace, "json", &["denied.txt", "a.txt"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({
            "result": {
                workspace.uri("denied.txt"): {
                    "error": {
                        "code": -31998,
                        "message": format!("[Errno 13] Permission denied: '{}'", denied.display()),
                    }
                },
                workspace.uri("a.txt"): {"diagnostics": [diagnostic()]},
            }
        })
    );
}
