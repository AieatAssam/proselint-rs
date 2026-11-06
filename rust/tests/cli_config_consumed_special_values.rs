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
            "proselint-rs-consumed-config-{}-{serial}",
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

fn run_cli(config: &[u8], paths: &[&str]) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("config.json"), config).expect("write config");
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
        ])
        .args(paths)
        .stdin(Stdio::null())
        .output()
        .expect("run CLI")
}

fn assert_max_errors_value_error(config: &[u8]) {
    let output = run_cli(config, &[]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "result": {
                "<stdin>": {
                    "error": {
                        "code": -31999,
                        "data": "ValueError",
                        "message": "Stop argument for islice() must be None or an integer: 0 <= x <= sys.maxsize."
                    }
                }
            }
        })
    );
    assert!(output.stderr.is_empty());
}

fn assert_checks_attribute_error(config: &[u8]) {
    let output = run_cli(config, &[]);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "error": {
                "code": -31999,
                "data": "AttributeError",
                "message": "'float' object has no attribute 'items'"
            }
        })
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn max_errors_nan_matches_python_islice_value_error() {
    assert_max_errors_value_error(br#"{"checks":{},"max_errors":NaN}"#);
}

#[test]
fn max_errors_overflow_float_matches_python_islice_value_error() {
    assert_max_errors_value_error(br#"{"checks":{},"max_errors":1e999}"#);
}

#[test]
fn max_errors_lone_surrogate_matches_python_islice_value_error() {
    assert_max_errors_value_error(br#"{"checks":{},"max_errors":"\ud800"}"#);
}

#[test]
fn checks_nan_matches_python_attribute_error() {
    assert_checks_attribute_error(br#"{"checks":{"annotations":NaN}}"#);
}

#[test]
fn checks_overflow_float_matches_python_attribute_error() {
    assert_checks_attribute_error(br#"{"checks":{"annotations":1e999}}"#);
}

#[test]
fn checks_error_precedes_max_errors_error() {
    assert_checks_attribute_error(br#"{"checks":{"annotations":NaN},"max_errors":NaN}"#);
}

#[test]
fn missing_source_error_precedes_max_errors_error() {
    let workspace = Workspace::new();
    fs::write(
        workspace.0.join("config.json"),
        br#"{"checks":{},"max_errors":NaN}"#,
    )
    .expect("write config");
    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
            "absent.txt",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let results = value["result"].as_object().unwrap();
    assert_eq!(results.len(), 1);
    let (source, result) = results.iter().next().unwrap();
    assert!(source.ends_with("/absent.txt"), "{source}");
    assert_eq!(result["error"]["code"], -31998);
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("No such file or directory")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn duplicate_max_errors_uses_last_valid_value() {
    let output = run_cli(br#"{"checks":{},"max_errors":NaN,"max_errors":100}"#, &[]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"result":{"<stdin>":{"diagnostics":[]}}})
    );
    assert!(output.stderr.is_empty());
}

fn assert_checks_type_attribute_error(config: &[u8], type_name: &str) {
    let output = run_cli(config, &[]);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "error": {
                "code": -31999,
                "data": "AttributeError",
                "message": format!("'{type_name}' object has no attribute 'items'")
            }
        })
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn checks_array_nan_matches_python_list_attribute_error() {
    assert_checks_type_attribute_error(br#"{"checks":{"annotations":[NaN]}}"#, "list");
}

#[test]
fn checks_nested_array_nan_matches_python_list_attribute_error() {
    assert_checks_type_attribute_error(br#"{"checks":{"annotations":{"nested":[NaN]}}}"#, "list");
}

#[test]
fn checks_lone_surrogate_matches_python_string_attribute_error() {
    assert_checks_type_attribute_error(br#"{"checks":{"annotations":"\ud800"}}"#, "str");
}

#[test]
fn duplicate_checks_misc_uses_last_boolean_value() {
    let output = run_cli(
        br#"{"checks":{"annotations":{"misc":NaN,"misc":true}}}"#,
        &[],
    );
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"result":{"<stdin>":{"diagnostics":[]}}})
    );
    assert!(output.stderr.is_empty());
}
