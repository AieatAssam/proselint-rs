use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "(c) is a goofy alphabetic approximation, use the symbol ©.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-copyright-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"typography":{"symbols":{"copyright":true}}}}"#,
        )
        .expect("write config selecting only typography.symbols.copyright");
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
        "check_path": "typography.symbols.copyright",
        "message": MESSAGE,
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
fn reports_ascii_copyright_approximation() {
    assert_result(
        "This document includes (c) 2026.",
        1,
        vec![diagnostic([24, 27], 24)],
    );
}

#[test]
fn matches_case_variants_nested_text_and_substrings_in_source_order() {
    assert_result(
        "Copyright (C); then (c), then ((c)), then x(c)y.",
        1,
        vec![
            diagnostic([11, 14], 11),
            diagnostic([21, 24], 21),
            diagnostic([32, 35], 32),
            diagnostic([44, 47], 44),
        ],
    );
}

#[test]
fn rejects_spacing_and_real_copyright_symbol() {
    assert_result("Copyright c, ( c ), ©, and (cc).", 0, vec![]);
}

#[test]
fn suppresses_matches_inside_double_and_single_quotes() {
    assert_result("Quoted: \"(c)\" and '(c)'.", 0, vec![]);
}

#[test]
fn positions_follow_unicode_codepoints() {
    assert_result(
        "é (c) © (C)",
        1,
        vec![diagnostic([3, 6], 3), diagnostic([9, 12], 9)],
    );
}

#[test]
fn selected_config_does_not_enable_other_rules() {
    assert_result(
        "TODO and very words; copyright (c)",
        1,
        vec![diagnostic([32, 35], 32)],
    );
}
