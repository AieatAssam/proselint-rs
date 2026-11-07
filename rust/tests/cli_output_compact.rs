use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-output-compact-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::write(
            path.join("config.json"),
            r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#,
        )
        .expect("write explicit config");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(workspace: &Workspace, input: Option<&[u8]>, file: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint"));
    command
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "compact",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(file) = file {
        command.arg(file);
    }
    if input.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn().expect("run native CLI");
    if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input)
            .unwrap();
    }
    child.wait_with_output().expect("wait for native CLI")
}

#[test]
fn clean_empty_stdin_has_empty_streams_and_success_status() {
    let output = run(&Workspace::new(), Some(b""), None);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn finding_stdin_uses_compact_byte_format_and_finding_status() {
    let output = run(
        &Workspace::new(),
        Some(b"Our policy states: More research is needed."),
        None,
    );
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:20: misc.apologizing: Excessive apologizing.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn nested_file_finding_uses_basename_as_source() {
    let workspace = Workspace::new();
    fs::create_dir(workspace.0.join("nested")).unwrap();
    fs::write(
        workspace.0.join("nested/draft.txt"),
        b"Our policy states: More research is needed.",
    )
    .unwrap();
    let output = run(&workspace, None, Some("nested/draft.txt"));
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"draft.txt:1:20: misc.apologizing: Excessive apologizing.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn invalid_utf8_file_is_reported_as_a_per_file_compact_error() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("broken.txt"), [0xff]).unwrap();
    let output = run(&workspace, None, Some("broken.txt"));
    assert_eq!(output.status.code(), Some(0), "stdout: {:?}", output.stdout);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(
        output.stderr,
        b"Error encountered while linting broken.txt: 'utf-8' codec can't decode byte 0xff in position 0: invalid start byte\n"
    );
}
