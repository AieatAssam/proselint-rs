use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "'...' is an approximation, use the ellipsis symbol '…'.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-ellipsis-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"typography":{"symbols":{"ellipsis":true}}}}"#,
        )
        .expect("write config selecting only typography.symbols.ellipsis");
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
        "check_path": "typography.symbols.ellipsis",
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
fn reports_ascii_three_dot_approximation() {
    assert_result(
        "The draft says wait...",
        1,
        vec![diagnostic([20, 23], 20)],
    );
}

#[test]
fn reports_adjacent_nonoverlapping_matches_in_source_order() {
    assert_result(
        "One... two...... three…",
        1,
        vec![
            diagnostic([4, 7], 4),
            diagnostic([11, 14], 11),
            diagnostic([14, 17], 14),
        ],
    );
}

#[test]
fn ignores_short_ascii_runs_and_unicode_ellipsis_symbol() {
    assert_result("The sentence ends. .. and then …", 0, vec![]);
}

#[test]
fn suppresses_match_inside_paired_quotes() {
    assert_result("He wrote \"...\" literally.", 0, vec![]);
}

#[test]
fn uses_unicode_codepoint_positions_and_spans() {
    assert_result(
        "Wait… then é... and finally...",
        1,
        vec![diagnostic([13, 16], 13), diagnostic([28, 31], 28)],
    );
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "A very unusual claim says TODO; then wait...",
        1,
        vec![diagnostic([42, 45], 42)],
    );
}
