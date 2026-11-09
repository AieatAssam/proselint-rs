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
            "proselint-rs-false-plurals-misc-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"false_plurals":{"misc":true}}}}"#,
        )
        .expect("write config selecting only misc.false_plurals.misc");
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
    run_check_in(&workspace, input)
}

fn run_check_in(workspace: &Workspace, input: &str) -> Output {
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

fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({
        "check_path": "misc.false_plurals.misc",
        "message": format!("The correct plural is '{replacement}'."),
        "pos": [1, column],
        "replacements": replacement,
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

#[test]
fn recommends_phenomena_for_phenomenons() {
    assert_result(
        "There were several phenomenons.",
        1,
        vec![diagnostic([20, 31], 20, "phenomena")],
    );
}

#[test]
fn recommends_talismans_for_talismen() {
    assert_result(
        "The spellbook mentioned talismen.",
        1,
        vec![diagnostic([25, 33], 25, "talismans")],
    );
}

#[test]
fn matches_case_variants_in_source_order() {
    assert_result(
        "TALISMEN; Phenomenons; talismen",
        1,
        vec![
            diagnostic([1, 9], 1, "talismans"),
            diagnostic([11, 22], 11, "phenomena"),
            diagnostic([24, 32], 24, "talismans"),
        ],
    );
}

#[test]
fn word_padding_rejects_embedded_and_altered_terms() {
    assert_result(
        "xphenomenons; phenomenonsish; xtalismen talismens",
        0,
        vec![],
    );
}

#[test]
fn suppresses_matches_inside_quotes() {
    assert_result("She called them 'phenomenons' and 'talismen'.", 0, vec![]);
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "There were several phenomenons, TODO.",
        1,
        vec![diagnostic([20, 31], 20, "phenomena")],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "talismen xtalismen",
        1,
        vec![
            diagnostic([1, 9], 1, "talismans"),
            diagnostic([11, 19], 11, "talismans"),
        ],
    );
}

#[test]
fn long_s_match_keeps_upstream_null_replacement_lookup() {
    assert_result(
        "taliſmen",
        1,
        vec![json!({
            "check_path": "misc.false_plurals.misc",
            "message": "The correct plural is 'None'.",
            "pos": [1, 1],
            "replacements": null,
            "span": [1, 9],
        })],
    );
}

#[test]
fn max_errors_two_keeps_talismen_source_order() {
    let workspace = Workspace::new();
    std::fs::write(
        workspace.path().join("selected config.json"),
        br#"{"max_errors":2,"checks":{"misc":{"false_plurals":{"misc":true}}}}"#,
    )
    .unwrap();
    let output = run_check_in(&workspace, "talismen phenomenons talismen");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[diagnostic([1,9],1,"talismans"),diagnostic([22,30],22,"talismans")]}}})
    );
}
