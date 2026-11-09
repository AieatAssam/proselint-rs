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
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
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

#[test]
fn colon_in_file_uri_is_percent_encoded() {
    let workspace = Workspace::new();
    fs::write(workspace.path().join("a:b.txt"), b"ordinary text").unwrap();
    let output = run_check(&workspace, None, Some("a:b.txt"));
    let value = assert_clean_json(&output);
    let result = value["result"].as_object().unwrap();
    let key = result.keys().next().unwrap();
    assert!(key.ends_with("/a%3Ab.txt"), "source key: {key}");
}

#[test]
fn missing_source_is_a_per_file_json_error_with_success_exit() {
    let workspace = Workspace::new();
    let output = run_check(&workspace, None, Some("absent.txt"));
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("JSON per-file error");
    let result = value["result"].as_object().expect("result object");
    let (_, source_result) = result.iter().next().expect("one missing source entry");
    assert_eq!(source_result["error"]["code"], -31998);
    let expected_path = workspace.path().join("absent.txt");
    assert_eq!(
        source_result["error"]["message"],
        format!(
            "[Errno 2] No such file or directory: '{}'",
            expected_path.display()
        )
    );
}

#[test]
fn missing_parent_source_uses_absolute_path_in_per_file_json_error() {
    let workspace = Workspace::new();
    let output = run_check(&workspace, None, Some("absent-dir/absent.txt"));
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("JSON per-file error");
    let source_result = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(source_result["error"]["code"], -31998);
    let expected_path = workspace.path().join("absent-dir/absent.txt");
    assert_eq!(
        source_result["error"]["message"],
        format!(
            "[Errno 2] No such file or directory: '{}'",
            expected_path.display()
        )
    );
}

#[test]
fn invalid_config_is_a_structured_json_decode_error() {
    let workspace = Workspace::new();
    fs::write(workspace.path().join("malformed config.json"), b"{").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "malformed config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI with malformed config");
    assert_eq!(output.status.code(), Some(4));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("structured JSON error");
    assert_eq!(value["error"]["code"], -31999);
    assert_eq!(value["error"]["data"], "JSONDecodeError");
    assert_eq!(
        value["error"]["message"],
        "Expecting property name enclosed in double quotes: line 1 column 2 (char 1)"
    );
}

#[test]
fn explicit_missing_config_is_a_structured_json_error() {
    let workspace = Workspace::new();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "missing config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI with missing config");
    assert_eq!(output.status.code(), Some(4));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("structured JSON error");
    assert_eq!(
        value,
        serde_json::json!({"error": {"code": -31998, "message": "Configuration file 'missing config.json' could not be read."}})
    );
}
