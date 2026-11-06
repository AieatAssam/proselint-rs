use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(max_errors: usize) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-not-guilty-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"not_guilty":true}}}}}}"#),
        )
        .expect("write config selecting misc.not_guilty");
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

fn run(input: &str, max_errors: usize) -> Output {
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

fn diagnostic(input: &str, matched: &str) -> Value {
    let start = input.find(matched).expect("matched phrase occurs in input");
    // The pinned ExistenceSimple check reports Python `str.find` offsets plus
    // one for both ends. Preserve that upstream coordinate convention.
    let start = input[..start].chars().count() + 1;
    let end = start + matched.chars().count();
    json!({
        "check_path": "misc.not_guilty",
        "message": "'not guilty beyond a reasonable doubt' is an ambiguous phrasing.",
        "pos": [1, start],
        "replacements": null,
        "span": [start, end],
    })
}

// Oracle evidence: rust/artifacts/misc-not-guilty-green-20261106/live-green.json
// Before this correction, the Rust expected spans were one codepoint low on
// all six positive diagnostics: mismatches were 8 + 1 + 2 = 11 span fields
// across the three positive cases. These assertions now use the pinned
// Python's observed spans exactly and retain the same four-case coverage.

fn assert_result(input: &str, max_errors: usize, expected: Vec<Value>) {
    let output = run(input, max_errors);
    assert_eq!(
        output.status.code(),
        Some(if expected.is_empty() { 0 } else { 1 }),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": expected}}})
    );
}

#[test]
fn matches_each_optional_article_case_insensitively() {
    let input = "not guilty beyond reasonable doubt; not guilty beyond a reasonable doubt; not guilty beyond any reasonable doubt; NOT GUILTY BEYOND ANY REASONABLE DOUBT";
    assert_result(
        input,
        1000,
        [
            "not guilty beyond reasonable doubt",
            "not guilty beyond a reasonable doubt",
            "not guilty beyond any reasonable doubt",
            "NOT GUILTY BEYOND ANY REASONABLE DOUBT",
        ]
        .into_iter()
        .map(|phrase| diagnostic(input, phrase))
        .collect(),
    );
}

#[test]
fn rejects_near_misses() {
    assert_result(
        "not guilty beyond the reasonable doubt; not guilty despite reasonable uncertainty",
        1000,
        vec![],
    );
}

#[test]
fn quote_suppression_precedes_max_errors_and_matching_is_unanchored() {
    let input =
        "\"not guilty beyond a reasonable doubt\" then xnot guilty beyond any reasonable doubty";
    assert_result(
        input,
        1,
        vec![diagnostic(input, "not guilty beyond any reasonable doubt")],
    );
}

#[test]
fn positions_count_unicode_codepoints() {
    let input = "éxnot guilty beyond reasonable doubt";
    assert_result(
        input,
        1000,
        vec![diagnostic(input, "not guilty beyond reasonable doubt")],
    );
}
