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
            "proselint-rs-directory-config-{}-{serial}",
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

#[test]
fn directory_config_matches_pinned_oserror_json() {
    assert_directory_config(".");
}

#[test]
fn parent_directory_config_matches_pinned_oserror_json() {
    let workspace = Workspace::new();
    fs::create_dir_all(workspace.0.join("child")).expect("create child directory");
    fs::write(workspace.0.join("child/probe.txt"), b"x").expect("write nested fixture");
    assert_directory_config_in(workspace, "child/..");
}

#[test]
fn full_directory_config_reports_canonical_oserror_on_stderr() {
    assert_directory_config_stderr("full");
}

#[test]
fn compact_directory_config_reports_canonical_oserror_on_stderr() {
    assert_directory_config_stderr("compact");
}

#[cfg(unix)]
#[test]
fn symlink_directory_config_uses_canonical_target_path() {
    let workspace = Workspace::new();
    let target = workspace.0.join("child");
    fs::create_dir(&target).expect("create target directory");
    let link = workspace.0.join("directory-link");
    std::os::unix::fs::symlink(&target, &link).expect("create directory symlink");
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "directory-link",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "error": {
                "code": -31999,
                "data": "OSError",
                "message": format!(
                    "[Errno Directories not permitted - found directory %s] {}",
                    target.display()
                )
            }
        })
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

fn assert_directory_config(config: &str) {
    let workspace = Workspace::new();
    assert_directory_config_in(workspace, config);
}

fn assert_directory_config_in(workspace: Workspace, config: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args(["check", "--config", config, "--output-format", "json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "error": {
                "code": -31999,
                "data": "OSError",
                "message": format!(
                    "[Errno Directories not permitted - found directory %s] {}",
                    workspace.0.canonicalize().unwrap().display()
                )
            }
        })
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

fn assert_directory_config_stderr(output_format: &str) {
    let workspace = Workspace::new();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args(["check", "--config", ".", "--output-format", output_format])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(
        output.stderr,
        format!(
            "Unrecoverable error encountered while running proselint: [Errno Directories not permitted - found directory %s] {}\n",
            workspace.0.canonicalize().unwrap().display()
        )
        .as_bytes()
    );
}
