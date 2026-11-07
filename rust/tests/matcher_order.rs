use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(config: &str) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-matcher-order-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(path.join("selected config.json"), config)
            .expect("write selected check config");
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

fn run(input: &str, config: &str) -> Output {
    let workspace = Workspace::new(config);
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

fn diagnostic(
    check: &str,
    message: &str,
    span: [usize; 2],
    column: usize,
    replacement: Value,
) -> Value {
    json!({
        "check_path": check,
        "message": message,
        "pos": [1, column],
        "replacements": replacement,
        "span": span,
    })
}

fn assert_first_two(input: &str, config: &str, expected: Vec<Value>) {
    let output = run(input, config);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":expected}}})
    );
}

#[test]
fn seasons_keep_first_activated_pattern_before_max_errors() {
    let check = "misc.capitalization.seasons";
    let message = "Seasons shouldn't be capitalized. 'winter' is the preferred form.";
    assert_first_two(
        "Winter; Summer; Winter",
        r#"{"max_errors":2,"checks":{"misc":{"capitalization":{"seasons":true}}}}"#,
        vec![
            diagnostic(check, message, [1, 7], 1, json!("winter")),
            diagnostic(check, message, [17, 23], 17, json!("winter")),
        ],
    );
}

#[test]
fn months_keep_first_activated_pattern_before_max_errors() {
    let check = "misc.capitalization.months";
    let message = "Months should be capitalized. 'January' is the preferred form.";
    assert_first_two(
        "january; february; january",
        r#"{"max_errors":2,"checks":{"misc":{"capitalization":{"months":true}}}}"#,
        vec![
            diagnostic(check, message, [1, 8], 1, json!("January")),
            diagnostic(check, message, [20, 27], 20, json!("January")),
        ],
    );
}

#[test]
fn days_keep_first_activated_pattern_before_max_errors() {
    let check = "misc.capitalization.days";
    let message = "Days of the week should be capitalized. 'Monday' is the preferred form.";
    assert_first_two(
        "monday; tuesday; monday",
        r#"{"max_errors":2,"checks":{"misc":{"capitalization":{"days":true}}}}"#,
        vec![
            diagnostic(check, message, [1, 7], 1, json!("Monday")),
            diagnostic(check, message, [18, 24], 18, json!("Monday")),
        ],
    );
}

#[test]
fn capitalization_terms_keep_first_activated_pattern_before_max_errors() {
    let check = "misc.capitalization.terms";
    let message = "Incorrect capitalization. 'Mother Nature' is the preferred form.";
    assert_first_two(
        "mother nature; Space Age; mother nature",
        r#"{"max_errors":2,"checks":{"misc":{"capitalization":{"terms":true}}}}"#,
        vec![
            diagnostic(check, message, [1, 14], 1, json!("Mother Nature")),
            diagnostic(check, message, [27, 40], 27, json!("Mother Nature")),
        ],
    );
}

#[test]
fn debased_keeps_first_activated_pattern_before_max_errors() {
    let check = "misc.debased";
    let message = "Debased language is a continuous temptation.";
    assert_first_two(
        "leaves much to be desired; would serve no purpose; leaves much to be desired",
        r#"{"max_errors":2,"checks":{"misc":{"debased":true}}}"#,
        vec![
            diagnostic(check, message, [1, 26], 1, Value::Null),
            diagnostic(check, message, [52, 77], 52, Value::Null),
        ],
    );
}

#[test]
fn metadiscourse_keeps_first_activated_pattern_before_max_errors() {
    let check = "misc.metadiscourse";
    let message = "Excessive metadiscourse.";
    assert_first_two(
        "This chapter discusses; The rest of this article; This chapter discusses",
        r#"{"max_errors":2,"checks":{"misc":{"metadiscourse":true}}}"#,
        vec![
            diagnostic(check, message, [1, 23], 1, Value::Null),
            diagnostic(check, message, [51, 73], 51, Value::Null),
        ],
    );
}

#[test]
fn many_a_keeps_first_activated_pattern_before_max_errors() {
    let check = "misc.many_a";
    let message = "'many a' requires a singular verb.";
    assert_first_two(
        "are many a; were many a; are many a",
        r#"{"max_errors":2,"checks":{"misc":{"many_a":true}}}"#,
        vec![
            diagnostic(check, message, [1, 11], 1, json!("is many a")),
            diagnostic(check, message, [26, 36], 26, json!("is many a")),
        ],
    );
}

#[test]
fn greylist_preserves_upstream_dictionary_order_before_max_errors() {
    let check = "misc.greylist";
    let message = "Use of 'obviously'. This is obviously an inadvisable word to use.";
    assert_first_two(
        "utilize; obviously; obviously",
        r#"{"max_errors":2,"checks":{"misc":{"greylist":true}}}"#,
        vec![
            diagnostic(check, message, [10, 19], 10, Value::Null),
            diagnostic(check, message, [21, 30], 21, Value::Null),
        ],
    );
}
