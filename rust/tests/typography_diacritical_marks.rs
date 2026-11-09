use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(max_errors: usize) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-typography-diacritical-marks-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"typography":{{"diacritical_marks":true}}}}}}"#),
        )
        .expect("write config selecting typography.diacritical_marks");
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

fn run_check(input: &str, max_errors: usize) -> Output {
    let workspace = Workspace::new(max_errors);
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

fn diagnostic(span: [usize; 2], column: usize, preferred: &str) -> Value {
    json!({
        "check_path": "typography.diacritical_marks",
        "message": format!("Use diacritical marks in '{preferred}'."),
        "pos": [1, column],
        "replacements": preferred,
        "span": span,
    })
}

fn assert_result(input: &str, expected_exit: i32, diagnostics: Vec<Value>) {
    let output = run_check(input, 1000);
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
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}

#[test]
fn quote_embedded_and_limit_one_match_pinned_output() {
    let output = run_check("“cafe” Beyonce xcafe", 1);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[diagnostic([17,21],17,"café")]}}})
    );
}

#[test]
fn overlapping_senor_forms_respect_limit_one() {
    let output = run_check("senorita; senora; senor", 1);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[diagnostic([1,9],1,"señorita")]}}})
    );
}

#[test]
fn overlapping_senor_forms_respect_limit_two() {
    let output = run_check("senorita; senora; senor", 2);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[
            diagnostic([1,9],1,"señorita"), diagnostic([11,17],11,"señora")
        ]}}})
    );
}

#[test]
fn all_unaccented_source_keys_match_pinned_diagnostic_count() {
    let output = run_check(include_str!("fixtures/diacriticals_all_keys.txt"), 1000);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual["result"]["<stdin>"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        86
    );
}

#[test]
fn all_preferred_targets_are_clean() {
    let output = run_check(include_str!("fixtures/diacriticals_all_targets.txt"), 1000);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(actual, json!({"result":{"<stdin>":{"diagnostics":[]}}}));
}

#[test]
fn ligature_fiance_matches_pinned_output() {
    assert_result("ﬁance; fiance", 1, vec![diagnostic([8, 14], 8, "fiancé")]);
}

#[test]
fn dotted_i_fiance_matches_pinned_output() {
    assert_result("fİance; fiance", 1, vec![diagnostic([9, 15], 9, "fiancé")]);
}

#[test]
fn replaces_beyonce_with_preferred_accent_in_message_and_replacement() {
    assert_result("Beyonce", 1, vec![diagnostic([1, 8], 1, "Beyoncé")]);
}

#[test]
fn uppercase_match_uses_preferred_spelling() {
    assert_result("BEYONCE", 1, vec![diagnostic([1, 8], 1, "Beyoncé")]);
}

#[test]
fn leaves_embedded_ascii_and_already_accented_forms_alone() {
    assert_result("xBeyonceish; Beyoncé", 0, vec![]);
    assert_result("xBeyonceish", 0, vec![]);
}

#[test]
fn raw_occurrences_are_reported_after_the_pattern_activates() {
    assert_result(
        "xBeyonceish; Beyonce",
        1,
        vec![
            diagnostic([2, 9], 2, "Beyoncé"),
            diagnostic([14, 21], 14, "Beyoncé"),
        ],
    );
}

#[test]
fn uses_ascii_word_boundaries_beside_non_ascii_letters() {
    assert_result(
        "éBeyonce; Beyonceé",
        1,
        vec![
            diagnostic([2, 9], 2, "Beyoncé"),
            diagnostic([11, 18], 11, "Beyoncé"),
        ],
    );
}

#[test]
fn applies_the_upstream_activation_and_raw_matching_casefolds() {
    assert_result("ſkoda", 0, vec![]);
    let output = run_check("skoda; ſkoda", 1000);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[
            diagnostic([1,6],1,"Škoda"),
            {"check_path":"typography.diacritical_marks","message":"Use diacritical marks in 'None'.","span":[8,13],"pos":[1,8],"replacements":null}
        ]}}})
    );
}

#[test]
fn preferred_lookup_lowercases_kelvin_sign() {
    assert_result("sKoda", 1, vec![diagnostic([1, 6], 1, "Škoda")]);
}

#[test]
fn reports_french_spanish_and_portuguese_names_in_source_order() {
    assert_result(
        "creme brulee; senor; acai",
        1,
        vec![
            diagnostic([1, 13], 1, "crème brûlée"),
            diagnostic([15, 20], 15, "señor"),
            diagnostic([22, 26], 22, "açaí"),
        ],
    );
}

#[test]
fn suppresses_matches_inside_quotes() {
    assert_result(
        "“Beyonce” and Beyonce",
        1,
        vec![diagnostic([15, 22], 15, "Beyoncé")],
    );
}

#[test]
fn positions_follow_unicode_codepoints() {
    assert_result("é Beyonce", 1, vec![diagnostic([3, 10], 3, "Beyoncé")]);
}

#[test]
fn max_errors_one_returns_only_first_diagnostic() {
    let output = run_check("Beyonce cafe", 1);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[diagnostic([1,8],1,"Beyoncé")]}}})
    );
}

#[test]
fn max_errors_preserves_pattern_group_order_before_truncation() {
    let output = run_check("cafe x Beyonce cafe", 2);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[
            diagnostic([1,5],1,"café"),
            diagnostic([16,20],16,"café")
        ]}}})
    );
}

#[test]
fn preserves_the_upstream_typo_lookup_key() {
    assert_result(
        "creme fraice",
        1,
        vec![diagnostic([1, 13], 1, "crème fraîche")],
    );
}
