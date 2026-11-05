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
            "proselint-rs-debased-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"debased":true}}}"#,
        )
        .expect("write config selecting only misc.debased");
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
        "check_path": "misc.debased",
        "message": "Debased language is a continuous temptation.",
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
fn detects_first_phrase() {
    assert_result(
        "A not unjustifiable assumption.",
        1,
        vec![diagnostic([1, 31], 1)],
    );
}

#[test]
fn detects_second_phrase() {
    assert_result(
        "It leaves much to be desired.",
        1,
        vec![diagnostic([4, 29], 4)],
    );
}

#[test]
fn detects_third_phrase() {
    assert_result(
        "This would serve no purpose.",
        1,
        vec![diagnostic([6, 28], 6)],
    );
}

#[test]
fn detects_fourth_phrase() {
    assert_result(
        "A consideration which we should do well to bear in mind.",
        1,
        vec![diagnostic([1, 56], 1)],
    );
}

#[test]
fn matching_is_case_insensitive_and_keeps_entry_order() {
    assert_result(
        "A NOT UNJUSTIFIABLE ASSUMPTION!; LEAVES MUCH TO BE DESIRED?; Would Serve No Purpose; A Consideration Which We Should Do Well To Bear In Mind.",
        1,
        vec![
            diagnostic([1, 31], 1),
            diagnostic([34, 59], 34),
            diagnostic([62, 84], 62),
            diagnostic([86, 141], 86),
        ],
    );
}

#[test]
fn word_boundary_near_misses_are_clean_without_other_hits() {
    assert_result(
        "xleaves much to be desiredx; xwould serve no purposex; xa not unjustifiable assumptionx; xa consideration which we should do well to bear in mindx",
        0,
        vec![],
    );
}

#[test]
fn altered_internal_spacing_does_not_match() {
    assert_result(
        "leaves  much to be desired; would serve  no purpose; a  not unjustifiable assumption; a consideration  which we should do well to bear in mind",
        0,
        vec![],
    );
}

#[test]
fn punctuation_boundaries_cover_all_four_phrases() {
    assert_result(
        "(a not unjustifiable assumption)! [leaves much to be desired]? {would serve no purpose}; (a consideration which we should do well to bear in mind).",
        1,
        vec![
            diagnostic([2, 32], 2),
            diagnostic([36, 61], 36),
            diagnostic([65, 87], 65),
            diagnostic([91, 146], 91),
        ],
    );
}

#[test]
fn repeated_occurrences_preserve_source_order() {
    assert_result(
        "leaves much to be desired, then leaves much to be desired.",
        1,
        vec![diagnostic([1, 26], 1), diagnostic([33, 58], 33)],
    );
}

#[test]
fn quoted_occurrences_are_suppressed() {
    assert_result("Quote: \"leaves much to be desired\".", 0, vec![]);
}

#[test]
fn unicode_prefix_uses_codepoint_positions() {
    assert_result(
        "é before leaves much to be desired.",
        1,
        vec![diagnostic([10, 35], 10)],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good document says leaves much to be desired and TODO.",
        1,
        vec![diagnostic([27, 52], 27)],
    );
}

#[test]
fn unicode_casefold_regression_debased() {
    assert_result(
        "leaveſ much to be desired",
        1,
        vec![
            json!({"check_path":"misc.debased","message":"Debased language is a continuous temptation.","pos":[1,1],"replacements":null,"span":[1,26]}),
        ],
    );
}
