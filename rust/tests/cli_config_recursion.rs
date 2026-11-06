use serde_json::{Value, json};
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
            "proselint-rs-config-recursion-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated config workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn invoke(config: &[u8]) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("deep config.json"), config).expect("write config");
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "deep config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("run CLI")
}

fn assert_clean(config: &[u8]) {
    let output = invoke(config);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"result":{"<stdin>":{"diagnostics":[]}}})
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn accepts_valid_config_array_beyond_serde_default_depth() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 150));
    config.push(b'0');
    config.extend(std::iter::repeat_n(b']', 150));
    config.push(b'}');
    assert_clean(&config);
}

#[test]
fn accepts_python_maximum_valid_array_depth_9996() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 9_996));
    config.push(b'0');
    config.extend(std::iter::repeat_n(b']', 9_996));
    config.push(b'}');
    assert_clean(&config);
}

fn assert_python_recursion_error(config: &[u8], kind: &str) {
    let output = invoke(config);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).expect("structured CLI error");
    assert_eq!(value["error"]["data"], "RecursionError");
    assert_eq!(
        value["error"]["message"],
        format!(
            "maximum recursion depth exceeded while decoding a JSON {kind} from a unicode string"
        )
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn array_depth_9997_returns_python_recursion_error() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 9_997));
    config.extend_from_slice(b",}");
    assert_python_recursion_error(&config, "array");
}

#[test]
fn object_depth_9997_returns_python_recursion_error() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b"{\"a\":".as_slice(), 9_997).flatten());
    config.extend_from_slice(b",}");
    assert_python_recursion_error(&config, "object");
}

#[test]
fn alternating_depth_9997_returns_python_recursion_error() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b"{\"a\":[".as_slice(), 4_998).flatten());
    config.extend_from_slice(b"{\"a\":,");
    config.push(b'}');
    assert_python_recursion_error(&config, "object");
}

#[test]
fn accepts_valid_config_object_beyond_serde_default_depth() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b"{\"a\":".as_slice(), 150).flatten());
    config.push(b'0');
    config.extend(std::iter::repeat_n(b'}', 150));
    config.push(b'}');
    assert_clean(&config);
}

#[test]
fn accepts_valid_alternating_containers_beyond_serde_default_depth() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b"{\"a\":[".as_slice(), 75).flatten());
    config.push(b'0');
    for _ in 0..75 {
        config.extend_from_slice(b"]}");
    }
    config.push(b'}');
    assert_clean(&config);
}

#[test]
fn deep_malformed_array_returns_structured_python_recursion_error() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 10_000));
    config.extend_from_slice(b",}");
    let output = invoke(&config);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).expect("structured CLI error");
    assert_eq!(
        value["error"]["data"],
        "RecursionError",
        "stdout: {} stderr: {:?}",
        String::from_utf8_lossy(&output.stdout),
        output.stderr
    );
    assert_eq!(
        value["error"]["message"],
        "maximum recursion depth exceeded while decoding a JSON array from a unicode string"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn one_hundred_thousand_unclosed_array_brackets_return_python_recursion_error() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 100_000));
    assert_python_recursion_error(&config, "array");
}

#[test]
fn sixty_six_thousand_level_input_returns_python_recursion_error_without_aborting_test_runner() {
    let mut config = b"{\"checks\":{},\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 66_000));
    config.extend_from_slice(b",}");
    let output = invoke(&config);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).expect("structured CLI error");
    assert_eq!(value["error"]["data"], "RecursionError");
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
