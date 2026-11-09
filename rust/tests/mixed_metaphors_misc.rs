use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(input: &str, max_errors: usize) -> (i32, Value, Vec<u8>) {
    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "proselint-rs-mixed-metaphors-misc-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).expect("create isolated test workspace");
    std::fs::write(
        dir.join("config.json"),
        format!(r#"{{"max_errors":{max_errors},"checks":{{"mixed_metaphors":{{"misc":true}}}}}}"#),
    )
    .expect("write selected config");
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
        .expect("write input");
    let output = child.wait_with_output().expect("wait for CLI");
    let _ = std::fs::remove_dir_all(dir);
    let value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    (output.status.code().unwrap_or(-1), value, output.stderr)
}

fn diagnostic(span: [usize; 2], column: usize, message: &str, replacement: Value) -> Value {
    json!({
        "check_path": "mixed_metaphors.misc",
        "message": message,
        "pos": [1, column],
        "replacements": replacement,
        "span": span,
    })
}

fn assert_case(input: &str, max_errors: usize, exit: i32, expected: Vec<Value>) {
    let (actual_exit, actual, stderr) = run(input, max_errors);
    let stderr = std::str::from_utf8(&stderr).unwrap_or("stderr was not UTF-8");
    assert_eq!(actual_exit, exit, "stderr: {stderr}");
    assert!(stderr.is_empty(), "stderr: {stderr}");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":expected}}})
    );
}

#[test]
fn all_pinned_items_and_regex_alternatives_match() {
    assert_case(
        "cream rises to the crop; button your seatbelts; a minute to decompose; sharpest marble in the shed; sharpest marble in the box; not rocket surgery.",
        100,
        1,
        vec![
            diagnostic(
                [1, 24],
                1,
                "Mixed metaphor. Try 'cream rises to the top'.",
                json!("cream rises to the top"),
            ),
            diagnostic(
                [26, 47],
                26,
                "Mixed metaphor. Try 'fasten your seatbelts'.",
                json!("fasten your seatbelts"),
            ),
            diagnostic(
                [49, 70],
                49,
                "Mixed metaphor. Try 'a minute to decompress'.",
                json!("a minute to decompress"),
            ),
            diagnostic([72, 99], 72, "Mixed metaphor. Try 'None'.", Value::Null),
            diagnostic([101, 127], 101, "Mixed metaphor. Try 'None'.", Value::Null),
            diagnostic(
                [129, 147],
                129,
                "Mixed metaphor. Try 'not rocket science'.",
                json!("not rocket science"),
            ),
        ],
    );
}

#[test]
fn case_duplicates_and_unicode_positions_match() {
    assert_case(
        "CREAM rises to the crop; not rocket surgery; NOT ROCKET SURGERY; a minute to decompose.",
        100,
        1,
        vec![
            diagnostic(
                [1, 24],
                1,
                "Mixed metaphor. Try 'cream rises to the top'.",
                json!("cream rises to the top"),
            ),
            diagnostic(
                [26, 44],
                26,
                "Mixed metaphor. Try 'not rocket science'.",
                json!("not rocket science"),
            ),
            diagnostic(
                [46, 64],
                46,
                "Mixed metaphor. Try 'not rocket science'.",
                json!("not rocket science"),
            ),
            diagnostic(
                [66, 87],
                66,
                "Mixed metaphor. Try 'a minute to decompress'.",
                json!("a minute to decompress"),
            ),
        ],
    );
    assert_case(
        "é😀 cream rises to the crop.",
        100,
        1,
        vec![diagnostic(
            [4, 27],
            4,
            "Mixed metaphor. Try 'cream rises to the top'.",
            json!("cream rises to the top"),
        )],
    );
}

#[test]
fn max_errors_preserves_pinned_active_pattern_group_order() {
    assert_case(
        "not rocket surgery; cream rises to the crop; not rocket surgery",
        2,
        1,
        vec![
            diagnostic(
                [1, 19],
                1,
                "Mixed metaphor. Try 'not rocket science'.",
                json!("not rocket science"),
            ),
            diagnostic(
                [46, 64],
                46,
                "Mixed metaphor. Try 'not rocket science'.",
                json!("not rocket science"),
            ),
        ],
    );
}

#[test]
fn correct_forms_near_misses_and_quoted_phrases_are_clean() {
    assert_case(
        "cream rises to the top; fasten your seatbelts; a minute to decompress; sharpest tool in the shed; sharpest marble in the barn; not rocket science; not rocket surgeon; rocket surgery.",
        100,
        0,
        vec![],
    );
    assert_case(
        "They wrote \"not rocket surgery\" and 'cream rises to the crop'.",
        100,
        0,
        vec![],
    );
}
