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
            "proselint-rs-hedging-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"hedging":true}}"#,
        )
        .expect("write config selecting only hedging");
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
        "check_path": "hedging",
        "message": "Hedging. Just say it.",
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
fn detects_i_would_argue_that() {
    assert_result(
        "I would argue that this is a good test.",
        1,
        vec![diagnostic([1, 19], 1)],
    );
}

#[test]
fn detects_comma_phrase_after_a_word() {
    assert_result(
        "You could say that, so to speak.",
        1,
        vec![diagnostic([19, 32], 19)],
    );
}

#[test]
fn comma_phrase_requires_a_preceding_word_boundary() {
    assert_result(", so to speak; x , so to speak;", 0, vec![]);
}

#[test]
fn detects_to_a_certain_degree() {
    assert_result(
        "This is true to a certain degree.",
        1,
        vec![diagnostic([14, 33], 14)],
    );
}

#[test]
fn matching_is_case_insensitive_and_keeps_mixed_source_order() {
    assert_result(
        "TO A CERTAIN DEGREE; I WOULD ARGUE THAT; words, SO TO SPEAK; I would argue that.",
        1,
        vec![
            diagnostic([1, 20], 1),
            diagnostic([22, 40], 22),
            diagnostic([47, 60], 47),
            diagnostic([62, 80], 62),
        ],
    );
}

#[test]
fn word_boundary_near_misses_are_clean() {
    assert_result(
        "xI would argue thatx; x, so to speakx; xto a certain degreex.",
        0,
        vec![],
    );
}

#[test]
fn punctuation_boundaries_allow_all_phrases() {
    assert_result(
        "(I would argue that)! words, so to speak! [to a certain degree]?",
        1,
        vec![
            diagnostic([2, 20], 2),
            diagnostic([28, 41], 28),
            diagnostic([44, 63], 44),
        ],
    );
}

#[test]
fn quoted_occurrences_are_suppressed() {
    assert_result(
        "Quoted \"I would argue that\" and \"to a certain degree\"; outside, so to speak.",
        1,
        vec![diagnostic([63, 76], 63)],
    );
}

#[test]
fn repeated_matches_preserve_source_order() {
    assert_result(
        "I would argue that, then I would argue that.",
        1,
        vec![diagnostic([1, 19], 1), diagnostic([26, 44], 26)],
    );
}

#[test]
fn unicode_prefixes_use_codepoint_positions() {
    assert_result(
        "é before I would argue that; e\u{301} before to a certain degree.",
        1,
        vec![diagnostic([10, 28], 10), diagnostic([40, 59], 40)],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good draft, I would argue that, uses TODO.",
        1,
        vec![diagnostic([20, 38], 20)],
    );
}

#[test]
fn valid_comma_phrase_activation_emits_embedded_raw_copy() {
    assert_result(
        "word, so to speak; x, so to speakx",
        1,
        vec![diagnostic([5, 18], 5), diagnostic([21, 34], 21)],
    );
}

#[test]
fn unicode_casefold_regression_hedging() {
    assert_result(
        "x, so to speak x, ſo to speak",
        1,
        vec![
            json!({"check_path":"hedging","message":"Hedging. Just say it.","pos":[1,2],"replacements":null,"span":[2,15]}),
            json!({"check_path":"hedging","message":"Hedging. Just say it.","pos":[1,17],"replacements":null,"span":[17,30]}),
        ],
    );
}
