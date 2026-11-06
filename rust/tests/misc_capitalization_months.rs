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
            "proselint-rs-months-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"capitalization":{"months":true}}}}"#,
        )
        .expect("write selected config");
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
    child.wait_with_output().expect("wait for CLI")
}

fn diagnostic(span: [usize; 2], month: &str) -> Value {
    let replacement = {
        let mut chars = month.chars();
        chars
            .next()
            .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
            .expect("month is nonempty")
    };
    json!({
        "check_path":"misc.capitalization.months",
        "message":format!("Months should be capitalized. '{replacement}' is the preferred form."),
        "pos":[1,span[0]],
        "replacements":replacement,
        "span":span
    })
}

fn assert_result(input: &str, exit: i32, diagnostics: Vec<Value>) {
    let out = run_check(input);
    assert_eq!(out.status.code(), Some(exit), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}

#[test]
fn maps_all_ten_lowercase_month_names() {
    assert_result(
        "january february april june july august september october november december",
        1,
        vec![
            diagnostic([1, 8], "january"),
            diagnostic([9, 17], "february"),
            diagnostic([18, 23], "april"),
            diagnostic([24, 28], "june"),
            diagnostic([29, 33], "july"),
            diagnostic([34, 40], "august"),
            diagnostic([41, 50], "september"),
            diagnostic([51, 58], "october"),
            diagnostic([59, 67], "november"),
            diagnostic([68, 76], "december"),
        ],
    );
}

#[test]
fn correct_case_and_unlisted_march_and_may_are_clean() {
    assert_result("January FEBRUARY March march MAY may wednesday", 0, vec![]);
}

#[test]
fn boundary_term_activates_raw_substring_matches() {
    assert_result("xjanuary", 0, vec![]);
    assert_result(
        "january xjanuary",
        1,
        vec![
            diagnostic([1, 8], "january"),
            diagnostic([10, 17], "january"),
        ],
    );
}

#[test]
fn quotes_are_suppressed_and_repeated_matches_keep_source_order() {
    assert_result(
        "january \"february\" april january",
        1,
        vec![
            diagnostic([1, 8], "january"),
            diagnostic([20, 25], "april"),
            diagnostic([26, 33], "january"),
        ],
    );
}

#[test]
fn unicode_prefixes_keep_python_codepoint_coordinates() {
    assert_result(
        "é january e\u{301} february 😀 april",
        1,
        vec![
            diagnostic([3, 10], "january"),
            diagnostic([14, 22], "february"),
            diagnostic([25, 30], "april"),
        ],
    );
}
