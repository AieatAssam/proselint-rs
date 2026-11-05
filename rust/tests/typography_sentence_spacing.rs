use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "More than two spaces after the period; use 1 or 2.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-sentence-spacing-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"typography":{"symbols":{"sentence_spacing":true}}}}"#,
        )
        .expect("write config selecting only typography.symbols.sentence_spacing");
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
        .write_all(input.as_bytes())
        .expect("write input");
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "typography.symbols.sentence_spacing",
        "message": MESSAGE,
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
fn reports_period_followed_by_three_spaces() {
    assert_result("A sentence.   Next.", 1, vec![diagnostic([11, 15], 11)]);
}

#[test]
fn four_spaces_match_only_period_and_first_three_spaces() {
    assert_result("A sentence.    Next.", 1, vec![diagnostic([11, 15], 11)]);
}

#[test]
fn reports_multiple_matches_in_source_order() {
    assert_result(
        "One.   Two.   Three.",
        1,
        vec![diagnostic([4, 8], 4), diagnostic([11, 15], 11)],
    );
}

#[test]
fn two_spaces_and_other_punctuation_are_clean() {
    assert_result("One.  Two!   Three?    Four", 0, vec![]);
}

#[test]
fn suppresses_match_inside_quotes() {
    assert_result("Quoted: \"A sentence.   Next.\"", 0, vec![]);
}

#[test]
fn positions_follow_unicode_codepoints() {
    assert_result(
        "é.   Next.   Fine.",
        1,
        vec![diagnostic([2, 6], 2), diagnostic([10, 14], 10)],
    );
}

#[test]
fn selected_config_does_not_enable_other_rules() {
    assert_result("TODO and very.   Next.", 1, vec![diagnostic([14, 18], 14)]);
}
