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
            "proselint-rs-decade-apostrophes-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"dates_times":{{"dates":{{"decade_apostrophes":true}}}}}}}}"#),
        )
        .expect("write config selecting decade_apostrophes");
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
        "check_path": "dates_times.dates.decade_apostrophes",
        "message": "Apostrophes aren't needed for decades.",
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

#[test]
fn both_same_path_registrations_report_overlapping_decade_forms() {
    assert_result(
        "1990's",
        1000,
        vec![diagnostic([1, 7], 1), diagnostic([3, 7], 3)],
    );
    assert_result(
        "the 90's and 1970's and 2000'S",
        1000,
        vec![
            diagnostic([5, 9], 5),
            diagnostic([14, 20], 14),
            diagnostic([16, 20], 16),
            diagnostic([25, 31], 25),
            diagnostic([27, 31], 27),
        ],
    );
}

#[test]
fn rejects_missing_apostrophes_and_suppresses_quoted_matches() {
    assert_result("1990s 1989's 1990’s", 1000, vec![]);
    assert_result("١٩٩0's", 1000, vec![]);
    assert_result(
        "“1990's” and 2000's",
        1000,
        vec![diagnostic([14, 20], 14), diagnostic([16, 20], 16)],
    );
}

#[test]
fn keeps_unanchored_raw_matches_and_unicode_codepoint_positions() {
    assert_result(
        "x1990'sy",
        1000,
        vec![diagnostic([2, 8], 2), diagnostic([4, 8], 4)],
    );
    assert_result(
        "é 1990's",
        1000,
        vec![diagnostic([3, 9], 3), diagnostic([5, 9], 5)],
    );
}

#[test]
fn max_errors_truncates_in_registration_order_before_position_sort() {
    let input = "1990's and 2000's";
    assert_result(input, 1, vec![diagnostic([3, 7], 3)]);
    assert_result(
        input,
        2,
        vec![diagnostic([3, 7], 3), diagnostic([14, 18], 14)],
    );
    assert_result(
        input,
        3,
        vec![
            diagnostic([1, 7], 1),
            diagnostic([3, 7], 3),
            diagnostic([14, 18], 14),
        ],
    );
}
