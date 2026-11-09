use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn diagnostics(input: &str) -> Vec<Value> {
    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let workspace = std::env::temp_dir().join(format!(
        "proselint-chatspeak-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(
        workspace.join("config.json"),
        br#"{"max_errors":1000,"checks":{"industrial_language":{"chatspeak":true}}}"#,
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace)
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
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    std::fs::remove_dir_all(workspace).unwrap();
    assert!(output.status.success() || output.status.code() == Some(1));
    assert!(output.stderr.is_empty());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    actual["result"]["<stdin>"]["diagnostics"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn emits_exact_chatspeak_diagnostic() {
    assert_eq!(
        diagnostics("lol"),
        vec![
            json!({
                "check_path":"industrial_language.chatspeak",
                "message":"'lol' is chatspeak. Write it out.",
                "span":[1,4], "pos":[1,1], "replacements":null
            }),
            json!({
                "check_path":"industrial_language.chatspeak",
                "message":"'lol' is chatspeak. Write it out.",
                "span":[1,4], "pos":[1,1], "replacements":null
            })
        ]
    );
}

#[test]
fn preserves_tuple_order_repeated_afk_and_case_insensitive_case_variants() {
    let actual = diagnostics("LOL AFK lol afk");
    assert_eq!(
        actual
            .iter()
            .map(|d| d["message"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "'LOL' is chatspeak. Write it out.",
            "'LOL' is chatspeak. Write it out.",
            "'AFK' is chatspeak. Write it out.",
            "'AFK' is chatspeak. Write it out.",
            "'lol' is chatspeak. Write it out.",
            "'lol' is chatspeak. Write it out.",
            "'afk' is chatspeak. Write it out.",
            "'afk' is chatspeak. Write it out."
        ]
    );
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![
            json!([1, 4]),
            json!([1, 4]),
            json!([5, 8]),
            json!([5, 8]),
            json!([9, 12]),
            json!([9, 12]),
            json!([13, 16]),
            json!([13, 16])
        ]
    );
}

#[test]
fn word_boundaries_activate_then_raw_occurrences_are_reported() {
    let actual = diagnostics("xlol lolly lol");
    assert_eq!(actual.len(), 6);
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![
            json!([2, 5]),
            json!([2, 5]),
            json!([6, 9]),
            json!([6, 9]),
            json!([12, 15]),
            json!([12, 15])
        ]
    );
}

#[test]
fn suppresses_findings_inside_paired_quotes() {
    let actual = diagnostics("\"lol\" lol");
    assert_eq!(actual.len(), 2);
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![json!([7, 10]), json!([7, 10])]
    );
}

#[test]
fn source_position_orders_groups_before_tuple_order() {
    let actual = diagnostics("XOXO 2day");
    assert_eq!(actual[0]["message"], "'XOXO' is chatspeak. Write it out.");
}
