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
            "proselint-rs-currency-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"currency":true}}}"#,
        )
        .expect("write config selecting only misc.currency");
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
        "check_path": "misc.currency",
        "message": format!("Incorrect use of symbols in {matched}."),
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
fn matches_suffix_without_digits_and_interpolates_substring() {
    assert_result(
        "The notation is $dollars.",
        1,
        vec![diagnostic([17, 25], 17, "$dollars")],
    );
}

#[test]
fn matches_one_and_multiple_digits_with_exact_interpolated_messages() {
    assert_result(
        "Price $1 usd; total $123 dollars.",
        1,
        vec![
            diagnostic([7, 13], 7, "$1 usd"),
            diagnostic([21, 33], 21, "$123 dollars"),
        ],
    );
}

#[test]
fn matches_all_suffixes_case_insensitively() {
    assert_result(
        "$usd $USD $UsD $us dollars $US DOLLARS $dollars",
        1,
        vec![
            diagnostic([1, 5], 1, "$usd"),
            diagnostic([6, 10], 6, "$USD"),
            diagnostic([11, 15], 11, "$UsD"),
            diagnostic([16, 27], 16, "$us dollars"),
            diagnostic([28, 39], 28, "$US DOLLARS"),
            diagnostic([40, 48], 40, "$dollars"),
        ],
    );
}

#[test]
fn allows_zero_or_one_ascii_space_but_not_two_or_a_tab() {
    assert_result(
        " $ dollars; $10 dollars; $10  dollars; $10\tdollars",
        1,
        vec![
            diagnostic([2, 11], 2, "$ dollars"),
            diagnostic([13, 24], 13, "$10 dollars"),
        ],
    );
}

#[test]
fn leading_nonboundary_requires_nonword_before_dollar() {
    assert_result(
        "$usd ($usd) _$usd x$usd",
        1,
        vec![
            diagnostic([1, 5], 1, "$usd"),
            diagnostic([7, 11], 7, "$usd"),
        ],
    );
}

#[test]
fn suffix_word_boundary_rejects_letter_and_underscore_adjacency() {
    assert_result(
        "$usd! $usdx $usd_ $usd!",
        1,
        vec![
            diagnostic([1, 5], 1, "$usd"),
            diagnostic([19, 23], 19, "$usd"),
        ],
    );
}

#[test]
fn quoted_currency_occurrence_is_suppressed() {
    assert_result("Quoted: \"$10 dollars\".", 0, vec![]);
}

#[test]
fn re2_ascii_digit_and_non_ascii_word_boundary_behavior_is_preserved() {
    assert_result(
        "$١ dollars $1 dollars ١$dollars",
        1,
        vec![
            diagnostic([12, 22], 12, "$1 dollars"),
            diagnostic([24, 32], 24, "$dollars"),
        ],
    );
}

#[test]
fn multiple_suffixes_preserve_source_order() {
    assert_result(
        "$10 usd; $5 dollars; $us dollars.",
        1,
        vec![
            diagnostic([1, 8], 1, "$10 usd"),
            diagnostic([10, 20], 10, "$5 dollars"),
            diagnostic([22, 33], 22, "$us dollars"),
        ],
    );
}

#[test]
fn unicode_prefix_uses_codepoint_positions() {
    assert_result(
        "é before $10 dollars.",
        1,
        vec![diagnostic([10, 21], 10, "$10 dollars")],
    );
}

#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good price is $10 dollars and TODO.",
        1,
        vec![diagnostic([22, 33], 22, "$10 dollars")],
    );
}

#[test]
fn unicode_casefold_regression_currency() {
    assert_result(
        "$1 uſd",
        1,
        vec![
            json!({"check_path":"misc.currency","message":"Incorrect use of symbols in $1 uſd.","pos":[1,1],"replacements":null,"span":[1,7]}),
        ],
    );
}
