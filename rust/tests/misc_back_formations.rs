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
            "proselint-rs-back-formations-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"back_formations":true}}}"#,
        )
        .expect("write config selecting only misc.back_formations");
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
        "check_path": "misc.back_formations",
        "message": "Back-formation. 'improper' is the preferred form.",
        "pos": [1, column],
        "replacements": "improper",
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
fn reports_the_preferred_form_in_context() {
    assert_result(
        "It is an improprietous use.",
        1,
        vec![diagnostic([10, 23], 10)],
    );
}

#[test]
fn maps_case_variants_and_preserves_order() {
    assert_result(
        "improprietous; IMPROPRIETOUS; Improprietous.",
        1,
        vec![
            diagnostic([1, 14], 1),
            diagnostic([16, 29], 16),
            diagnostic([31, 44], 31),
        ],
    );
}

#[test]
fn word_padding_rejects_embedded_terms() {
    assert_result("x_improprietous improprietousish", 0, vec![]);
}

#[test]
fn suppresses_matches_inside_quotes() {
    assert_result("She called it 'improprietous'.", 0, vec![]);
}

#[test]
fn unicode_punctuation_is_a_word_boundary() {
    assert_result("—IMPROPRIETOUS—", 1, vec![diagnostic([2, 15], 2)]);
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "An improprietous idea, TODO.",
        1,
        vec![diagnostic([4, 17], 4)],
    );
}
