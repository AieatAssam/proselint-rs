use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "Use the multiplication symbol ×, not the letter x.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-multiplication-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"typography":{"symbols":{"multiplication":true}}}}"#,
        )
        .expect("write config selecting only typography.symbols.multiplication");
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
        "check_path": "typography.symbols.multiplication",
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
fn reports_compact_multiplication_approximation() {
    assert_result("The area is 3x4.", 1, vec![diagnostic([13, 16], 13)]);
}

#[test]
fn accepts_optional_spaces_and_case_variants() {
    assert_result(
        "2 x3; 4x 5; 6 X 7.",
        1,
        vec![
            diagnostic([1, 5], 1),
            diagnostic([7, 11], 7),
            diagnostic([13, 18], 13),
        ],
    );
}

#[test]
fn reports_raw_substrings_in_source_order() {
    assert_result(
        "ab2x3cd and 12x34.",
        1,
        vec![diagnostic([3, 6], 3), diagnostic([13, 18], 13)],
    );
}

#[test]
fn rejects_other_symbols_spacing_and_non_ascii_digits() {
    assert_result("2 × 3; x3; 4x; 5  x 6; ٣x4; 3x４.", 0, vec![]);
}

#[test]
fn suppresses_patterns_inside_quotes() {
    assert_result("Quoted: \"3x4\" and '5 x 6'.", 0, vec![]);
}

#[test]
fn positions_follow_unicode_codepoints() {
    assert_result(
        "é 8x9, then 10 x 11.",
        1,
        vec![diagnostic([3, 6], 3), diagnostic([13, 20], 13)],
    );
}

#[test]
fn selected_config_does_not_enable_other_rules() {
    assert_result(
        "TODO and very; dimensions are 3x4.",
        1,
        vec![diagnostic([31, 34], 31)],
    );
}
