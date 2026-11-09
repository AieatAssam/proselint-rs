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
            "proselint-rs-write-good-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"cliches":{"misc":{"write_good":true}}}}"#,
        )
        .unwrap();
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

fn run(input: &str) -> Output {
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
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn diagnostic(span: [usize; 2], phrase: &str) -> Value {
    json!({"check_path":"cliches.misc.write_good","message":format!("'{phrase}' is a cliché"),"pos":[1,span[0]],"replacements":null,"span":span})
}

fn assert_findings(input: &str, expected: Vec<Value>) {
    let output = run(input);
    assert_eq!(
        output.status.code(),
        Some(if expected.is_empty() { 0 } else { 1 }),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"result":{"<stdin>":{"diagnostics":expected}}})
    );
}

#[test]
fn corpus_has_706_diagnostics_and_exact_pinned_corpus_order() {
    let output = run(include_str!(
        "../data/third_party/no_cliches_write_good.txt"
    ));
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = value["result"]["<stdin>"]["diagnostics"]
        .as_array()
        .unwrap();
    assert_eq!(findings.len(), 706);
    assert_eq!(findings[0], diagnostic([1, 25], "a chip off the old block"));
    assert_eq!(
        findings.last().unwrap(),
        &json!({
            "check_path":"cliches.misc.write_good",
            "message":"'young and vibrant' is a cliché",
            "pos":[697,1],
            "replacements":null,
            "span":[13308,13325]
        })
    );
}

#[test]
fn case_boundaries_quote_suppression_unicode_coordinates_and_plague_exception() {
    assert_findings(
        "A CLEAN SLATE; small a clean slatey",
        vec![
            diagnostic([1, 14], "A CLEAN SLATE"),
            diagnostic([22, 35], "a clean slate"),
        ],
    );
    assert_findings(
        "“a clean slate” and 😀 a clean slate",
        vec![diagnostic([23, 36], "a clean slate")],
    );
    assert_findings("like the plague", vec![]);
    assert_findings(
        "you're the boss",
        vec![diagnostic([1, 16], "you're the boss")],
    );
}

#[test]
fn raw_question_mark_quantifies_final_e_in_will_wonders_phrase() {
    assert_findings(
        "will wonders never ceas",
        vec![diagnostic([1, 24], "will wonders never ceas")],
    );
}

#[test]
fn max_errors_keeps_source_order_before_position_sort() {
    let workspace = Workspace::new();
    std::fs::write(
        workspace.path().join("selected config.json"),
        br#"{"max_errors":1,"checks":{"cliches":{"misc":{"write_good":true}}}}"#,
    )
    .unwrap();
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
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"you're the boss; a clean slate")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["result"]["<stdin>"]["diagnostics"][0],
        diagnostic([18, 31], "a clean slate")
    );
}
