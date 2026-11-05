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
            "proselint-rs-collusion-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"illogic":{"collusion":true}}}}"#,
        )
        .expect("write config selecting only misc.illogic.collusion");
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
        "check_path": "misc.illogic.collusion",
        "message": "It's impossible to defraud yourself. Try 'aquiescence'.",
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
fn detects_the_literal_pattern_in_context() {
    assert_result(
        "They acted without your collusion.",
        1,
        vec![diagnostic([12, 34], 12)],
    );
}

#[test]
fn matching_is_case_insensitive_and_keeps_match_order() {
    assert_result(
        "WITHOUT YOUR COLLUSION; Without Your Collusion; without your collusion",
        1,
        vec![
            diagnostic([1, 23], 1),
            diagnostic([25, 47], 25),
            diagnostic([49, 71], 49),
        ],
    );
}

#[test]
fn changed_separators_and_spacing_do_not_match() {
    assert_result(
        "without-your collusion; without  your collusion; without our collusion",
        0,
        vec![],
    );
}

#[test]
fn matches_inside_paired_quotes_are_suppressed() {
    assert_result("She said, 'without your collusion'.", 0, vec![]);
}

#[test]
fn literal_match_has_no_word_boundary_padding() {
    assert_result(
        "awithout your collusionary",
        1,
        vec![diagnostic([2, 24], 2)],
    );
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "Without your collusion, TODO.",
        1,
        vec![diagnostic([1, 23], 1)],
    );
}

#[test]
fn unicode_casefold_regression_collusion() {
    assert_result(
        "without your colluſion",
        1,
        vec![
            json!({"check_path":"misc.illogic.collusion","message":"It's impossible to defraud yourself. Try 'aquiescence'.","pos":[1,1],"replacements":null,"span":[1,23]}),
        ],
    );
}
