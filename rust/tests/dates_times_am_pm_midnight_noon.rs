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
            "proselint-rs-midnight-noon-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"dates_times":{{"am_pm":{{"midnight_noon":true}}}}}}}}"#),
        )
        .expect("write config selecting the shared midnight_noon path");
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
        .expect("write input");
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize, message: &str) -> Value {
    json!({
        "check_path": "dates_times.am_pm.midnight_noon",
        "message": message,
        "pos": [1, column],
        "replacements": null,
        "span": span,
    })
}

const AMBIGUOUS: &str = "'12 a.m.' and '12 p.m.' are ambiguous. Use 'midnight' or 'noon'.";
const REDUNDANT: &str = "'a.m.' is always morning; 'p.m.' is always night.";

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
fn matches_optional_periods_spacing_and_case() {
    assert_result(
        "12 a.m.; 12 p.m.",
        1000,
        vec![
            diagnostic([1, 8], 1, AMBIGUOUS),
            diagnostic([10, 17], 10, AMBIGUOUS),
        ],
    );
    assert_result(
        "12am 12AM 12 p.m. 12P.M.",
        1000,
        vec![
            diagnostic([1, 5], 1, AMBIGUOUS),
            diagnostic([6, 10], 6, AMBIGUOUS),
            diagnostic([11, 18], 11, AMBIGUOUS),
            diagnostic([19, 25], 19, AMBIGUOUS),
        ],
    );
}

#[test]
fn preserves_unanchored_matches_and_rejects_near_misses() {
    assert_result(
        "x12amx; 11am; 13pm; 12  am; 12\tam",
        1000,
        vec![diagnostic([2, 6], 2, AMBIGUOUS)],
    );
}

#[test]
fn quote_suppression_and_unicode_positions_match_python() {
    assert_result(
        "“12am” and 12pm",
        1000,
        vec![diagnostic([12, 16], 12, AMBIGUOUS)],
    );
    assert_result("é 12am", 1000, vec![diagnostic([3, 7], 3, AMBIGUOUS)]);
}

#[test]
fn both_same_path_registrations_and_max_errors_order_match_python() {
    let input = "12 p.m. in the evening; 12 a.m. in the morning";
    let ambiguous = |span, column| diagnostic(span, column, AMBIGUOUS);
    let redundant = |span, column| diagnostic(span, column, REDUNDANT);
    assert_result(input, 1, vec![ambiguous([1, 8], 1)]);
    assert_result(
        input,
        2,
        vec![ambiguous([1, 8], 1), ambiguous([25, 32], 25)],
    );
    assert_result(
        input,
        3,
        vec![
            ambiguous([1, 8], 1),
            redundant([1, 23], 1),
            ambiguous([25, 32], 25),
        ],
    );
    assert_result(
        input,
        1000,
        vec![
            ambiguous([1, 8], 1),
            redundant([1, 23], 1),
            ambiguous([25, 32], 25),
            redundant([25, 47], 25),
        ],
    );
}

#[test]
fn fast_set_boundary_activation_keeps_raw_embedded_matches() {
    assert_result(
        "12 p.m. in the evening; x12 p.m. in the evening",
        1000,
        vec![
            diagnostic([1, 8], 1, AMBIGUOUS),
            diagnostic([1, 23], 1, REDUNDANT),
            diagnostic([26, 33], 26, AMBIGUOUS),
            diagnostic([26, 48], 26, REDUNDANT),
        ],
    );
}
