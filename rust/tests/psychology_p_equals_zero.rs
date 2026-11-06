use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "Unless p really equals zero, you should use more decimal places.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-p-equals-zero-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"psychology":{"p_equals_zero":true}}}"#,
        )
        .expect("write config selecting only psychology.p_equals_zero");
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
        "check_path":"psychology.p_equals_zero",
        "message":MESSAGE,
        "pos":[1,column],
        "replacements":null,
        "span":span,
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
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}

#[test]
fn ordinary_and_uppercase_inputs_match_case_insensitively() {
    assert_result(
        "p = 0.00; P = 0.00",
        1,
        vec![diagnostic([1, 9], 1), diagnostic([11, 19], 11)],
    );
}

#[test]
fn raw_pattern_keeps_the_unescaped_dot_and_bounded_zero_quantifier() {
    assert_result("p = 0x00", 1, vec![diagnostic([1, 9], 1)]);
    assert_result("p = 0.00000", 1, vec![diagnostic([1, 11], 1)]);
}

#[test]
fn near_misses_and_newlines_do_not_match() {
    assert_result("p = 0.0; q = 0.00; p=0.00", 0, vec![]);
    assert_result("p = 0\n00", 0, vec![]);
}

#[test]
fn quoted_matches_are_suppressed_and_repeated_matches_keep_order() {
    assert_result("'p = 0.00' and p = 0x00", 1, vec![diagnostic([16, 24], 16)]);
    assert_result(
        "p = 0.00; p = 0x0000",
        1,
        vec![diagnostic([1, 9], 1), diagnostic([11, 21], 11)],
    );
}

#[test]
fn positions_count_unicode_codepoints() {
    assert_result("😀 p = 0x00", 1, vec![diagnostic([3, 11], 3)]);
}
