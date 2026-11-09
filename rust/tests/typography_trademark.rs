use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const TM_MESSAGE: &str = "(TM) is a goofy alphabetic approximation, use the symbol ™.";
const REGISTERED_MESSAGE: &str = "(R) is a goofy alphabetic approximation, use the symbol ®.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-trademark-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"typography":{"symbols":{"trademark":true}}}}"#,
        )
        .expect("write config selecting only typography.symbols.trademark");
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

fn diagnostic(span: [usize; 2], column: usize, message: &str) -> Value {
    json!({
        "check_path": "typography.symbols.trademark",
        "message": message,
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
fn reports_tm_approximation() {
    assert_result(
        "Use (TM) when needed.",
        1,
        vec![diagnostic([5, 9], 5, TM_MESSAGE)],
    );
}

#[test]
fn reports_registered_symbol_approximation() {
    assert_result(
        "Use (R) when needed.",
        1,
        vec![diagnostic([5, 8], 5, REGISTERED_MESSAGE)],
    );
}

#[test]
fn matches_case_variants_and_orders_both_entries_by_source() {
    assert_result(
        "Our (tm) mark, then (R), then ((TM)), and x(r)y.",
        1,
        vec![
            diagnostic([5, 9], 5, TM_MESSAGE),
            diagnostic([21, 24], 21, REGISTERED_MESSAGE),
            diagnostic([32, 36], 32, TM_MESSAGE),
            diagnostic([44, 47], 44, REGISTERED_MESSAGE),
        ],
    );
}

#[test]
fn rejects_spacing_near_misses_and_real_symbols() {
    assert_result("Use ( T M ), (r ), (tmx), and ™ ®.", 0, vec![]);
}

#[test]
fn suppresses_both_patterns_inside_quotes() {
    assert_result("Quoted \"(tm)\" and '(r)'.", 0, vec![]);
}

#[test]
fn positions_follow_unicode_codepoints() {
    assert_result(
        "é (TM) then ® then (R)",
        1,
        vec![
            diagnostic([3, 7], 3, TM_MESSAGE),
            diagnostic([20, 23], 20, REGISTERED_MESSAGE),
        ],
    );
}

#[test]
fn selected_config_does_not_enable_other_rules() {
    assert_result(
        "TODO and very; (TM) and (r)",
        1,
        vec![
            diagnostic([16, 20], 16, TM_MESSAGE),
            diagnostic([25, 28], 25, REGISTERED_MESSAGE),
        ],
    );
}
