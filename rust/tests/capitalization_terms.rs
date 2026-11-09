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
            "proselint-rs-capterms-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"capitalization":{"terms":true}}}}"#,
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
fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({"check_path":"misc.capitalization.terms","message":format!("Incorrect capitalization. '{replacement}' is the preferred form."),"pos":[1,column],"replacements":replacement,"span":span})
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
fn maps_all_four_terms() {
    assert_result(
        "mother nature; Space Age; stone age; the American west.",
        1,
        vec![
            diagnostic([1, 14], 1, "Mother Nature"),
            diagnostic([16, 25], 16, "space age"),
            diagnostic([27, 36], 27, "Stone Age"),
            diagnostic([38, 55], 38, "the American West"),
        ],
    );
}
#[test]
fn preferred_forms_are_clean() {
    assert_result(
        "Mother Nature; space age; Stone Age; the American West.",
        0,
        vec![],
    );
}
#[test]
fn other_case_variants_are_clean() {
    assert_result(
        "MOTHER NATURE; Mother nature; SPACE AGE; space Age; STONE AGE; Stone age; The American West; the american west.",
        0,
        vec![],
    );
}
#[test]
fn longer_word_boundaries_are_clean() {
    assert_result(
        "xmother nature; mother naturex; Space Agex; _stone age; the American west2.",
        0,
        vec![],
    );
}
#[test]
fn punctuation_boundaries_are_allowed() {
    assert_result(
        "(mother nature)! [Space Age]? {stone age}, <the American west>.",
        1,
        vec![
            diagnostic([2, 15], 2, "Mother Nature"),
            diagnostic([19, 28], 19, "space age"),
            diagnostic([32, 41], 32, "Stone Age"),
            diagnostic([45, 62], 45, "the American West"),
        ],
    );
}
#[test]
fn quotes_are_suppressed_but_unquoted_terms_match() {
    assert_result(
        "Quoted \"mother nature\" and \"Space Age\"; outside stone age and the American west.",
        1,
        vec![
            diagnostic([49, 58], 49, "Stone Age"),
            diagnostic([63, 80], 63, "the American West"),
        ],
    );
}
#[test]
fn repeated_matches_keep_source_order() {
    assert_result(
        "the American west; mother nature; the American west; Space Age; stone age.",
        1,
        vec![
            diagnostic([1, 18], 1, "the American West"),
            diagnostic([20, 33], 20, "Mother Nature"),
            diagnostic([35, 52], 35, "the American West"),
            diagnostic([54, 63], 54, "space age"),
            diagnostic([65, 74], 65, "Stone Age"),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_coordinates() {
    assert_result(
        "é before mother nature; e\u{301} before the American west.",
        1,
        vec![
            diagnostic([10, 23], 10, "Mother Nature"),
            diagnostic([35, 52], 35, "the American West"),
        ],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: mother nature is quiet. TODO.",
        1,
        vec![diagnostic([23, 36], 23, "Mother Nature")],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "mother nature xmother nature",
        1,
        vec![
            diagnostic([1, 14], 1, "Mother Nature"),
            diagnostic([16, 29], 16, "Mother Nature"),
        ],
    );
}
