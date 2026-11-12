#[path = "../src/rules/spelling_able_ible.rs"]
mod spelling_able_ible;

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
            "proselint-rs-able-ible-{}-{serial}",
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
fn detects_case_insensitive_forms_with_exact_coordinates_and_replacements() {
    assert_eq!(
        spelling_able_ible::diagnostics("é accessable and VISABLE"),
        vec![
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'accessible' is the preferred spelling.","span":[3,13],"pos":[1,3],"replacements":"accessible"}),
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'visible' is the preferred spelling.","span":[18,25],"pos":[1,18],"replacements":"visible"}),
        ] as Vec<Value>
    );
}

#[test]
fn retains_the_complete_pinned_mapping_table() {
    let input = spelling_able_ible::form_text_for_test();
    let actual = spelling_able_ible::diagnostics(&input);
    assert_eq!(actual.len(), 173);
    assert_eq!(actual[0]["replacements"], "actionable");
    assert_eq!(actual[1]["replacements"], "addable");
    assert!(actual.iter().any(|d| d["replacements"] == "accessible"));
    assert!(actual.iter().any(|d| d["replacements"] == "visible"));
}

#[test]
fn emits_raw_occurrences_after_boundary_activation() {
    let actual = spelling_able_ible::diagnostics("xaccessable accessablex AcCeSsAbLe accessable");
    assert_eq!(actual.len(), 4);
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![
            json!([2, 12]),
            json!([13, 23]),
            json!([25, 35]),
            json!([36, 46]),
        ]
    );
    assert!(actual.iter().all(|d| d["replacements"] == "accessible"));
}

#[test]
fn suppresses_paired_quotes_and_uses_codepoint_positions() {
    assert!(spelling_able_ible::diagnostics("“accessable” and ‘visable’").is_empty());
    let actual = spelling_able_ible::diagnostics("é accessable\nvisable");
    assert_eq!(actual[0]["span"], json!([3, 13]));
    assert_eq!(actual[0]["pos"], json!([1, 3]));
    assert_eq!(actual[1]["span"], json!([14, 21]));
    assert_eq!(actual[1]["pos"], json!([2, 1]));
}

#[test]
fn matches_re2_unicode_case_behavior_and_exact_lookup() {
    assert_eq!(
        spelling_able_ible::diagnostics("ſensable senſable visable vіsable"),
        vec![
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'None' is the preferred spelling.","span":[1,9],"pos":[1,1],"replacements":null}),
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'None' is the preferred spelling.","span":[10,18],"pos":[1,10],"replacements":null}),
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'visible' is the preferred spelling.","span":[19,26],"pos":[1,19],"replacements":"visible"}),
        ] as Vec<Value>
    );
}

#[test]
fn matcher_order_uses_earliest_pattern_activation() {
    let actual = spelling_able_ible::diagnostics("visable actionible edable");
    assert_eq!(
        actual
            .iter()
            .map(|d| d["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![json!("visible"), json!("actionable"), json!("edible")]
    );
    assert_eq!(actual[0]["span"], json!([1, 8]));
}

#[test]
fn default_config_dispatches_able_ible_and_keeps_preferred_form_clean() {
    let positive = run_default_check("actionible");
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
        json!({"result":{"<stdin>":{"diagnostics":[{"check_path":"spelling.able_ible","message":"-able vs. -ible. 'actionable' is the preferred spelling.","pos":[1,1],"replacements":"actionable","span":[1,11]}]}}})
    );

    let clean = run_default_check("actionable");
    assert_eq!(clean.status.code(), Some(0), "stderr: {:?}", clean.stderr);
    assert!(clean.stderr.is_empty());
    let actual: Value = serde_json::from_slice(&clean.stdout).expect("valid JSON output");
    assert_eq!(actual, json!({"result":{"<stdin>":{"diagnostics":[]}}}));
}
