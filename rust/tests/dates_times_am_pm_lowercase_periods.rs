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
            "proselint-rs-am-pm-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"dates_times":{"am_pm":{"lowercase_periods":true}}}}"#,
        )
        .expect("write config selecting only dates_times.am_pm.lowercase_periods");
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
        "check_path": "dates_times.am_pm.lowercase_periods",
        "message": "It's standard to use periods for lowercase 'a.m.' or 'p.m.'.",
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
fn matches_one_and_two_digit_lowercase_periods_without_or_with_one_space() {
    assert_result("It happened at 7am.", 1, vec![diagnostic([16, 19], 16)]);
    assert_result("Call at 12 pm.", 1, vec![diagnostic([9, 14], 9)]);
}

#[test]
fn preserves_order_and_exact_spans_for_multiple_matches() {
    assert_result(
        "7am, 12 pm, 9am.",
        1,
        vec![
            diagnostic([1, 4], 1),
            diagnostic([6, 11], 6),
            diagnostic([13, 16], 13),
        ],
    );
}

#[test]
fn rejects_case_period_spacing_and_word_boundary_near_misses() {
    assert_result("At 7AM, 12 PM, 7a.m., and 12 p.m.", 0, vec![]);
    assert_result("At 7  am and 123am.", 0, vec![]);
    assert_result("x7am 7amx 7am_thing A7amZ", 0, vec![]);
    assert_result("7am-foo", 1, vec![diagnostic([1, 4], 1)]);
}

#[test]
fn suppresses_quotes_and_reports_unicode_codepoint_positions() {
    assert_result("She wrote \"7am\" and then left.", 0, vec![]);
    assert_result(
        "é and e\u{301} before 7am.",
        1,
        vec![diagnostic([17, 20], 17)],
    );
}

#[test]
fn explicit_rule_selection_excludes_unrelated_default_checks() {
    assert_result(
        "A very good time is 7am.",
        1,
        vec![diagnostic([21, 24], 21)],
    );
}
