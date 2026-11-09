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
            "proselint-rs-metadiscourse-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"metadiscourse":true}}}"#,
        )
        .expect("write selected config");
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
    child.wait_with_output().expect("wait for CLI")
}
fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({"check_path":"misc.metadiscourse","message":"Excessive metadiscourse.","pos":[1,column],"replacements":null,"span":span})
}
fn assert_result(input: &str, exit: i32, diagnostics: Vec<Value>) {
    let out = run_check(input);
    assert_eq!(out.status.code(), Some(exit), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}
#[test]
fn detects_all_five_upstream_phrases() {
    assert_result(
        "The preceeding discussion; The rest of this article; This chapter discusses; The preceding paragraph demonstrated; The previous section analyzed.",
        1,
        vec![
            diagnostic([1, 26], 1),
            diagnostic([28, 52], 28),
            diagnostic([54, 76], 54),
            diagnostic([78, 114], 78),
            diagnostic([116, 145], 116),
        ],
    );
}
#[test]
fn matching_is_case_insensitive_and_preserves_the_misspelling() {
    assert_result(
        "THE PRECEEDING DISCUSSION; the rest of this article; THIS CHAPTER DISCUSSES; the preceding paragraph demonstrated; THE PREVIOUS SECTION ANALYZED.",
        1,
        vec![
            diagnostic([1, 26], 1),
            diagnostic([28, 52], 28),
            diagnostic([54, 76], 54),
            diagnostic([78, 114], 78),
            diagnostic([116, 145], 116),
        ],
    );
    assert_result(
        "The preceeding discussion. The preceding discussion.",
        1,
        vec![diagnostic([1, 26], 1)],
    );
}
#[test]
fn word_boundaries_and_near_misses_are_clean() {
    assert_result(
        "xThe preceeding discussion; The preceeding discussionx; The rest of this articlex; xThis chapter discusses.",
        0,
        vec![],
    );
    assert_result(
        "The preceding discussion; The rest of that article; This chapter discussed; The preceding paragraph demonstrates; The previous section analyses.",
        0,
        vec![],
    );
}
#[test]
fn quotations_are_suppressed_but_unquoted_phrase_is_reported() {
    assert_result(
        "Quoted \"The rest of this article\" and \"This chapter discusses\"; outside The previous section analyzed the topic.",
        1,
        vec![diagnostic([73, 102], 73)],
    );
}
#[test]
fn repeated_matches_keep_source_order() {
    assert_result(
        "This chapter discusses; The rest of this article; This chapter discusses; The preceeding discussion.",
        1,
        vec![
            diagnostic([1, 23], 1),
            diagnostic([25, 49], 25),
            diagnostic([51, 73], 51),
            diagnostic([75, 100], 75),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_codepoint_coordinates() {
    assert_result(
        "é before This chapter discusses; e\u{301} before The previous section analyzed.",
        1,
        vec![diagnostic([10, 32], 10), diagnostic([44, 73], 44)],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: The rest of this article. TODO.",
        1,
        vec![diagnostic([23, 47], 23)],
    );
}

#[test]
fn unicode_casefold_regression_metadiscourse() {
    assert_result(
        "This chapter diſcusses",
        1,
        vec![
            json!({"check_path":"misc.metadiscourse","message":"Excessive metadiscourse.","pos":[1,1],"replacements":null,"span":[1,23]}),
        ],
    );
}
