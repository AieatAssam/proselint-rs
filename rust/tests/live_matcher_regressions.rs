use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(input: &str, config: &str) -> Value {
    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "proselint-rs-live-regression-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("config.json"), config).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint"))
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

fn spans(result: Value) -> Vec<Value> {
    result["result"]["<stdin>"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| json!([d["check_path"], d["span"]]))
        .collect()
}

#[test]
fn existence_checks_group_activated_patterns_before_limit() {
    let jargon = run(
        "agendize; disincentivize; agendize",
        r#"{"max_errors":2,"checks":{"industrial_language":{"jargon":true}}}"#,
    );
    assert_eq!(
        spans(jargon),
        vec![
            json!(["industrial_language.jargon", [1, 9]]),
            json!(["industrial_language.jargon", [27, 35]])
        ]
    );
    let hedging = run(
        "I would argue that; to a certain degree; I would argue that",
        r#"{"max_errors":2,"checks":{"hedging":true}}"#,
    );
    assert_eq!(
        spans(hedging),
        vec![json!(["hedging", [1, 19]]), json!(["hedging", [42, 60]])]
    );
}

#[test]
fn very_uses_pinned_python_word_membership() {
    for (prefix, suffix) in [
        ("\u{1c89}", ""),
        ("\u{0345}", ""),
        ("", "\u{0345}"),
        ("", "\u{05b0}"),
    ] {
        assert_eq!(
            spans(run(
                &format!("{prefix}very{suffix}"),
                r#"{"max_errors":1000,"checks":{"weasel_words":{"very":true}}}"#
            )),
            vec![json!([
                "weasel_words.very",
                [prefix.chars().count() + 1, prefix.chars().count() + 5]
            ])]
        );
    }
    for text in [
        "_very",
        "very_",
        "1very",
        "very2",
        "xvery",
        "veryx",
        "ver\u{0301}y",
    ] {
        assert!(
            spans(run(
                text,
                r#"{"max_errors":1000,"checks":{"weasel_words":{"very":true}}}"#
            ))
            .is_empty(),
            "{text:?}"
        );
    }
    for text in ["x‿very", "very‿x", "\u{0301}very", "very\u{0301}"] {
        assert_eq!(
            spans(run(
                text,
                r#"{"max_errors":1000,"checks":{"weasel_words":{"very":true}}}"#
            ))
            .len(),
            1,
            "{text:?}"
        );
    }
    assert_eq!(
        spans(run(
            "very",
            r#"{"max_errors":1000,"checks":{"weasel_words":{"very":true}}}"#
        )),
        vec![json!(["weasel_words.very", [1, 5]])]
    );
}
