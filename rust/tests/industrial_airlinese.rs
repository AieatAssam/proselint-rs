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
            "proselint-rs-airlinese-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"industrial_language":{"airlinese":true}}}"#,
        )
        .expect("write config selecting only industrial_language.airlinese");
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
        "check_path": "industrial_language.airlinese",
        "message": format!("'{matched}' is airlinese."),
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
fn detects_all_eight_enplan_deplan_inflections_with_oracle_surfaces() {
    assert_result(
        "enplane; enplaned; enplaning; enplanement; deplane; deplaned; deplaning; deplanement.",
        1,
        vec![
            diagnostic([1, 8], 1, "enplane"),
            diagnostic([10, 17], 10, "enplane"),
            diagnostic([20, 29], 20, "enplaning"),
            diagnostic([31, 38], 31, "enplane"),
            diagnostic([44, 51], 44, "deplane"),
            diagnostic([53, 60], 53, "deplane"),
            diagnostic([63, 72], 63, "deplaning"),
            diagnostic([74, 81], 74, "deplane"),
        ],
    );
}

#[test]
fn detects_taking_off_momentarily() {
    assert_result(
        "The aircraft is taking off momentarily.",
        1,
        vec![diagnostic([17, 39], 17, "taking off momentarily")],
    );
}

#[test]
fn matching_is_case_insensitive_and_interpolates_surface_text() {
    assert_result(
        "ENPLANED; DePlaning; TAKING OFF MOMENTARILY.",
        1,
        vec![
            diagnostic([1, 8], 1, "ENPLANE"),
            diagnostic([11, 20], 11, "DePlaning"),
            diagnostic([22, 44], 22, "TAKING OFF MOMENTARILY"),
        ],
    );
}

#[test]
fn upstream_hydroplane_and_planing_mill_examples_are_clean() {
    assert_result(
        "Did that car just hydroplane? I know how to operate a planing mill.",
        0,
        vec![],
    );
}

#[test]
fn longer_word_boundary_near_misses_are_clean() {
    assert_result(
        "xenplanedx; xdeplaningx; xtaking off momentarilyx.",
        0,
        vec![],
    );
}

#[test]
fn spacing_near_misses_are_clean() {
    assert_result(
        "taking  off momentarily; taking off  momentarily; enplan ed; deplan ing.",
        0,
        vec![],
    );
}

#[test]
fn punctuation_boundaries_allow_all_patterns() {
    assert_result(
        "(enplaned)! [deplanement]? {taking off momentarily}.",
        1,
        vec![
            diagnostic([2, 9], 2, "enplane"),
            diagnostic([14, 21], 14, "deplane"),
            diagnostic([29, 51], 29, "taking off momentarily"),
        ],
    );
}

#[test]
fn quote_suppression_retains_unquoted_matches() {
    assert_result(
        "Quoted \"enplaning\". Outside deplaned; then taking off momentarily.",
        1,
        vec![
            diagnostic([29, 36], 29, "deplane"),
            diagnostic([44, 66], 44, "taking off momentarily"),
        ],
    );
}

#[test]
fn repeated_mixed_patterns_preserve_source_order() {
    assert_result(
        "taking off momentarily; enplaned; taking off momentarily; deplaning.",
        1,
        vec![
            diagnostic([1, 23], 1, "taking off momentarily"),
            diagnostic([25, 32], 25, "enplane"),
            diagnostic([35, 57], 35, "taking off momentarily"),
            diagnostic([59, 68], 59, "deplaning"),
        ],
    );
}

#[test]
fn unicode_prefixes_use_codepoint_positions() {
    assert_result(
        "é before enplanement; e\u{301} before taking off momentarily.",
        1,
        vec![
            diagnostic([10, 17], 10, "enplane"),
            diagnostic([33, 55], 33, "taking off momentarily"),
        ],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good plan will enplane the team and TODO.",
        1,
        vec![diagnostic([23, 30], 23, "enplane")],
    );
}

#[test]
fn unicode_casefold_regression_airlinese() {
    assert_result(
        "taKing off momentarily",
        1,
        vec![
            json!({"check_path":"industrial_language.airlinese","message":"'taKing off momentarily' is airlinese.","pos":[1,1],"replacements":null,"span":[1,23]}),
        ],
    );
}
