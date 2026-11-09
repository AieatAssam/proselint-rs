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
        let path =
            std::env::temp_dir().join(format!("proselint-rs-days-{}-{serial}", std::process::id()));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(
                r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"capitalization":{{"days":true}}}}}}}}"#
            ),
        )
        .expect("write selected config");
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

fn run_check(input: &str, max_errors: usize) -> Output {
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
    child.wait_with_output().expect("wait for CLI")
}

fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({
        "check_path":"misc.capitalization.days",
        "message":format!("Days of the week should be capitalized. '{replacement}' is the preferred form."),
        "pos":[1,column],
        "replacements":replacement,
        "span":span
    })
}

fn assert_result(input: &str, max_errors: usize, exit: i32, diagnostics: Vec<Value>) {
    let out = run_check(input, max_errors);
    assert_eq!(out.status.code(), Some(exit), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}

#[test]
fn maps_all_seven_lowercase_day_names() {
    assert_result(
        "monday tuesday wednesday thursday friday saturday sunday",
        1000,
        1,
        vec![
            diagnostic([1, 7], 1, "Monday"),
            diagnostic([8, 15], 8, "Tuesday"),
            diagnostic([16, 25], 16, "Wednesday"),
            diagnostic([26, 34], 26, "Thursday"),
            diagnostic([35, 41], 35, "Friday"),
            diagnostic([42, 50], 42, "Saturday"),
            diagnostic([51, 57], 51, "Sunday"),
        ],
    );
}

#[test]
fn capitalization_is_case_sensitive() {
    assert_result("Monday TUESDAY Sunday mondAy", 1000, 0, vec![]);
}

#[test]
fn word_boundaries_reject_embedded_only_names() {
    assert_result("xmonday mondayx _monday tuesday_", 1000, 0, vec![]);
}

#[test]
fn punctuation_boundaries_allow_each_day_name() {
    assert_result(
        "[monday], (tuesday)! {wednesday}? <thursday>.",
        1000,
        1,
        vec![
            diagnostic([2, 8], 2, "Monday"),
            diagnostic([12, 19], 12, "Tuesday"),
            diagnostic([23, 32], 23, "Wednesday"),
            diagnostic([36, 44], 36, "Thursday"),
        ],
    );
}

#[test]
fn quote_suppression_keeps_only_unquoted_match() {
    assert_result(
        "Quoted \"monday\"; outside tuesday.",
        1000,
        1,
        vec![diagnostic([26, 33], 26, "Tuesday")],
    );
}

#[test]
fn repeated_days_keep_source_order() {
    assert_result(
        "sunday monday sunday",
        1000,
        1,
        vec![
            diagnostic([1, 7], 1, "Sunday"),
            diagnostic([8, 14], 8, "Monday"),
            diagnostic([15, 21], 15, "Sunday"),
        ],
    );
}

#[test]
fn repeated_days_keep_source_order_and_max_errors_truncates() {
    assert_result(
        "sunday monday friday",
        1,
        1,
        vec![diagnostic([1, 7], 1, "Sunday")],
    );
}

#[test]
fn unicode_prefixes_use_python_codepoint_positions() {
    assert_result(
        "é monday; e\u{301} tuesday; 😀 wednesday",
        1000,
        1,
        vec![
            diagnostic([3, 9], 3, "Monday"),
            diagnostic([14, 21], 14, "Tuesday"),
            diagnostic([25, 34], 25, "Wednesday"),
        ],
    );
}

#[test]
fn valid_boundary_activation_preserves_embedded_raw_copy() {
    assert_result(
        "monday xmonday",
        1000,
        1,
        vec![
            diagnostic([1, 7], 1, "Monday"),
            diagnostic([9, 15], 9, "Monday"),
        ],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A good sentence: monday is here. TODO.",
        1000,
        1,
        vec![diagnostic([18, 24], 18, "Monday")],
    );
}
