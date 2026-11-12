#[path = "../src/rules/spelling_able_atable.rs"]
mod spelling_able_atable;

use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-able-atable-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run_default_check(input: &str) -> Output {
    let workspace = Workspace::new();
    let home = workspace.0.join("home");
    let xdg = workspace.0.join("xdg");
    std::fs::create_dir_all(&home).expect("create isolated HOME");
    std::fs::create_dir_all(&xdg).expect("create isolated XDG_CONFIG_HOME");
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", xdg)
        .args(["check", "--output-format", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native CLI without config");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input.as_bytes())
        .expect("write input");
    child.wait_with_output().expect("wait for native CLI")
}

#[test]
fn reports_pinned_forms_with_codepoint_coordinates() {
    assert_eq!(
        spelling_able_atable::diagnostics("é abbreviatable, navigatable"),
        vec![
            json!({"check_path":"spelling.able_atable","message":"-able vs. -atable. 'abbreviable' is the preferred spelling.","span":[3,16],"pos":[1,3],"replacements":"abbreviable"}),
            json!({"check_path":"spelling.able_atable","message":"-able vs. -atable. 'navigable' is the preferred spelling.","span":[18,29],"pos":[1,18],"replacements":"navigable"}),
        ] as Vec<Value>
    );
}

#[test]
fn activation_uses_word_boundaries_then_emits_raw_occurrences() {
    assert!(spelling_able_atable::diagnostics("xAbbreviatable abbreviatablex").is_empty());
    let actual = spelling_able_atable::diagnostics("xAbbreviatable abbreviatablex AbBrEvIaTaBlE");
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["span"], json!([2, 15]));
    assert_eq!(actual[1]["span"], json!([16, 29]));
    assert_eq!(actual[2]["span"], json!([31, 44]));
    assert!(actual.iter().all(|d| d["replacements"] == "abbreviable"));
}

#[test]
fn suppresses_only_paired_quotes_and_keeps_line_positions() {
    assert!(spelling_able_atable::diagnostics("“abbreviatable” and ‘navigatable’").is_empty());
    let actual = spelling_able_atable::diagnostics("é abbreviatable\nnavigatable");
    assert_eq!(actual[0]["span"], json!([3, 16]));
    assert_eq!(actual[0]["pos"], json!([1, 3]));
    assert_eq!(actual[1]["span"], json!([17, 28]));
    assert_eq!(actual[1]["pos"], json!([2, 1]));
}

#[test]
fn follows_fastset_activation_order_before_cli_truncation() {
    let actual = spelling_able_atable::diagnostics("navigatable abbreviatable operatable");
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["replacements"], "navigable");
    assert_eq!(actual[1]["replacements"], "abbreviable");
    assert_eq!(actual[2]["replacements"], "operable");

    let input = spelling_able_atable::diagnostics("abbreviatable navigatable abbreviatable");
    assert_eq!(
        input[..2]
            .iter()
            .map(|d| d["replacements"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["abbreviable", "abbreviable"]
    );
}

#[test]
fn retains_all_pinned_mapping_pairs() {
    assert_eq!(
        spelling_able_atable::form_text_for_test()
            .split_whitespace()
            .count(),
        68
    );
}

#[test]
fn clean_preferred_forms_do_not_match() {
    assert!(spelling_able_atable::diagnostics("abbreviable navigable operable").is_empty());
}

#[test]
fn default_config_dispatches_able_atable_and_keeps_preferred_form_clean() {
    let positive = run_default_check("abbreviatable");
    assert_eq!(
        positive.status.code(),
        Some(1),
        "stderr: {:?}",
        positive.stderr
    );
    assert!(positive.stderr.is_empty());
    let actual: Value = serde_json::from_slice(&positive.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[{"check_path":"spelling.able_atable","message":"-able vs. -atable. 'abbreviable' is the preferred spelling.","pos":[1,1],"replacements":"abbreviable","span":[1,14]}]}}})
    );

    let clean = run_default_check("abbreviable");
    assert_eq!(clean.status.code(), Some(0), "stderr: {:?}", clean.stderr);
    assert!(clean.stderr.is_empty());
    let actual: Value = serde_json::from_slice(&clean.stdout).expect("valid JSON output");
    assert_eq!(actual, json!({"result":{"<stdin>":{"diagnostics":[]}}}));
}
