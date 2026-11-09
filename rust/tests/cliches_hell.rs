use serde_json::{Value, json};
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
            "proselint-rs-cliches-hell-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"cliches":{"hell":true}}}"#,
        )
        .expect("write config selecting only cliches.hell");
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

fn run_check(input: &str) -> Output {
    let workspace = Workspace::new();
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
        "check_path": "cliches.hell",
        "message": "Never use the phrase 'all hell broke loose'.",
        "pos": [1, column],
        "replacements": null,
        "span": span,
    })
}

fn assert_result(input: &str, expected_exit: i32, diagnostics: Vec<Value>) {
    let output = run_check(input);
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": diagnostics}}})
    );
}

#[test]
fn detects_the_phrase_in_surrounding_prose() {
    assert_result(
        "Then all hell broke loose again.",
        1,
        vec![diagnostic([6, 26], 6)],
    );
}

#[test]
fn matching_is_case_insensitive_and_keeps_match_order() {
    assert_result(
        "ALL HELL BROKE LOOSE; All Hell Broke Loose; all hell broke loose.",
        1,
        vec![
            diagnostic([1, 21], 1),
            diagnostic([23, 43], 23),
            diagnostic([45, 65], 45),
        ],
    );
}

#[test]
fn altered_separators_and_joined_first_word_do_not_match() {
    assert_result(
        "all hell-broke loose; all hell  broke loose; allhell broke loose.",
        0,
        vec![],
    );
}

#[test]
fn quoted_phrase_is_suppressed() {
    assert_result("\"all hell broke loose\"", 0, vec![]);
}

#[test]
fn matching_is_an_unanchored_raw_substring() {
    assert_result("small hell broke loosely", 1, vec![diagnostic([3, 23], 3)]);
}

#[test]
fn selected_config_does_not_enable_the_annotation_rule() {
    assert_result(
        "All hell broke loose and TODO.",
        1,
        vec![diagnostic([1, 21], 1)],
    );
}

#[test]
fn unicode_casefold_window_handles_kelvin_sign() {
    assert_result("all hell broKe loose", 1, vec![diagnostic([1, 21], 1)]);
}
