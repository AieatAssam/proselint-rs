use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(input: &str, max_errors: usize) -> (i32, Value) {
    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "proselint-rs-bottleneck-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).expect("create test workspace");
    std::fs::write(
        dir.join("config.json"),
        format!(
            r#"{{"max_errors":{max_errors},"checks":{{"mixed_metaphors":{{"bottleneck":true}}}}}}"#
        ),
    )
    .expect("write test config");
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&dir)
        .args([
            "check",
            "--config",
            "config.json",
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
    let output = child.wait_with_output().expect("wait for native CLI");
    let _ = std::fs::remove_dir_all(dir);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    (
        output.status.code().expect("process exit code"),
        serde_json::from_slice(&output.stdout).expect("valid JSON output"),
    )
}

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "mixed_metaphors.bottleneck",
        "message": "Mixed metaphor — bottles with big necks are easy to pass through.",
        "pos": [1, column],
        "replacements": null,
        "span": span,
    })
}

fn assert_case(input: &str, max_errors: usize, exit: i32, expected: Vec<Value>) {
    let (actual_exit, actual) = run(input, max_errors);
    assert_eq!(actual_exit, exit);
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":expected}}})
    );
}

#[test]
fn all_seven_phrases_match_pinned_output() {
    assert_case(
        "biggest bottleneck; big bottleneck; large bottleneck; largest bottleneck; world-wide bottleneck; huge bottleneck; massive bottleneck",
        100,
        1,
        [
            ([1, 19], 1),
            ([21, 35], 21),
            ([37, 53], 37),
            ([55, 73], 55),
            ([75, 96], 75),
            ([98, 113], 98),
            ([115, 133], 115),
        ]
        .into_iter()
        .map(|(span, col)| diagnostic(span, col))
        .collect(),
    );
}

#[test]
fn case_duplicates_and_overlapping_phrases_match_pinned_order() {
    assert_case(
        "BIG BOTTLENECK, biggest bottleneck, and biggest bottleneck.",
        100,
        1,
        [([1, 15], 1), ([17, 35], 17), ([41, 59], 41)]
            .into_iter()
            .map(|(span, col)| diagnostic(span, col))
            .collect(),
    );
}

#[test]
fn clean_quoted_limited_and_unicode_cases_match_pinned_outputs() {
    assert_case(
        "bottleneck; bigger bottleneck; bottlenecks; world wide bottleneck; huge-bottleneck.",
        100,
        0,
        vec![],
    );
    assert_case("They called it \"huge bottleneck\".", 100, 0, vec![]);
    assert_case(
        "big bottleneck; large bottleneck; massive bottleneck.",
        1,
        1,
        vec![diagnostic([1, 15], 1)],
    );
    assert_case(
        "é😀 A world-wide bottleneck.",
        100,
        1,
        vec![diagnostic([6, 27], 6)],
    );
}

#[test]
fn max_errors_preserves_pinned_phrase_group_order() {
    assert_case(
        "big bottleneck; massive bottleneck; big bottleneck.",
        2,
        1,
        vec![diagnostic([1, 15], 1), diagnostic([37, 51], 37)],
    );
    assert_case(
        "massive bottleneck; big bottleneck.",
        1,
        1,
        vec![diagnostic([1, 19], 1)],
    );
}
