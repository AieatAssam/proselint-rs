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
            "proselint-rs-kudos-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"false_plurals":{"kudos":true}}}}"#,
        )
        .expect("write config selecting only misc.false_plurals.kudos");
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
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "misc.false_plurals.kudos",
        "message": "Kudos is singular.",
        "pos": [1, column],
        "replacements": null,
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
fn reports_many_kudos_in_context() {
    assert_result("I give you many kudos.", 1, vec![diagnostic([12, 22], 12)]);
}

#[test]
fn matches_case_variants_in_source_order() {
    assert_result(
        "MANY KUDOS; Many Kudos; many kudos",
        1,
        vec![
            diagnostic([1, 11], 1),
            diagnostic([13, 23], 13),
            diagnostic([25, 35], 25),
        ],
    );
}

#[test]
fn changed_spacing_or_separator_does_not_match() {
    assert_result("many-kudos; many  kudos; several kudos", 0, vec![]);
}

#[test]
fn matching_inside_quotes_is_suppressed() {
    assert_result("She wrote, 'many kudos'.", 0, vec![]);
}

#[test]
fn literal_match_has_no_word_boundary_padding() {
    assert_result("somany kudosish", 1, vec![diagnostic([3, 13], 3)]);
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "I give you many kudos, TODO.",
        1,
        vec![diagnostic([12, 22], 12)],
    );
}

#[test]
fn case_insensitive_match_folds_kelvin_sign() {
    assert_result("many Kudos", 1, vec![diagnostic([1, 11], 1)]);
}
