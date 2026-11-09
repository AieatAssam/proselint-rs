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
            "proselint-rs-greylist-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"greylist":true}}}"#,
        )
        .expect("write config selecting only misc.greylist");
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

fn diagnostic(span: [usize; 2], column: usize, term: &str) -> Value {
    let explanation = match term {
        "obviously" => "This is obviously an inadvisable word to use.",
        "utilize" => "Do you know anyone who needs to utilize the word utilize?",
        _ => panic!("unknown greylist term"),
    };
    json!({
        "check_path": "misc.greylist",
        "message": format!("Use of '{term}'. {explanation}"),
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
fn reports_obviously_with_its_term_specific_message() {
    assert_result(
        "This is obviously inadvisable.",
        1,
        vec![diagnostic([9, 18], 9, "obviously")],
    );
}

#[test]
fn reports_utilize_with_its_term_specific_message() {
    assert_result(
        "Please utilize this tool.",
        1,
        vec![diagnostic([8, 15], 8, "utilize")],
    );
}

#[test]
fn matches_case_variants_and_repeated_occurrences_in_source_order() {
    assert_result(
        "OBVIOUSLY, obviously; UTILIZE, utilize, utilize.",
        1,
        vec![
            diagnostic([1, 10], 1, "obviously"),
            diagnostic([12, 21], 12, "obviously"),
            diagnostic([23, 30], 23, "utilize"),
            diagnostic([32, 39], 32, "utilize"),
            diagnostic([41, 48], 41, "utilize"),
        ],
    );
}

#[test]
fn matches_embedded_substrings_without_word_boundaries() {
    assert_result(
        "xobviouslyx; utilizeXYZ; preutilize; XUTILIZEsuffix.",
        1,
        vec![
            diagnostic([2, 11], 2, "obviously"),
            diagnostic([14, 21], 14, "utilize"),
            diagnostic([29, 36], 29, "utilize"),
            diagnostic([39, 46], 39, "utilize"),
        ],
    );
}

#[test]
fn mixed_terms_follow_source_order_not_registry_order() {
    assert_result(
        "utilize first; obviously second; then utilize again.",
        1,
        vec![
            diagnostic([1, 8], 1, "utilize"),
            diagnostic([16, 25], 16, "obviously"),
            diagnostic([39, 46], 39, "utilize"),
        ],
    );
}

#[test]
fn quote_suppression_keeps_unquoted_occurrence() {
    assert_result(
        "Quoted: \"obviously utilize\". Outside: utilize.",
        1,
        vec![diagnostic([39, 46], 39, "utilize")],
    );
}

#[test]
fn unicode_prefixes_use_codepoint_positions() {
    assert_result(
        "é before obviously; e\u{301} before utilize.",
        1,
        vec![
            diagnostic([10, 19], 10, "obviously"),
            diagnostic([31, 38], 31, "utilize"),
        ],
    );
}

#[test]
fn selected_config_excludes_other_default_rules() {
    assert_result(
        "A very good plan will obviously utilize TODO.",
        1,
        vec![
            diagnostic([23, 32], 23, "obviously"),
            diagnostic([33, 40], 33, "utilize"),
        ],
    );
}

#[test]
fn unicode_casefold_regression_greylist() {
    assert_result(
        "obviouſly",
        1,
        vec![
            json!({"check_path":"misc.greylist","message":"Use of 'obviously'. This is obviously an inadvisable word to use.","pos":[1,1],"replacements":null,"span":[1,10]}),
        ],
    );
}
