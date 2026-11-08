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
            "proselint-rs-version-precedence-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        let home = path.join("home");
        fs::create_dir_all(home.join("xdg")).expect("create isolated home and XDG config");
        Self(path)
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_proselint"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("home/xdg"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn root_version_before_check_is_overridden_by_check_default() {
    let output = Workspace::new().run(&["--version", "check"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn root_version_before_check_json_runs_normal_empty_stdin_check() {
    let output = Workspace::new().run(&["--version", "check", "--output-format", "json"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("normal JSON check output");
    assert_eq!(
        result,
        serde_json::json!({"result": {"<stdin>": {"diagnostics": []}}})
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn root_output_format_before_check_uses_check_subparser_default() {
    let output = Workspace::new().run(&["--output-format", "json", "--version", "check"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn root_output_format_followed_by_version_reports_missing_value() {
    for flag in ["--output-format", "-o"] {
        let output = Workspace::new().run(&[flag, "--version", "check"]);

        assert_eq!(output.status.code(), Some(2), "stderr: {:?}", output.stderr);
        assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
        assert_eq!(
            output.stderr,
            b"usage: proselint [options] <command>\nproselint: error: argument --output-format/-o: expected one argument\n"
        );
    }
}

#[test]
fn root_version_before_check_missing_path_runs_check_behavior() {
    let workspace = Workspace::new();
    let output = workspace.run(&["--version", "check", "--", "missing.txt"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error encountered while linting"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("missing.txt"), "stderr: {stderr}");
}

#[test]
fn check_version_flag_remains_a_version_request() {
    let output = Workspace::new().run(&["check", "--version"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, b"Proselint 0.16.0\n");
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
