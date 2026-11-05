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
            "proselint-rs-wallace-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"redundancy":{"misc":{"wallace":true}}}}"#,
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
    child.wait_with_output().expect("wait for CLI")
}
fn diagnostic(span: [usize; 2], column: usize, replacement: &str, original: &str) -> Value {
    json!({"check_path":"redundancy.misc.wallace","message":format!("Redundancy. Use '{replacement}' instead of '{original}'."),"pos":[1,column],"replacements":replacement,"span":span})
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
fn maps_both_redundant_forms() {
    assert_result(
        "rectangular in shape; audible to the ear.",
        1,
        vec![
            diagnostic([1, 21], 1, "rectangular", "rectangular in shape"),
            diagnostic([23, 41], 23, "audible", "audible to the ear"),
        ],
    );
}
#[test]
fn case_insensitive_messages_keep_replacement_first_and_surface_original_second() {
    assert_result(
        "RECTANGULAR IN SHAPE; Audible To The Ear.",
        1,
        vec![
            diagnostic([1, 21], 1, "rectangular", "RECTANGULAR IN SHAPE"),
            diagnostic([23, 41], 23, "audible", "Audible To The Ear"),
        ],
    );
}
#[test]
fn longer_word_boundaries_are_clean() {
    assert_result(
        "xrectangular in shape; rectangular in shapex; audible to the earx; xaudible to the ear.",
        0,
        vec![],
    );
}
#[test]
fn near_miss_phrases_are_clean() {
    assert_result(
        "rectangular shape; audible ear; rectangular in shapes; audible to ears.",
        0,
        vec![],
    );
}
#[test]
fn punctuation_boundaries_are_allowed() {
    assert_result(
        "(rectangular in shape)! [audible to the ear]?",
        1,
        vec![
            diagnostic([2, 22], 2, "rectangular", "rectangular in shape"),
            diagnostic([26, 44], 26, "audible", "audible to the ear"),
        ],
    );
}
#[test]
fn quoted_forms_are_suppressed() {
    assert_result(
        "Quoted \"rectangular in shape\" and \"audible to the ear\"; outside rectangular in shape and audible to the ear.",
        1,
        vec![
            diagnostic([65, 85], 65, "rectangular", "rectangular in shape"),
            diagnostic([90, 108], 90, "audible", "audible to the ear"),
        ],
    );
}
#[test]
fn repeated_matches_keep_source_order() {
    assert_result(
        "audible to the ear; rectangular in shape; audible to the ear; rectangular in shape.",
        1,
        vec![
            diagnostic([1, 19], 1, "audible", "audible to the ear"),
            diagnostic([21, 41], 21, "rectangular", "rectangular in shape"),
            diagnostic([43, 61], 43, "audible", "audible to the ear"),
            diagnostic([63, 83], 63, "rectangular", "rectangular in shape"),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_coordinates() {
    assert_result(
        "é before rectangular in shape; e\u{301} before audible to the ear.",
        1,
        vec![
            diagnostic([10, 30], 10, "rectangular", "rectangular in shape"),
            diagnostic([42, 60], 42, "audible", "audible to the ear"),
        ],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: rectangular in shape, TODO.",
        1,
        vec![diagnostic(
            [23, 43],
            23,
            "rectangular",
            "rectangular in shape",
        )],
    );
}

#[test]
fn unicode_casefold_regression_wallace() {
    assert_result(
        "rectangular in ſhape",
        1,
        vec![
            json!({"check_path":"redundancy.misc.wallace","message":"Redundancy. Use 'None' instead of 'rectangular in ſhape'.","pos":[1,1],"replacements":null,"span":[1,21]}),
        ],
    );
}
