use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-coordinates-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"annotations":{"misc":true}}}"#,
        )
        .expect("write config selecting only annotations.misc");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_stdin(input: &[u8]) -> Output {
    let workspace = Workspace::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native CLI");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input)
        .expect("write input");
    child.wait_with_output().expect("wait for native CLI")
}

fn run_file(path: &str, input: &[u8]) -> (Output, String) {
    let workspace = Workspace::new();
    fs::write(workspace.path().join(path), input).expect("write source file");
    let uri = proselint_rs::file_uri(&workspace.path().join(path)).expect("source URI");
    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
            path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI");
    (output, uri)
}

fn diagnostic(span: [usize; 2], pos: [usize; 2]) -> Value {
    json!({
        "check_path": "annotations.misc",
        "message": "Annotation left in text.",
        "pos": pos,
        "replacements": null,
        "span": span,
    })
}

fn assert_stdin_result(input: &[u8], expected_exit: i32, diagnostics: Vec<Value>) {
    let output = run_stdin(input);
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "stderr: {:?}",
        output.stderr
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": diagnostics}}})
    );
}

#[test]
fn reports_multiline_matches_with_final_lf() {
    assert_stdin_result(
        b"first\nTODO\nsecond\nTODO\n",
        1,
        vec![diagnostic([7, 11], [2, 1]), diagnostic([19, 23], [4, 1])],
    );
}

#[test]
fn reports_multiline_matches_without_final_lf() {
    assert_stdin_result(
        b"first\nTODO\nsecond\nTODO",
        1,
        vec![diagnostic([7, 11], [2, 1]), diagnostic([19, 23], [4, 1])],
    );
}

#[test]
fn crlf_stdin_offsets_keep_both_newline_codepoints() {
    assert_stdin_result(b"first\r\nTODO", 1, vec![diagnostic([8, 12], [2, 1])]);
}

#[test]
fn unicode_prefix_uses_codepoint_columns() {
    assert_stdin_result(
        "é e\u{301} TODO".as_bytes(),
        1,
        vec![diagnostic([6, 10], [1, 6])],
    );
}

#[test]
fn boundary_control_text_has_no_findings() {
    assert_stdin_result(b"TODO1 TODO_foo TODOx xTODO", 0, vec![]);
}

#[test]
fn file_crlf_is_universal_newline_normalized() {
    let (output, uri) = run_file("chapter é.txt", b"first\r\nTODO");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {uri: {"diagnostics": [diagnostic([7, 11], [2, 1])]}}})
    );
}
