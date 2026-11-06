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
            "proselint-rs-ance-ence-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(
                r#"{{"max_errors":{max_errors},"checks":{{"spelling":{{"ance_ence":true}}}}}}"#
            ),
        )
        .expect("write config selecting only spelling.ance_ence");
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

fn expected(span: [usize; 2], replacement: &str) -> Value {
    json!({
        "check_path":"spelling.ance_ence",
        "message":format!("-ance vs. -ence. '{replacement}' is the correct spelling."),
        "pos":[1,span[0]],
        "replacements":replacement,
        "span":span,
    })
}

fn assert_result(input: &str, max_errors: usize, expected_exit: i32, diagnostics: Vec<Value>) {
    let output = run_check(input, max_errors);
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
fn recognizes_each_misspelling_and_correct_forms_are_clean() {
    assert_result(
        "appearence occurrance resistence",
        1000,
        1,
        vec![
            expected([1, 11], "appearance"),
            expected([12, 22], "occurrence"),
            expected([23, 33], "resistance"),
        ],
    );
    assert_result("appearance occurrence resistance", 1000, 0, vec![]);
}

#[test]
fn matches_case_insensitively_and_reports_replacement() {
    assert_result(
        "ApPeArEnCe OCCURRANCE ReSiStEnCe",
        1000,
        1,
        vec![
            expected([1, 11], "appearance"),
            expected([12, 22], "occurrence"),
            expected([23, 33], "resistance"),
        ],
    );
}

#[test]
fn word_boundary_controls_reject_longer_forms() {
    assert_result("xappearence appearencex", 1000, 0, vec![]);
}

#[test]
fn suppresses_quoted_matches() {
    assert_result(
        "\"appearence\" and appearence",
        1000,
        1,
        vec![expected([18, 28], "appearance")],
    );
}

#[test]
fn emits_repeated_matches_in_pattern_set_order() {
    assert_result(
        "resistence appearence occurrance appearence",
        1000,
        1,
        vec![
            expected([1, 11], "resistance"),
            expected([12, 22], "appearance"),
            expected([23, 33], "occurrence"),
            expected([34, 44], "appearance"),
        ],
    );
}

#[test]
fn max_errors_truncates_in_pattern_order() {
    assert_result(
        "resistence appearence",
        1,
        1,
        vec![expected([1, 11], "resistance")],
    );
}

#[test]
fn positions_count_unicode_codepoints() {
    assert_result(
        "😀 café appearence",
        1000,
        1,
        vec![expected([8, 18], "appearance")],
    );
}

#[test]
fn selecting_this_check_does_not_enable_unrelated_checks() {
    assert_result(
        "appearence; p = 0.00",
        1000,
        1,
        vec![expected([1, 11], "appearance")],
    );
}

#[test]
fn a_later_boundary_match_can_activate_an_earlier_embedded_substring() {
    assert_result(
        "appearencex appearence",
        1000,
        1,
        vec![
            expected([1, 11], "appearance"),
            expected([13, 23], "appearance"),
        ],
    );
}
