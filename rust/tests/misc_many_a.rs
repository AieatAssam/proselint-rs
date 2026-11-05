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
            "proselint-rs-many-a-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"many_a":true}}}"#,
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
    child.wait_with_output().expect("wait for CLI")
}
fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({"check_path":"misc.many_a","message":"'many a' requires a singular verb.","pos":[1,column],"replacements":replacement,"span":span})
}
fn assert_result(input: &str, exit: i32, diagnostics: Vec<Value>) {
    let out = run_check(input);
    assert_eq!(out.status.code(), Some(exit), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}
#[test]
fn maps_all_three_forms_to_singular_replacements() {
    assert_result(
        "There are many a reason. There have been many a day. There were many a day.",
        1,
        vec![
            diagnostic([7, 17], 7, "is many a"),
            diagnostic([32, 48], 32, "has been many a"),
            diagnostic([60, 71], 60, "was many a"),
        ],
    );
}
#[test]
fn replacements_are_case_insensitive_and_preserve_source_order() {
    assert_result(
        "ARE MANY A; Have Been Many A; WERE MANY A.",
        1,
        vec![
            diagnostic([1, 11], 1, "is many a"),
            diagnostic([13, 29], 13, "has been many a"),
            diagnostic([31, 42], 31, "was many a"),
        ],
    );
}
#[test]
fn punctuation_is_a_valid_word_boundary() {
    assert_result(
        "(are many a)! [have been many a]? {were many a}.",
        1,
        vec![
            diagnostic([2, 12], 2, "is many a"),
            diagnostic([16, 32], 16, "has been many a"),
            diagnostic([36, 47], 36, "was many a"),
        ],
    );
}
#[test]
fn longer_word_boundaries_and_correct_forms_are_clean() {
    assert_result(
        "xare many ax; have been many ax; xwere many axy.",
        0,
        vec![],
    );
    assert_result("is many a; has been many a; was many a.", 0, vec![]);
}
#[test]
fn quotations_are_suppressed_but_unquoted_form_is_reported() {
    assert_result(
        "Quoted \"are many a\" and \"were many a\"; outside have been many a day.",
        1,
        vec![diagnostic([48, 64], 48, "has been many a")],
    );
}
#[test]
fn repeated_forms_keep_order_and_unicode_prefix_positions() {
    assert_result(
        "were many a; are many a; were many a; have been many a.",
        1,
        vec![
            diagnostic([1, 12], 1, "was many a"),
            diagnostic([14, 24], 14, "is many a"),
            diagnostic([26, 37], 26, "was many a"),
            diagnostic([39, 55], 39, "has been many a"),
        ],
    );
    assert_result(
        "é before are many a; e\u{301} before were many a.",
        1,
        vec![
            diagnostic([10, 20], 10, "is many a"),
            diagnostic([32, 43], 32, "was many a"),
        ],
    );
}
#[test]
fn selected_rule_does_not_enable_other_default_checks() {
    assert_result(
        "A very good sentence: there are many a reason and TODO.",
        1,
        vec![diagnostic([29, 39], 29, "is many a")],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "are many a xare many a",
        1,
        vec![
            diagnostic([1, 11], 1, "is many a"),
            diagnostic([13, 23], 13, "is many a"),
        ],
    );
}
