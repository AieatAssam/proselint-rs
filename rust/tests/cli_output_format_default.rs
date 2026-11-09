use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(config: &str) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-output-full-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated workspace");
        fs::write(path.join("config.json"), config).expect("write explicit config");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(config: &str, input: &[u8], explicit_full: bool) -> std::process::Output {
    let workspace = Workspace::new(config);
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .current_dir(&workspace.0)
        .args(["check", "--config", "config.json"]);
    if explicit_full {
        command.args(["--output-format", "full"]);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run native CLI without --output-format");
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().expect("wait for native CLI")
}

#[test]
fn omitted_output_format_is_full_for_clean_result() {
    let output = run(r#"{"checks":{}}"#, b"hello world", false);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn omitted_output_format_is_full_for_findings() {
    assert_full_finding(false);
}

#[test]
fn explicit_full_output_matches_the_default_format() {
    assert_full_finding(true);
}

fn assert_full_finding(explicit_full: bool) {
    let output = run(
        r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#,
        b"Our policy states: More research is needed.",
        explicit_full,
    );
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:20: misc.apologizing: Excessive apologizing.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
