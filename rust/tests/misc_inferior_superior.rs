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
            "proselint-rs-inferior-superior-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"inferior_superior":true}}}"#,
        )
        .expect("write config selecting only misc.inferior_superior");
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
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({
        "check_path": "misc.inferior_superior",
        "message": format!("'Inferior' and 'superior' are not true comparatives. Use '{replacement}'."),
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
fn replaces_inferior_than() {
    assert_result(
        "A is inferior than B.",
        1,
        vec![diagnostic([6, 19], 6, "inferior to")],
    );
}

#[test]
fn replaces_superior_than() {
    assert_result(
        "A is superior than B.",
        1,
        vec![diagnostic([6, 19], 6, "superior to")],
    );
}

#[test]
fn matches_case_insensitive_variants_and_expands_each_replacement() {
    assert_result(
        "SUPERIOR THAN; Superior Than; INFERIOR THAN; Inferior Than.",
        1,
        vec![
            diagnostic([1, 14], 1, "superior to"),
            diagnostic([16, 29], 16, "superior to"),
            diagnostic([31, 44], 31, "inferior to"),
            diagnostic([46, 59], 46, "inferior to"),
        ],
    );
}

#[test]
fn correct_preferred_forms_are_clean() {
    assert_result("inferior to; superior to.", 0, vec![]);
}

#[test]
fn word_boundary_near_misses_are_clean() {
    assert_result(
        "xinferior than; inferior thanx; _superior than; superior than_",
        0,
        vec![],
    );
}

#[test]
fn punctuation_boundaries_allow_both_forms() {
    assert_result(
        "(inferior than)! [superior than]?",
        1,
        vec![
            diagnostic([2, 15], 2, "inferior to"),
            diagnostic([19, 32], 19, "superior to"),
        ],
    );
}

#[test]
fn quoted_match_is_suppressed() {
    assert_result("She said \"inferior than\" and left.", 0, vec![]);
}

#[test]
fn reverse_pattern_order_still_reports_source_order() {
    assert_result(
        "superior than comes first; then inferior than.",
        1,
        vec![
            diagnostic([1, 14], 1, "superior to"),
            diagnostic([33, 46], 33, "inferior to"),
        ],
    );
}

#[test]
fn unicode_prefix_uses_codepoint_positions() {
    assert_result(
        "é before inferior than.",
        1,
        vec![diagnostic([10, 23], 10, "inferior to")],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good but inferior than excellent plan uses TODO.",
        1,
        vec![diagnostic([17, 30], 17, "inferior to")],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "inferior than xinferior than",
        1,
        vec![
            diagnostic([1, 14], 1, "inferior to"),
            diagnostic([16, 29], 16, "inferior to"),
        ],
    );
}

#[test]
fn unicode_casefold_regression_inferior_superior() {
    assert_result(
        "superior than ſuperior than",
        1,
        vec![
            json!({"check_path":"misc.inferior_superior","message":"'Inferior' and 'superior' are not true comparatives. Use 'superior to'.","pos":[1,1],"replacements":"superior to","span":[1,14]}),
            json!({"check_path":"misc.inferior_superior","message":"'Inferior' and 'superior' are not true comparatives. Use 'None'.","pos":[1,15],"replacements":null,"span":[15,28]}),
        ],
    );
}
