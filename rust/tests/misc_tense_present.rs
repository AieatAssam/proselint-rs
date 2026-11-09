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
            "proselint-rs-tense-present-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(
                r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"tense_present":true}}}}}}"#
            ),
        )
        .expect("write config selecting misc.tense_present");
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

fn run(input: &str, max_errors: usize) -> Output {
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
        .expect("write stdin");
    child.wait_with_output().expect("wait for CLI")
}

fn diagnostic(input: &str, matched: &str, search_from: usize) -> (Value, usize) {
    let byte_start = input[search_from..]
        .find(matched)
        .map(|offset| offset + search_from)
        .expect("oracle-observed match occurs in input");
    let prefix = &input[..byte_start];
    let start = prefix.chars().count() + 1;
    let line = prefix.matches('\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    let end = start + matched.chars().count();
    (
        json!({
            "check_path": "misc.tense_present",
            "message": format!("'{matched}'."),
            "pos": [line, column],
            "replacements": null,
            "span": [start, end],
        }),
        byte_start + matched.len(),
    )
}

fn assert_result(input: &str, max_errors: usize, expected: Vec<Value>) {
    let output = run(input, max_errors);
    assert_eq!(
        output.status.code(),
        Some(if expected.is_empty() { 0 } else { 1 }),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": expected}}})
    );
}

fn expected_matches(input: &str, matches: &[&str]) -> Vec<Value> {
    let mut search_from = 0;
    matches
        .iter()
        .map(|matched| {
            let (diagnostic, next) = diagnostic(input, matched, search_from);
            search_from = next;
            diagnostic
        })
        .collect()
}

// Calibrated against the pinned Python CLI with only misc.tense_present
// enabled. The first pattern preserves both literal Unicode escapes and its
// optional trailing non-word character in the raw match/message.
#[test]
fn all_ten_active_patterns_match_with_exact_raw_messages() {
    let input = "UP TO 10%—OR MORE!; Between you and I; on accident; somewhat of a; all it's own; reason is because; audible to the ear; in regards to; would of; I am feeling nauseous";
    assert_result(
        input,
        1000,
        expected_matches(
            input,
            &[
                "UP TO 10%—OR MORE!",
                "Between you and I",
                "on accident",
                "somewhat of a",
                "all it's own",
                "reason is because",
                "audible to the ear",
                "in regards to",
                "would of",
                "I am feeling nauseous",
            ],
        ),
    );
}

#[test]
fn escaped_em_dash_and_en_dash_alternatives_are_preserved() {
    let input = "up to 10%—or more; up to 3%–and more; up to 8%-or more";
    assert_result(
        input,
        1000,
        expected_matches(
            input,
            &[
                "up to 10%—or more;",
                "up to 3%–and more;",
                "up to 8%-or more",
            ],
        ),
    );
}

#[test]
fn word_activation_quotes_and_near_misses_match_the_pinned_check() {
    assert_result(
        "\"on accident\" ON ACCIDENT; myon accidentally; up to 1000% or more; and so",
        1000,
        expected_matches(
            "\"on accident\" ON ACCIDENT; myon accidentally; up to 1000% or more; and so",
            &["ON ACCIDENT", "on accident"],
        ),
    );
}

#[test]
fn unicode_prefix_uses_python_codepoint_columns() {
    assert_result(
        "😀 would of\nup to 6%–and more.",
        1000,
        expected_matches(
            "😀 would of\nup to 6%–and more.",
            &["would of", "up to 6%–and more."],
        ),
    );
}

#[test]
fn max_errors_truncates_after_the_earliest_match_order() {
    let input = "would of; between you and I; would of; on accident";
    // Fast Existence retains pattern-group order: the `would of` group emits
    // both raw hits before the earlier-in-text hit for the next pattern.
    assert_result(input, 2, expected_matches(input, &["would of", "would of"]));
}
