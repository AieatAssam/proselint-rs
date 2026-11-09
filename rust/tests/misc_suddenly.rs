use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "Suddenly is nondescript, slows the action, and warns your reader.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-misc-suddenly-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"suddenly":true}}}"#,
        )
        .expect("write config selecting only misc.suddenly");
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
        "check_path": "misc.suddenly",
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
fn reports_case_sensitive_suddenly_comma() {
    assert_result(
        "Suddenly, the lights went out.",
        1,
        vec![diagnostic([1, 10], 1)],
    );
}

#[test]
fn lowercase_initial_letter_is_clean() {
    assert_result("suddenly, the lights went out.", 0, vec![]);
}

#[test]
fn reports_raw_substrings_in_source_order() {
    assert_result(
        "NotSuddenly, and Suddenly, again.",
        1,
        vec![diagnostic([4, 13], 4), diagnostic([18, 27], 18)],
    );
}

#[test]
fn suppresses_pattern_inside_paired_quotes() {
    assert_result("He wrote 'Suddenly,' in the note.", 0, vec![]);
}

#[test]
fn requires_exact_case_and_comma() {
    assert_result("Suddenly!; SuddenLy,", 0, vec![]);
}

#[test]
fn positions_follow_unicode_codepoints() {
    assert_result(
        "é Suddenly, then Suddenly,",
        1,
        vec![diagnostic([3, 12], 3), diagnostic([18, 27], 18)],
    );
}

#[test]
fn selected_config_does_not_enable_other_rules() {
    assert_result(
        "TODO; very; Suddenly, now",
        1,
        vec![diagnostic([13, 22], 13)],
    );
}
