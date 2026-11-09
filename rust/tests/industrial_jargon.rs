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
            "proselint-rs-jargon-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"industrial_language":{"jargon":true}}}"#,
        )
        .expect("write config selecting only industrial_language.jargon");
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

fn diagnostic(span: [usize; 2], column: usize, matched: &str) -> Value {
    json!({
        "check_path": "industrial_language.jargon",
        "message": format!("'{matched}' is jargon. Can you replace it with something more standard?"),
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
fn detects_all_six_registered_phrases_in_source_order() {
    assert_result(
        "in the affirmative; in the negative; agendize; per your order; per your request; disincentivize.",
        1,
        vec![
            diagnostic([1, 19], 1, "in the affirmative"),
            diagnostic([21, 36], 21, "in the negative"),
            diagnostic([38, 46], 38, "agendize"),
            diagnostic([48, 62], 48, "per your order"),
            diagnostic([64, 80], 64, "per your request"),
            diagnostic([82, 96], 82, "disincentivize"),
        ],
    );
}

#[test]
fn matching_is_case_insensitive_and_interpolates_surface_text() {
    assert_result(
        "IN THE AFFIRMATIVE; In The Negative; AGENDIZE; Per Your Order; PER YOUR REQUEST; disIncentivize.",
        1,
        vec![
            diagnostic([1, 19], 1, "IN THE AFFIRMATIVE"),
            diagnostic([21, 36], 21, "In The Negative"),
            diagnostic([38, 46], 38, "AGENDIZE"),
            diagnostic([48, 62], 48, "Per Your Order"),
            diagnostic([64, 80], 64, "PER YOUR REQUEST"),
            diagnostic([82, 96], 82, "disIncentivize"),
        ],
    );
}

#[test]
fn longer_word_boundary_near_misses_are_clean() {
    assert_result(
        "xin the affirmativex; yin the negativex; agendizer; xper your orderx; xper your requestx; disincentivizesx.",
        0,
        vec![],
    );
}

#[test]
fn punctuation_boundaries_allow_all_six_phrases() {
    assert_result(
        "(in the affirmative)! [in the negative]? {agendize}; (per your order)! [per your request]? {disincentivize}.",
        1,
        vec![
            diagnostic([2, 20], 2, "in the affirmative"),
            diagnostic([24, 39], 24, "in the negative"),
            diagnostic([43, 51], 43, "agendize"),
            diagnostic([55, 69], 55, "per your order"),
            diagnostic([73, 89], 73, "per your request"),
            diagnostic([93, 107], 93, "disincentivize"),
        ],
    );
}

#[test]
fn quote_suppression_retains_unquoted_occurrences() {
    assert_result(
        "Quoted \"agendize\". Outside per your order, then disincentivize.",
        1,
        vec![
            diagnostic([28, 42], 28, "per your order"),
            diagnostic([49, 63], 49, "disincentivize"),
        ],
    );
}

#[test]
fn repeated_mixed_phrases_preserve_source_order() {
    assert_result(
        "disincentivize; agendize; disincentivize; in the negative.",
        1,
        vec![
            diagnostic([1, 15], 1, "disincentivize"),
            diagnostic([17, 25], 17, "agendize"),
            diagnostic([27, 41], 27, "disincentivize"),
            diagnostic([43, 58], 43, "in the negative"),
        ],
    );
}

#[test]
fn unicode_prefixes_use_codepoint_positions() {
    assert_result(
        "é before in the affirmative; e\u{301} before disincentivize.",
        1,
        vec![
            diagnostic([10, 28], 10, "in the affirmative"),
            diagnostic([40, 54], 40, "disincentivize"),
        ],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good plan may agendize the request and TODO.",
        1,
        vec![diagnostic([22, 30], 22, "agendize")],
    );
}

#[test]
fn valid_phrase_activation_emits_embedded_raw_copy() {
    assert_result(
        "agendize; xagendizey",
        1,
        vec![
            diagnostic([1, 9], 1, "agendize"),
            diagnostic([12, 20], 12, "agendize"),
        ],
    );
}

#[test]
fn unicode_casefold_regression_jargon() {
    assert_result(
        "diſincentivize",
        1,
        vec![
            json!({"check_path":"industrial_language.jargon","message":"'diſincentivize' is jargon. Can you replace it with something more standard?","pos":[1,1],"replacements":null,"span":[1,15]}),
        ],
    );
}
