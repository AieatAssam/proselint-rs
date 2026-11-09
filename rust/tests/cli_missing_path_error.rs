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
            "proselint-rs-missing-path-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::create_dir(path.join("home")).expect("create isolated HOME");
        fs::create_dir(path.join("xdg")).expect("create isolated XDG_CONFIG_HOME");
        Self(path.canonicalize().expect("canonicalize test workspace"))
    }

    fn missing_path(&self) -> PathBuf {
        self.0.join("--help")
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
        .env("HOME", workspace.0.join("home"))
        .env("XDG_CONFIG_HOME", workspace.0.join("xdg"))
        .args(["check", "--output-format", format, "--", "--help"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI")
}

fn expected_error(path: &std::path::Path) -> String {
    format!("[Errno 2] No such file or directory: '{}'", path.display())
}

#[test]
fn missing_path_after_double_dash_uses_full_stderr_format() {
    let workspace = Workspace::new();
    let output = run(&workspace, "full");
    let path = workspace.missing_path();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(
        output.stderr,
        format!(
            "Error encountered while linting {}: {}\n",
            path.display(),
            expected_error(&path)
        )
        .as_bytes()
    );
}

#[test]
fn missing_path_after_double_dash_uses_compact_stderr_format() {
    let workspace = Workspace::new();
    let output = run(&workspace, "compact");
    let path = workspace.missing_path();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(
        output.stderr,
        format!(
            "Error encountered while linting --help: {}\n",
            expected_error(&path)
        )
        .as_bytes()
    );
}

#[test]
fn missing_path_after_double_dash_uses_json_file_error_shape() {
    let workspace = Workspace::new();
    let output = run(&workspace, "json");
    let path = workspace.missing_path();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    let result = value["result"].as_object().expect("result object");
    assert_eq!(result.len(), 1);
    let source_uri = format!("file://{}", path.display());
    let file_error = result.get(&source_uri).expect("file URI result key");
    assert_eq!(file_error["error"]["code"], -31998);
    assert_eq!(file_error["error"]["message"], expected_error(&path));
    assert!(file_error["error"].get("data").is_none());
}
