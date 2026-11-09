use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(max_errors: usize) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-narcissism-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"narcissism":true}}}}}}"#),
        )
        .expect("write config selecting misc.narcissism");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(input: &str, max_errors: usize) -> Output {
    let workspace = Workspace::new(max_errors);
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
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
        .write_all(input.as_bytes())
        .expect("write input");
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "misc.narcissism",
        "message": "Professional narcissism. Talk about the subject, not its study.",
        "pos": [1, column],
        "replacements": null,
        "span": span,
    })
}

fn assert_result(input: &str, max_errors: usize, expected: Vec<Value>) {
    let output = run(input, max_errors);
    assert_eq!(
        output.status.code(),
        Some(if expected.is_empty() { 0 } else { 1 }),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": expected}}})
    );
}

const FIRST: &str = "In recent years, an increasing number of scientists have appeared.";
const SECOND: &str = "In recent years, an increasing number of artists have appeared.";

#[test]
fn finds_scientist_phrase_case_insensitively() {
    assert_result(FIRST, 1000, vec![diagnostic([1, 57], 1)]);
    assert_result(
        "IN RECENT YEARS, AN INCREASING NUMBER OF Scientists HAVE appeared.",
        1000,
        vec![diagnostic([1, 57], 1)],
    );
}

#[test]
fn rejects_phrase_variants_and_suppresses_quotes() {
    assert_result(
        "In recent years, an increasing number of AI scientists have appeared.",
        1000,
        vec![],
    );
    assert_result(
        "In recent years, an increasing number of scientists had appeared.",
        1000,
        vec![],
    );
    assert_result(
        "“In recent years, an increasing number of scientists have” and In recent years, an increasing number of artists have appeared.",
        1000,
        vec![diagnostic([64, 117], 64)],
    );
}

#[test]
fn reports_unanchored_matches_and_unicode_positions() {
    assert_result(
        "xIn recent years, an increasing number of scientists havey",
        1000,
        vec![diagnostic([2, 58], 2)],
    );
    assert_result(
        "é In recent years, an increasing number of scientists have appeared.",
        1000,
        vec![diagnostic([3, 59], 3)],
    );
}

#[test]
fn max_errors_limits_repeated_matches() {
    assert_result(FIRST, 1, vec![diagnostic([1, 57], 1)]);
    assert_result(
        &format!("{FIRST} {SECOND}"),
        1,
        vec![diagnostic([1, 57], 1)],
    );
    assert_result(
        &format!("{FIRST} {SECOND}"),
        2,
        vec![diagnostic([1, 57], 1), diagnostic([68, 121], 68)],
    );
}
