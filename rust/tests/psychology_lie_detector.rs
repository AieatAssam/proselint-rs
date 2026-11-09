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
            "proselint-rs-lie-detector-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"psychology":{"lie_detector":true}}}"#,
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
    json!({"check_path":"psychology.lie_detector","message":format!("Polygraph machines measure arousal, not lying per se. Try '{replacement}'."),"pos":[1,column],"replacements":replacement,"span":span})
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
fn maps_both_lie_detector_forms() {
    assert_result(
        "lie detector test; lie detector machine.",
        1,
        vec![
            diagnostic([1, 18], 1, "polygraph test"),
            diagnostic([20, 40], 20, "polygraph machine"),
        ],
    );
}
#[test]
fn matching_is_case_insensitive_and_replacements_are_exact() {
    assert_result(
        "LIE DETECTOR TEST; Lie Detector Machine.",
        1,
        vec![
            diagnostic([1, 18], 1, "polygraph test"),
            diagnostic([20, 40], 20, "polygraph machine"),
        ],
    );
}
#[test]
fn longer_word_boundaries_are_clean() {
    assert_result(
        "xlie detector test; lie detector testx; lie detector machinex; xlie detector machine.",
        0,
        vec![],
    );
}
#[test]
fn near_miss_phrases_are_clean() {
    assert_result(
        "detector lie test; lie detectors test; lie detector machines.",
        0,
        vec![],
    );
}
#[test]
fn punctuation_boundaries_are_allowed() {
    assert_result(
        "(lie detector test)! [lie detector machine]?",
        1,
        vec![
            diagnostic([2, 19], 2, "polygraph test"),
            diagnostic([23, 43], 23, "polygraph machine"),
        ],
    );
}
#[test]
fn quotes_are_suppressed_but_unquoted_forms_match() {
    assert_result(
        "Quoted \"lie detector test\" and \"lie detector machine\"; outside lie detector test and lie detector machine.",
        1,
        vec![
            diagnostic([64, 81], 64, "polygraph test"),
            diagnostic([86, 106], 86, "polygraph machine"),
        ],
    );
}
#[test]
fn repeated_matches_keep_source_order() {
    assert_result(
        "lie detector machine; lie detector test; lie detector machine; lie detector test.",
        1,
        vec![
            diagnostic([1, 21], 1, "polygraph machine"),
            diagnostic([23, 40], 23, "polygraph test"),
            diagnostic([42, 62], 42, "polygraph machine"),
            diagnostic([64, 81], 64, "polygraph test"),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_coordinates() {
    assert_result(
        "é before lie detector test; e\u{301} before lie detector machine.",
        1,
        vec![
            diagnostic([10, 27], 10, "polygraph test"),
            diagnostic([39, 59], 39, "polygraph machine"),
        ],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: lie detector test, TODO.",
        1,
        vec![diagnostic([23, 40], 23, "polygraph test")],
    );
}

#[test]
fn fastset_activation_reports_raw_embedded_copies() {
    assert_result(
        "xlie detector testx; lie detector test; xlie detector testx.",
        1,
        vec![
            diagnostic([2, 19], 2, "polygraph test"),
            diagnostic([22, 39], 22, "polygraph test"),
            diagnostic([42, 59], 42, "polygraph test"),
        ],
    );
}

#[test]
fn unicode_letter_boundary_matches_upstream_re2_behavior() {
    assert_result(
        "élie detector test",
        1,
        vec![diagnostic([2, 19], 2, "polygraph test")],
    );
}

#[test]
fn unicode_casefold_regression_lie_detector() {
    assert_result(
        "lie detector teſt",
        1,
        vec![
            json!({"check_path":"psychology.lie_detector","message":"Polygraph machines measure arousal, not lying per se. Try 'None'.","pos":[1,1],"replacements":null,"span":[1,18]}),
        ],
    );
}
