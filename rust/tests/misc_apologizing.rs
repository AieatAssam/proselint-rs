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
            "proselint-rs-apologizing-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#,
        )
        .expect("write config selecting only misc.apologizing");
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
        "check_path": "misc.apologizing",
        "message": "Excessive apologizing.",
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
fn reports_the_literal_pattern_in_context() {
    assert_result(
        "Our policy states: More research is needed.",
        1,
        vec![diagnostic([20, 43], 20)],
    );
}

#[test]
fn literal_matching_is_case_insensitive_and_ordered() {
    assert_result(
        "MORE RESEARCH IS NEEDED; more research is needed.",
        1,
        vec![diagnostic([1, 24], 1), diagnostic([26, 49], 26)],
    );
}

#[test]
fn changed_separators_do_not_match() {
    assert_result(
        "More-research is needed; More research  is needed; More research is necessary.",
        0,
        vec![],
    );
}

#[test]
fn matching_inside_quotes_is_suppressed() {
    assert_result("She wrote, 'More research is needed.'", 0, vec![]);
}

#[test]
fn literal_match_has_no_word_boundary_padding() {
    assert_result("More research is neededly", 1, vec![diagnostic([1, 24], 1)]);
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "More research is needed, TODO.",
        1,
        vec![diagnostic([1, 24], 1)],
    );
}
