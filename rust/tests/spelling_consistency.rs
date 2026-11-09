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
            "proselint-rs-consistency-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"spelling":{"consistency":true}}}"#,
        )
        .expect("write config selecting spelling.consistency");
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

fn diagnostics(input: &str) -> Vec<Value> {
    let output = run_check(input);
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "unexpected CLI status {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    actual["result"]["<stdin>"]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .clone()
}

#[test]
fn equal_counts_favor_the_right_hand_spelling() {
    assert_eq!(
        diagnostics("advisor; adviser"),
        vec![json!({
            "check_path": "spelling.consistency",
            "message": "Inconsistent spelling of 'adviser' (vs. 'advisor').",
            "pos": [1, 1],
            "replacements": "adviser",
            "span": [1, 8],
        })]
    );
}

#[test]
fn reports_the_less_frequent_spelling_case_insensitively() {
    assert_eq!(
        diagnostics("advisor; adviser; ADVISER."),
        vec![json!({
            "check_path": "spelling.consistency",
            "message": "Inconsistent spelling of 'adviser' (vs. 'advisor').",
            "pos": [1, 1],
            "replacements": "adviser",
            "span": [1, 8],
        })]
    );
}

#[test]
fn quoted_occurrences_count_before_their_findings_are_suppressed() {
    let actual = diagnostics("advisor advisor; “adviser” adviser");
    assert_eq!(actual.len(), 2);
    assert!(actual.iter().all(|item| {
        item["message"] == "Inconsistent spelling of 'adviser' (vs. 'advisor')."
            && item["replacements"] == "adviser"
    }));
    assert_eq!(actual[0]["span"], json!([1, 8]));
    assert_eq!(actual[1]["span"], json!([9, 16]));
}

#[test]
fn applies_word_boundaries_and_preserves_case_and_unicode_coordinates() {
    assert_eq!(
        diagnostics("xadvisor advisorX ADVISOR; ADVISER."),
        vec![json!({
            "check_path": "spelling.consistency",
            "message": "Inconsistent spelling of 'adviser' (vs. 'ADVISOR').",
            "pos": [1, 19],
            "replacements": "adviser",
            "span": [19, 26],
        })]
    );
    let unicode = diagnostics("é advisor,\nthen ADVISER; colour and COLOR.");
    assert_eq!(unicode.len(), 2);
    assert_eq!(unicode[0]["pos"], json!([1, 3]));
    assert_eq!(unicode[0]["span"], json!([3, 10]));
    assert_eq!(unicode[1]["pos"], json!([2, 15]));
    assert_eq!(unicode[1]["span"], json!([26, 32]));
}

#[test]
fn reports_all_pairs_in_upstream_order_and_leaves_single_variant_clean() {
    let all_pairs = diagnostics(
        "advisor and adviser; analyse and analyze; centre and center; chocolatey and chocolaty; colour and color; emphasise and emphasize; finalise and finalize; focussed and focused; labour and labor; learnt and learned; organise and organize; organised and organized; organising and organizing; recognise and recognize.",
    );
    assert_eq!(all_pairs.len(), 14);
    assert_eq!(
        all_pairs
            .iter()
            .map(|item| item["replacements"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "adviser",
            "analyze",
            "center",
            "chocolaty",
            "color",
            "emphasize",
            "finalize",
            "focused",
            "labor",
            "learned",
            "organize",
            "organized",
            "organizing",
            "recognize",
        ]
    );
    assert!(diagnostics("advisory; analyzer; centered; ordinary prose.").is_empty());
}
