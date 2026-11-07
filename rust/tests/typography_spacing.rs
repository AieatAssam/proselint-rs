use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "Inconsistent spacing after period (1 vs. 2 spaces).";

struct Workspace(PathBuf);

impl Workspace {
    fn new(max_errors: usize) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-spacing-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(
                r#"{{"max_errors":{max_errors},"checks":{{"typography":{{"punctuation":{{"spacing":true}}}}}}}}"#
            ),
        )
        .expect("write config selecting only typography.punctuation.spacing");
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

fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({
        "check_path": "typography.punctuation.spacing",
        "message": MESSAGE,
        "pos": [1, column],
        "replacements": replacement,
        "span": span,
    })
}

fn assert_result(input: &str, max_errors: usize, exit: i32, diagnostics: Vec<Value>) {
    let output = run_check(input, max_errors);
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": diagnostics}}})
    );
}

#[test]
fn a_single_spacing_style_is_consistent() {
    assert_result("A. B. C! D?", 1000, 0, vec![]);
    assert_result("A.  B.  C!  D?", 1000, 0, vec![]);
}

#[test]
fn reports_two_spaces_when_one_space_is_the_majority() {
    assert_result(
        "A. B. C. D.  E!",
        1000,
        1,
        vec![diagnostic([11, 15], 11, r"[\.\?!] \w")],
    );
}

#[test]
fn reports_one_space_when_two_spaces_are_the_majority() {
    assert_result(
        "A.  B.  C.  D! E?",
        1000,
        1,
        vec![diagnostic([14, 17], 14, r"[\.\?!]  \w")],
    );
}

#[test]
fn ties_choose_one_space_as_the_minority_form() {
    assert_result(
        "A. B.  C!",
        1000,
        1,
        vec![diagnostic([2, 5], 2, r"[\.\?!]  \w")],
    );
}

#[test]
fn interleaved_input_reports_each_minority_match_in_source_order() {
    assert_result(
        "A. B.  C. D.  E! F? G.  H.",
        1000,
        1,
        vec![
            diagnostic([5, 9], 5, r"[\.\?!] \w"),
            diagnostic([12, 16], 12, r"[\.\?!] \w"),
            diagnostic([22, 26], 22, r"[\.\?!] \w"),
        ],
    );
}

#[test]
fn quoted_minority_matches_are_suppressed() {
    assert_result("“A.  B.” C. D. E. F!", 1000, 0, vec![]);
}

#[test]
fn unicode_coordinates_and_max_errors_match_the_pinned_cli() {
    assert_result(
        "é.  B! C. D? E.",
        1000,
        1,
        vec![diagnostic([2, 6], 2, r"[\.\?!] \w")],
    );
    assert_result(
        "A. B.  C. D.  E! F. G?",
        1,
        1,
        vec![diagnostic([5, 9], 5, r"[\.\?!] \w")],
    );
}
