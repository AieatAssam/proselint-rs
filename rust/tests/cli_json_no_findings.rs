use std::fs;
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
            "proselint-rs-cli-json-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::write(
            path.join("empty checks.json"),
            br#"{"max_errors":1000,"checks":{}}"#,
        )
        .expect("write explicit all-disabled configuration");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_check(workspace: &Workspace, input: Option<&[u8]>, file: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint"));
    command
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "empty checks.json",
            "--output-format",
            "json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(file) = file {
        command.arg(file);
    }
    if input.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn().expect("start native CLI");
    if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input)
            .expect("write test input");
    }
    child.wait_with_output().expect("wait for native CLI")
}

fn assert_clean_json(output: &Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must be valid JSON: {error}; bytes={:?}",
            output.stdout
        )
    });
    assert_eq!(value.as_object().map(serde_json::Map::len), Some(1));
    let result = value
        .get("result")
        .and_then(serde_json::Value::as_object)
        .expect("top-level result object");
    assert_eq!(result.len(), 1, "one source entry is required");
    let source = result.values().next().expect("one source result");
    assert_eq!(source.as_object().map(serde_json::Map::len), Some(1));
    assert_eq!(source.get("diagnostics"), Some(&serde_json::json!([])));
    value
}

#[test]
fn empty_stdin_emits_a_clean_json_result() {
    let workspace = Workspace::new();
    let output = run_check(&workspace, Some(b""), None);
    let value = assert_clean_json(&output);
    assert!(value["result"].get("<stdin>").is_some());
}

#[test]
fn ordinary_no_hit_stdin_emits_a_clean_json_result() {
    let workspace = Workspace::new();
    let output = run_check(
        &workspace,
        Some(b"Ordinary prose without annotation markers.\n"),
        None,
    );
    let value = assert_clean_json(&output);
    assert!(value["result"].get("<stdin>").is_some());
}

#[test]
fn empty_file_with_unicode_and_spaces_emits_a_clean_json_result() {
    let workspace = Workspace::new();
    let source = "draft é blank.txt";
    fs::write(workspace.path().join(source), b"").expect("create empty source file");
    let output = run_check(&workspace, None, Some(source));
    let value = assert_clean_json(&output);
    let result = value["result"].as_object().expect("result map");
    let source_uri = result.keys().next().expect("file source key");
    assert!(
        source_uri.starts_with("file://"),
        "source key: {source_uri}"
    );
    assert!(
        source_uri.ends_with("/draft%20%C3%A9%20blank.txt"),
        "source key: {source_uri}"
    );
}
