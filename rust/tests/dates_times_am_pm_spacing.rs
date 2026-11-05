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
            "proselint-rs-am-pm-spacing-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"dates_times":{"am_pm":{"spacing":true}}}}"#,
        )
        .expect("write config selecting only dates_times.am_pm.spacing");
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
        "check_path": "dates_times.am_pm.spacing",
        "message": "It's standard to put a space before 'a.m.' or 'p.m.'.",
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
fn matches_case_insensitively_with_optional_periods() {
    assert_result(
        "7AM 12a.m 9Pm. 4pM",
        1,
        vec![
            diagnostic([1, 4], 1),
            diagnostic([5, 10], 5),
            diagnostic([11, 15], 11),
            diagnostic([16, 19], 16),
        ],
    );
}

#[test]
fn accepts_one_and_two_digit_hours() {
    assert_result(
        "7am 12pm",
        1,
        vec![diagnostic([1, 4], 1), diagnostic([5, 9], 5)],
    );
}

#[test]
fn whitespace_before_the_suffix_is_not_a_match() {
    assert_result("7 am 12\tam 9 p.m.", 0, vec![]);
}

#[test]
fn three_digit_hours_can_match_the_final_two_digits() {
    assert_result(
        "7am 123am 123p.m.",
        1,
        vec![
            diagnostic([1, 4], 1),
            diagnostic([6, 10], 6),
            diagnostic([12, 18], 12),
        ],
    );
}

#[test]
fn matches_are_unanchored_and_have_no_word_boundaries() {
    assert_result(
        "x7amx abc12a.m.def",
        1,
        vec![diagnostic([2, 5], 2), diagnostic([10, 16], 10)],
    );
}

#[test]
fn multiple_variants_preserve_order_and_exact_spans() {
    assert_result(
        "7am, 12A.M., 9p.m.",
        1,
        vec![
            diagnostic([1, 4], 1),
            diagnostic([6, 12], 6),
            diagnostic([14, 19], 14),
        ],
    );
}

#[test]
fn quoted_occurrences_are_suppressed() {
    assert_result("Say \"7AM\" and '12p.m.'.", 0, vec![]);
}

#[test]
fn selected_config_excludes_other_date_and_default_checks() {
    assert_result(
        "A very good time is 7am.",
        1,
        vec![diagnostic([21, 25], 21)],
    );
}
