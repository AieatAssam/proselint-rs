use proselint_rs::file_uri;
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
            "proselint-rs-misc-but-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"but":true}}}"#,
        )
        .expect("write config selecting only misc.but");
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

fn run_stdin(input: &str) -> Output {
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
        .expect("write stdin");
    child.wait_with_output().expect("wait for native CLI")
}

fn run_file(bytes: &[u8]) -> (Output, String) {
    let workspace = Workspace::new();
    let file = workspace.path().join("crlf document.txt");
    std::fs::write(&file, bytes).expect("write CRLF fixture");
    let source = file_uri(&file).expect("file URI");
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
            "crlf document.txt",
        ])
        .output()
        .expect("run native CLI on file");
    (output, source)
}

fn diagnostic(span: [usize; 2], line: usize, column: usize) -> Value {
    json!({
        "check_path": "misc.but",
        "message": "No paragraph should start with a 'But'.",
        "pos": [line, column],
        "replacements": null,
        "span": span,
    })
}

fn assert_stdin(input: &str, exit: i32, diagnostics: Vec<Value>) {
    let output = run_stdin(input);
    assert_eq!(
        output.status.code(),
        Some(exit),
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
fn flags_but_at_document_start_only() {
    assert_stdin(
        "But I never start with the word but.",
        1,
        vec![diagnostic([1, 4], 1, 1)],
    );
    assert_stdin("First.\nBut not a paragraph.", 0, vec![]);
}

#[test]
fn flags_each_blank_line_paragraph_start_in_order() {
    assert_stdin(
        "First.\n\nBut second.\n\nBut third.",
        1,
        vec![diagnostic([9, 12], 3, 1), diagnostic([22, 25], 5, 1)],
    );
}

#[test]
fn applies_case_sensitive_word_boundary() {
    assert_stdin(
        "but lowercase.\n\nButtery.\n\nBut.",
        1,
        vec![diagnostic([27, 30], 5, 1)],
    );
}

#[test]
fn ignores_inline_or_indented_but() {
    assert_stdin("Inline. But here.\n\n  But indented.", 0, vec![]);
}

#[test]
fn suppresses_quoted_paragraph_start_but_keeps_unquoted_one() {
    assert_stdin(
        "A quote says \"beginning.\n\nBut this is quoted.\"\n\nBut this is real.",
        1,
        vec![diagnostic([49, 52], 5, 1)],
    );
}

#[test]
fn recognizes_crlf_file_paragraphs_after_text_normalization() {
    let (output, source) =
        run_file(b"Intro.\r\n\r\nBut CRLF paragraph.\r\nNext.\r\n\r\nBut second.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{source:{"diagnostics":[diagnostic([9,12],3,1),diagnostic([36,39],6,1)]}}})
    );
}

#[test]
fn selected_config_excludes_annotations_and_weasel_words() {
    assert_stdin(
        "TODO; very.\n\nBut configured check only.",
        1,
        vec![diagnostic([14, 17], 3, 1)],
    );
}
