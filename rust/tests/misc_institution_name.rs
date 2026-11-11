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
            "proselint-rs-institution-name-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"institution_name":true}}}"#,
        )
        .expect("write config selecting only misc.institution_name");
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

fn run_default_check(input: &str) -> Output {
    let workspace = Workspace::new();
    std::fs::remove_file(workspace.path().join("selected config.json"))
        .expect("remove rule-selection config");
    let home = workspace.path().join("home");
    let xdg = workspace.path().join("xdg");
    std::fs::create_dir_all(&home).expect("create isolated HOME");
    std::fs::create_dir_all(&xdg).expect("create isolated XDG_CONFIG_HOME");
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace.path())
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
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize, original: &str) -> Value {
    json!({
        "check_path": "misc.institution_name",
        "message": format!(
            "Incorrect name. Use 'Virginia Polytechnic Institute and State University' instead of '{original}'."
        ),
        "pos": [1, column],
        "replacements": "Virginia Polytechnic Institute and State University",
        "span": span,
    })
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
        json!({"result": {"<stdin>": {"diagnostics": diagnostics}}})
    );
}

const ORIGINAL: &str = "Virginia Polytechnic and State University";

#[test]
fn recommends_the_preferred_name_in_context() {
    assert_result(
        "I went to the Virginia Polytechnic and State University.",
        1,
        vec![diagnostic([15, 56], 15, ORIGINAL)],
    );
}

#[test]
fn matches_case_variants_and_preserves_order() {
    let uppercase = "VIRGINIA POLYTECHNIC AND STATE UNIVERSITY";
    assert_result(
        "VIRGINIA POLYTECHNIC AND STATE UNIVERSITY; Virginia Polytechnic and State University.",
        1,
        vec![
            diagnostic([1, 42], 1, uppercase),
            diagnostic([44, 85], 44, ORIGINAL),
        ],
    );
}

#[test]
fn word_padding_rejects_embedded_names() {
    assert_result(
        "xVirginia Polytechnic and State University Virginia Polytechnic and State Universityx",
        0,
        vec![],
    );
}

#[test]
fn suppresses_names_inside_quotes() {
    assert_result(
        "We refer to 'Virginia Polytechnic and State University'.",
        0,
        vec![],
    );
}

#[test]
fn unicode_punctuation_forms_word_boundaries() {
    let uppercase = "VIRGINIA POLYTECHNIC AND STATE UNIVERSITY";
    assert_result(
        "—VIRGINIA POLYTECHNIC AND STATE UNIVERSITY—",
        1,
        vec![diagnostic([2, 43], 2, uppercase)],
    );
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "Virginia Polytechnic and State University, TODO.",
        1,
        vec![diagnostic([1, 42], 1, ORIGINAL)],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "Virginia Polytechnic and State University xVirginia Polytechnic and State University",
        1,
        vec![
            diagnostic([1, 42], 1, ORIGINAL),
            diagnostic([44, 85], 44, ORIGINAL),
        ],
    );
}

#[test]
fn unicode_casefold_regression_institution_name() {
    assert_result(
        "Virginia Polytechnic and State Univerſity",
        1,
        vec![
            json!({"check_path":"misc.institution_name","message":"Incorrect name. Use 'None' instead of 'Virginia Polytechnic and State Univerſity'.","pos":[1,1],"replacements":null,"span":[1,42]}),
        ],
    );
}

#[test]
fn default_config_dispatches_institution_name_and_keeps_clean_control() {
    let positive = run_default_check("I went to the Virginia Polytechnic and State University.");
    assert_eq!(
        positive.status.code(),
        Some(1),
        "stderr: {:?}",
        positive.stderr
    );
    assert!(positive.stderr.is_empty(), "stderr: {:?}", positive.stderr);
    let actual: Value = serde_json::from_slice(&positive.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": [diagnostic([15, 56], 15, ORIGINAL)]}}})
    );

    let clean = run_default_check("I went to Virginia Tech.");
    assert_eq!(clean.status.code(), Some(0), "stderr: {:?}", clean.stderr);
    assert!(clean.stderr.is_empty(), "stderr: {:?}", clean.stderr);
    let actual: Value = serde_json::from_slice(&clean.stdout).expect("valid JSON output");
    assert_eq!(actual, json!({"result": {"<stdin>": {"diagnostics": []}}}));
}
