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
            "proselint-rs-latin-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"latin":true}}}"#,
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
    json!({"check_path":"misc.latin","message":format!("Use English. '{replacement}' is the preferred form."),"pos":[1,column],"replacements":replacement,"span":span})
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
fn maps_all_four_latin_forms_to_english() {
    assert_result(
        "ceteris paribus; inter alia; simpliciter; mutatis mutandis.",
        1,
        vec![
            diagnostic([1, 16], 1, "other things being equal"),
            diagnostic([18, 28], 18, "among other things"),
            diagnostic([30, 41], 30, "in and of itself"),
            diagnostic([43, 59], 43, "having made the necessary changes"),
        ],
    );
}
#[test]
fn matching_is_case_insensitive_and_replacements_are_returned() {
    assert_result(
        "CETERIS PARIBUS; Inter Alia; SIMPLICITER; Mutatis Mutandis.",
        1,
        vec![
            diagnostic([1, 16], 1, "other things being equal"),
            diagnostic([18, 28], 18, "among other things"),
            diagnostic([30, 41], 30, "in and of itself"),
            diagnostic([43, 59], 43, "having made the necessary changes"),
        ],
    );
}
#[test]
fn longer_word_boundaries_are_clean() {
    assert_result(
        "xceteris paribus; ceteris paribusx; inter aliax; xsimpliciter; mutatis mutandisx.",
        0,
        vec![],
    );
}
#[test]
fn misspelled_or_incomplete_forms_are_clean() {
    assert_result(
        "ceteris paribi; inter aliis; simplicity; mutatis mutandi.",
        0,
        vec![],
    );
}
#[test]
fn punctuation_boundaries_are_allowed() {
    assert_result(
        "(ceteris paribus)! [inter alia]? {simpliciter}, <mutatis mutandis>.",
        1,
        vec![
            diagnostic([2, 17], 2, "other things being equal"),
            diagnostic([21, 31], 21, "among other things"),
            diagnostic([35, 46], 35, "in and of itself"),
            diagnostic([50, 66], 50, "having made the necessary changes"),
        ],
    );
}
#[test]
fn quoted_forms_are_suppressed() {
    assert_result(
        "Quoted \"ceteris paribus\" and \"inter alia\"; outside simpliciter and mutatis mutandis.",
        1,
        vec![
            diagnostic([52, 63], 52, "in and of itself"),
            diagnostic([68, 84], 68, "having made the necessary changes"),
        ],
    );
}
#[test]
fn repeated_matches_keep_source_order() {
    assert_result(
        "mutatis mutandis; ceteris paribus; mutatis mutandis; inter alia; simpliciter.",
        1,
        vec![
            diagnostic([1, 17], 1, "having made the necessary changes"),
            diagnostic([19, 34], 19, "other things being equal"),
            diagnostic([36, 52], 36, "having made the necessary changes"),
            diagnostic([54, 64], 54, "among other things"),
            diagnostic([66, 77], 66, "in and of itself"),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_coordinates() {
    assert_result(
        "é before ceteris paribus; e\u{301} before simpliciter.",
        1,
        vec![
            diagnostic([10, 25], 10, "other things being equal"),
            diagnostic([37, 48], 37, "in and of itself"),
        ],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: ceteris paribus, TODO.",
        1,
        vec![diagnostic([23, 38], 23, "other things being equal")],
    );
}

#[test]
fn unicode_casefold_regression_latin() {
    assert_result(
        "mutatiſ mutandis",
        1,
        vec![
            json!({"check_path":"misc.latin","message":"Use English. 'None' is the preferred form.","pos":[1,1],"replacements":null,"span":[1,17]}),
        ],
    );
}
