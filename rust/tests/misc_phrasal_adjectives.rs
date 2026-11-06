use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "No hyphen is necessary in phrasal adjectives with an adverb ending in -ly, unless the -ly adverb is part of a longer phrase.";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-phrasal-adjectives-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"phrasal_adjectives":{"ly":true}}}}"#,
        )
        .expect("write config selecting only misc.phrasal_adjectives.ly");
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

fn diagnostic(span: [usize; 2], line: usize, column: usize) -> Value {
    json!({"check_path":"misc.phrasal_adjectives.ly", "message":MESSAGE,
        "pos":[line,column], "replacements":null, "span":span})
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
fn reports_matches_in_source_order_and_uses_python_whitespace() {
    assert_result(
        "It is carefully-written\tquietly-read\npoorly-lit",
        1,
        vec![
            diagnostic([7, 17], 1, 7),
            diagnostic([25, 33], 1, 25),
            diagnostic([38, 45], 2, 1),
        ],
    );
    assert_result(
        "It is carefully-written\u{a0}quietly-read",
        1,
        vec![diagnostic([7, 17], 1, 7)],
    );
}

#[test]
fn rejects_nonmatches_reports_quotes_and_selects_only_this_check() {
    assert_result(
        "carefully written; careful-ly written; carefully—written",
        0,
        vec![],
    );
    assert_result(
        "“carefully-written” and quietly-read",
        1,
        vec![diagnostic([1, 12], 1, 1), diagnostic([25, 33], 1, 25)],
    );
    assert_result(
        "TODO; carefully-written; very",
        1,
        vec![diagnostic([7, 17], 1, 7)],
    );
}

#[test]
fn positions_use_unicode_codepoints() {
    assert_result(
        "é text carefully-written and 雪 quietly-read",
        1,
        vec![diagnostic([8, 18], 1, 8), diagnostic([32, 40], 1, 32)],
    );
}

#[test]
fn positive_and_negative_examples_match_the_oracle() {
    assert_result(
        "This is a carefully-written note; THIS IS A QUIETLY-READ result",
        1,
        vec![diagnostic([11, 21], 1, 11), diagnostic([45, 53], 1, 45)],
    );
    assert_result(
        "carefully written; careful-ly written; carefully—written",
        0,
        vec![],
    );
}
