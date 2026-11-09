use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const MESSAGE: &str = "Substitute 'damn' every time you're inclined to write 'very'; your editor will delete it and the writing will be just as it should be.";

fn run(input: &str, max_errors: usize) -> Value {
    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "proselint-rs-weasel-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("config.json"),
        format!(r#"{{"max_errors":{max_errors},"checks":{{"weasel_words":{{"very":true}}}}}}"#),
    )
    .unwrap();
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
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(dir);
    assert!(out.stderr.is_empty(), "{:?}", out.stderr);
    serde_json::from_slice(&out.stdout).unwrap()
}

fn diagnostic(span: [usize; 2], pos: [usize; 2]) -> Value {
    json!({"check_path":"weasel_words.very","message":MESSAGE,"pos":pos,"replacements":null,"span":span})
}

fn assert_findings(input: &str, max_errors: usize, expected: Vec<Value>) {
    assert_eq!(
        run(input, max_errors),
        json!({"result":{"<stdin>":{"diagnostics":expected}}})
    );
}

#[test]
fn ordinary_case_and_case_insensitive_match() {
    assert_findings(
        "This is very useful. VERY clear.",
        1000,
        vec![diagnostic([9, 13], [1, 9]), diagnostic([22, 26], [1, 22])],
    );
}

#[test]
fn very_well_is_excluded() {
    assert_findings(
        "It went very well; very Wellington was mentioned.",
        1000,
        vec![],
    );
}

#[test]
fn tab_and_newline_positions() {
    assert_findings(
        "This is very\tuseful. It is very\nimportant.",
        1000,
        vec![diagnostic([9, 13], [1, 9]), diagnostic([28, 32], [1, 28])],
    );
}

#[test]
fn unicode_letters_are_word_boundaries() {
    assert_findings("évery useful and very猫 useful.", 1000, vec![]);
}

#[test]
fn quoted_match_is_suppressed() {
    assert_findings("He called it \"very useful\".", 1000, vec![]);
}

#[test]
fn max_errors_truncates_diagnostics() {
    assert_findings(
        "This is very useful and very important.",
        1,
        vec![diagnostic([9, 13], [1, 9])],
    );
}

#[test]
fn combining_marks_and_ascii_boundaries() {
    assert_findings(
        "́very veŕy _very 1very xvery veryx",
        1000,
        vec![diagnostic([2, 6], [1, 2])],
    );
}

#[test]
fn multiline_position_uses_line_and_column() {
    assert_findings(
        "first line\nsecond line very useful",
        1000,
        vec![diagnostic([24, 28], [2, 13])],
    );
}
