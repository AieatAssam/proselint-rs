use serde_json::{Value, json};
use std::fs;
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
            "proselint-rs-annotations-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"annotations":{"misc":true}}}"#,
        )
        .expect("write config selecting only annotations.misc");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_check(input: &str) -> Output {
    let workspace = Workspace::new();
    run_check_in(&workspace, input)
}

fn run_check_in(workspace: &Workspace, input: &str) -> Output {
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

fn diagnostic(span: [usize; 2], pos: [usize; 2]) -> Value {
    json!({
        "check_path": "annotations.misc",
        "message": "Annotation left in text.",
        "pos": pos,
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
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": diagnostics}}})
    );
}

#[test]
fn reports_todo_with_padded_span_and_position() {
    assert_result("TODO", 1, vec![diagnostic([1, 5], [1, 1])]);
}

#[test]
fn annotation_check_is_case_sensitive() {
    assert_result("Todo", 0, vec![]);
}

#[test]
fn reports_all_supported_terms_in_source_order() {
    assert_result(
        "todo TODO Todo FIXME",
        1,
        vec![
            diagnostic([1, 5], [1, 1]),
            diagnostic([6, 10], [1, 6]),
            diagnostic([16, 21], [1, 16]),
        ],
    );
}

#[test]
fn ascii_word_boundaries_reject_embedded_annotations() {
    assert_result("TODO_foo TODO1 xTODO TODOx", 0, vec![]);
}

#[test]
fn suppresses_annotations_inside_matching_quotes() {
    assert_result("\"TODO\"", 0, vec![]);
}

#[test]
fn preserves_upstream_ascii_word_boundaries_and_codepoint_positions() {
    assert_result(
        "éTODO TODOé TODO—end éTODO TODO-list",
        1,
        vec![
            diagnostic([2, 6], [1, 2]),
            diagnostic([7, 11], [1, 7]),
            diagnostic([13, 17], [1, 13]),
            diagnostic([24, 28], [1, 24]),
            diagnostic([29, 33], [1, 29]),
        ],
    );
}

#[test]
fn selected_config_runs_annotation_without_default_weasel_rule() {
    assert_result(
        "A very good sentence with TODO.",
        1,
        vec![diagnostic([27, 31], [1, 27])],
    );
}

#[test]
fn max_errors_one_truncates_repeated_annotation_diagnostics() {
    let workspace = Workspace::new();
    let config = r#"{"max_errors":1,"checks":{"annotations":{"misc":true}}}"#;
    fs::write(workspace.path().join("selected config.json"), config).unwrap();
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
        .unwrap();
    child.stdin.take().unwrap().write_all(b"TODO TODO").unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let actual: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        actual["result"]["<stdin>"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "TODO xTODO",
        1,
        vec![diagnostic([1, 5], [1, 1]), diagnostic([7, 11], [1, 7])],
    );
}

#[test]
fn max_errors_one_keeps_todo_before_fixme() {
    let workspace = Workspace::new();
    fs::write(
        workspace.path().join("selected config.json"),
        br#"{"max_errors":1,"checks":{"annotations":{"misc":true}}}"#,
    )
    .unwrap();
    let output = run_check_in(&workspace, "TODO FIXME");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[diagnostic([1,5],[1,1])]}}})
    );
}
