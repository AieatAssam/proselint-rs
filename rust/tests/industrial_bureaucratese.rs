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
            "proselint-rs-bureaucratese-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"industrial_language":{"bureaucratese":true}}}"#,
        )
        .expect("write config selecting only industrial_language.bureaucratese");
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

fn diagnostic(span: [usize; 2], column: usize, matched: &str) -> Value {
    json!({
        "check_path": "industrial_language.bureaucratese",
        "message": format!("'{matched}' is bureaucratese."),
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
fn detects_meet_form_with_exact_message() {
    assert_result(
        "Please meet with your approval.",
        1,
        vec![diagnostic([8, 31], 8, "meet with your approval")],
    );
}

#[test]
fn detects_meets_form() {
    assert_result(
        "The report meets with your approval.",
        1,
        vec![diagnostic([12, 36], 12, "meets with your approval")],
    );
}

#[test]
fn matches_case_variants_and_preserves_matched_casing_and_order() {
    assert_result(
        "MEETS WITH YOUR APPROVAL; Meet with your approval; meets WITH YOUR approval.",
        1,
        vec![
            diagnostic([1, 25], 1, "MEETS WITH YOUR APPROVAL"),
            diagnostic([27, 50], 27, "Meet with your approval"),
            diagnostic([52, 76], 52, "meets WITH YOUR approval"),
        ],
    );
}

#[test]
fn matches_as_unanchored_substrings() {
    assert_result(
        "preMEET with your approvalX; Xmeets with your approvalY.",
        1,
        vec![
            diagnostic([4, 27], 4, "MEET with your approval"),
            diagnostic([31, 55], 31, "meets with your approval"),
        ],
    );
}

#[test]
fn punctuation_boundaries_are_accepted() {
    assert_result(
        "(meet with your approval)! meets with your approval?",
        1,
        vec![
            diagnostic([2, 25], 2, "meet with your approval"),
            diagnostic([28, 52], 28, "meets with your approval"),
        ],
    );
}

#[test]
fn structural_and_spacing_near_misses_are_clean() {
    assert_result(
        "meat with your approval; meet with approval; meet  with your approval; meet your approval.",
        0,
        vec![],
    );
}

#[test]
fn quoted_occurrence_is_suppressed_but_outside_match_remains() {
    assert_result(
        "Quoted \"meet with your approval\"; outside meets with your approval.",
        1,
        vec![diagnostic([43, 67], 43, "meets with your approval")],
    );
}

#[test]
fn repeated_matches_preserve_source_order() {
    assert_result(
        "meet with your approval, then meet with your approval.",
        1,
        vec![
            diagnostic([1, 24], 1, "meet with your approval"),
            diagnostic([31, 54], 31, "meet with your approval"),
        ],
    );
}

#[test]
fn unicode_prefixes_use_codepoint_positions() {
    assert_result(
        "é before meets with your approval; e\u{301} before meet with your approval.",
        1,
        vec![
            diagnostic([10, 34], 10, "meets with your approval"),
            diagnostic([46, 69], 46, "meet with your approval"),
        ],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good draft meets with your approval and TODO.",
        1,
        vec![diagnostic([19, 43], 19, "meets with your approval")],
    );
}
