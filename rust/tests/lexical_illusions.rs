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
            "proselint-rs-lexical-illusions-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"lexical_illusions":true}}}}"#),
        )
        .expect("write config selecting lexical_illusions");
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
        .expect("write stdin");
    child.wait_with_output().expect("wait for CLI")
}

fn diagnostic(start: usize, matched: &str) -> Value {
    json!({
        "check_path": "lexical_illusions",
        "message": format!("There's a lexical illusion in '{matched}' - a phrase is repeated."),
        "pos": [1, start],
        "replacements": null,
        "span": [start, start + matched.chars().count()],
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

#[test]
fn reports_repeated_words_and_phrases_up_to_four_words() {
    assert_result(
        "red red; a b a b; a b c a b c; a b c d a b c d",
        1000,
        vec![
            diagnostic(1, "red red"),
            diagnostic(10, "a b a b"),
            diagnostic(19, "a b c a b c"),
            diagnostic(32, "a b c d a b c d"),
        ],
    );
}

#[test]
fn exceptions_apply_to_exact_matches_and_longer_repetitions_still_report() {
    assert_result("had had; that that", 1000, vec![]);
    assert_result(
        "had had had; that that that",
        1000,
        vec![
            diagnostic(1, "had had had"),
            diagnostic(14, "that that that"),
        ],
    );
}

#[test]
fn repeated_phrases_require_the_same_captured_whitespace() {
    assert_result("a  b\ta  b", 1000, vec![diagnostic(1, "a  b\ta  b")]);
    assert_result("a b a  b", 1000, vec![]);
}

#[test]
fn respects_strict_boundaries_and_unicode_casefolding() {
    assert_result("don't don't; well-known well-known", 1000, vec![]);
    assert_result(
        "ſ s; K k; İ i",
        1000,
        vec![diagnostic(6, "K k"), diagnostic(11, "İ i")],
    );
}

#[test]
fn quote_suppression_unicode_offsets_and_error_limit_match_the_cli() {
    assert_result("She wrote “word word” and said 'that that'.", 1000, vec![]);
    assert_result("x é😊 word word", 1000, vec![diagnostic(6, "word word")]);
    assert_result(
        "one one; two two; three three",
        1,
        vec![diagnostic(1, "one one")],
    );
}
