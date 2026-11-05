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
            "proselint-rs-seasons-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"capitalization":{"seasons":true}}}}"#,
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
fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({"check_path":"misc.capitalization.seasons","message":format!("Seasons shouldn't be capitalized. '{replacement}' is the preferred form."),"pos":[1,column],"replacements":replacement,"span":span})
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
fn maps_capitalized_winter_and_summer() {
    assert_result(
        "Winter; Summer.",
        1,
        vec![
            diagnostic([1, 7], 1, "winter"),
            diagnostic([9, 15], 9, "summer"),
        ],
    );
}
#[test]
fn matching_is_case_sensitive() {
    assert_result("winter; summer; WINTER; SUMMER; wInter; suMMer.", 0, vec![]);
}
#[test]
fn commented_spring_and_fall_forms_are_clean() {
    assert_result("Spring; Fall; spring; fall; SPRING; FALL.", 0, vec![]);
}
#[test]
fn longer_word_boundaries_are_clean() {
    assert_result(
        "xWinter; Winterx; Summer_ ; _Summer; WInterfall.",
        0,
        vec![],
    );
}
#[test]
fn punctuation_boundaries_allow_matches() {
    assert_result(
        "(Winter)! [Summer]? {Winter}, <Summer>.",
        1,
        vec![
            diagnostic([2, 8], 2, "winter"),
            diagnostic([12, 18], 12, "summer"),
            diagnostic([22, 28], 22, "winter"),
            diagnostic([32, 38], 32, "summer"),
        ],
    );
}
#[test]
fn quoted_forms_are_suppressed() {
    assert_result(
        "Quoted \"Winter\" and \"Summer\"; outside Winter and Summer.",
        1,
        vec![
            diagnostic([39, 45], 39, "winter"),
            diagnostic([50, 56], 50, "summer"),
        ],
    );
}
#[test]
fn repeated_matches_keep_source_order() {
    assert_result(
        "Summer; Winter; Summer; Winter.",
        1,
        vec![
            diagnostic([1, 7], 1, "summer"),
            diagnostic([9, 15], 9, "winter"),
            diagnostic([17, 23], 17, "summer"),
            diagnostic([25, 31], 25, "winter"),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_coordinates() {
    assert_result(
        "é before Winter; e\u{301} before Summer.",
        1,
        vec![
            diagnostic([10, 16], 10, "winter"),
            diagnostic([28, 34], 28, "summer"),
        ],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: Winter is coming. TODO.",
        1,
        vec![diagnostic([23, 29], 23, "winter")],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "Winter xWinter",
        1,
        vec![
            diagnostic([1, 7], 1, "winter"),
            diagnostic([9, 15], 9, "winter"),
        ],
    );
}
