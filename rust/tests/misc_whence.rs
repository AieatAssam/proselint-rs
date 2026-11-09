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
            "proselint-rs-whence-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"whence":true}}}"#,
        )
        .expect("write config selecting only misc.whence");
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
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "misc.whence",
        "message": "The 'from' in 'from whence' is not needed.",
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
fn detects_the_literal_pattern_in_surrounding_context() {
    assert_result(
        "Go back from whence you came!",
        1,
        vec![diagnostic([9, 20], 9)],
    );
}

#[test]
fn matching_is_case_insensitive_and_keeps_match_order() {
    assert_result(
        "FROM WHENCE; From Whence; from whence",
        1,
        vec![
            diagnostic([1, 12], 1),
            diagnostic([14, 25], 14),
            diagnostic([27, 38], 27),
        ],
    );
}

#[test]
fn changed_separators_do_not_match() {
    assert_result("from-whence; from  whence; from hence", 0, vec![]);
}

#[test]
fn matching_inside_quotes_is_suppressed() {
    assert_result("She said, 'from whence'.", 0, vec![]);
}

#[test]
fn literal_match_has_no_word_boundary_padding() {
    assert_result("misfrom whence", 1, vec![diagnostic([4, 15], 4)]);
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "Go back from whence you came, TODO.",
        1,
        vec![diagnostic([9, 20], 9)],
    );
}
