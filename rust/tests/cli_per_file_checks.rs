use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-per-file-checks-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"*.txt":{"annotations":{"misc":false}}}}"#,
        ).expect("write config with per-file override");
        fs::write(path.join("draft.txt"), b"TODO").expect("write source file");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn matching_per_file_glob_disables_globally_enabled_annotation() {
    let workspace = Workspace::new();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
            "draft.txt",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI on per-file-selected source");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    let result = value["result"].as_object().expect("result map");
    let source = result.values().next().expect("source result");
    assert_eq!(source["diagnostics"], serde_json::json!([]));
}

fn run_with_config(config: &str, source_name: &str, source_text: &str) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("selected config.json"), config).unwrap();
    fs::write(workspace.0.join(source_name), source_text).unwrap();
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
            source_name,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI with file-specific config")
}

#[test]
fn parent_override_preserves_more_specific_global_child() {
    let output = run_with_config(
        r#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"*.txt":{"annotations":false}}}"#,
        "input.txt",
        "TODO",
    );
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let source = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(
        source["diagnostics"],
        serde_json::json!([{
            "check_path":"annotations.misc", "message":"Annotation left in text.",
            "pos":[1,1], "replacements":null, "span":[1,5]
        }])
    );
}

#[test]
fn bracket_character_class_matches_literal_filename() {
    let output = run_with_config(
        r#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"[i]nput.txt":{"annotations":{"misc":false}}}}"#,
        "input.txt",
        "TODO",
    );
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let source = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(source["diagnostics"], serde_json::json!([]));
}

#[test]
fn first_matching_per_file_pattern_wins_in_insertion_order() {
    let output = run_with_config(
        r#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"*.txt":{"annotations":{"misc":false}},"*":{"annotations":{"misc":true}}}}"#,
        "input.txt",
        "TODO",
    );
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let source = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(source["diagnostics"], serde_json::json!([]));
}
