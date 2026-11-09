use serde_json::{Value, json};
use std::fs;
use std::io::Write;
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
    Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
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
fn max_errors_null_is_unlimited_for_two_todos() {
    let workspace = Workspace::new();
    fs::write(
        workspace.0.join("config.json"),
        br#"{"max_errors":null,"checks":{"annotations":{"misc":true}}}"#,
    )
    .unwrap();
    let output = run_stdin(&workspace.0, "TODO TODO");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let diagnostics = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["diagnostics"]
        .as_array()
        .unwrap();
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn max_errors_true_acts_as_integer_one() {
    let workspace = Workspace::new();
    fs::write(
        workspace.0.join("config.json"),
        br#"{"max_errors":true,"checks":{"annotations":{"misc":true}}}"#,
    )
    .unwrap();
    let output = run_stdin(&workspace.0, "TODO TODO");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let diagnostics = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()["diagnostics"]
        .as_array()
        .unwrap();
    assert_eq!(diagnostics.len(), 1);
}

fn run_file(workspace: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
            "source.txt",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("run CLI")
}

fn run_stdin(workspace: &std::path::Path, input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace)
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
        .expect("run CLI");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().expect("wait for CLI")
}

fn assert_per_file_max_errors_error(value: &str) {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("source.txt"), "text").unwrap();
    let config = format!(r#"{{"checks":{{}},"max_errors":{value}}}"#);
    fs::write(workspace.0.join("config.json"), config).unwrap();
    let output = run_file(&workspace.0);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let result = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(result["error"]["code"], -31999);
    assert_eq!(result["error"]["data"], "ValueError");
    assert_eq!(
        result["error"]["message"],
        "Stop argument for islice() must be None or an integer: 0 <= x <= sys.maxsize."
    );
}

#[test]
fn per_file_string_list_object_negative_and_fractional_limits_match_python_value_error() {
    for value in [r#""1""#, "[]", "{}", "-1", "1.5"] {
        assert_per_file_max_errors_error(value);
    }
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
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
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

fn run_dump_config(config: &[u8], format: &str, version: bool) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("config.json"), config).expect("write config");
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .current_dir(&workspace.0)
        .args(["dump-config", "--config", "config.json"]);
    if version {
        command.arg("--version");
    }
    command
        .args(["--output-format", format])
        .stdin(Stdio::null())
        .output()
        .expect("run dump-config")
}

fn assert_dump_config_checks_attribute_error(
    config: &[u8],
    format: &str,
    type_name: &str,
    version: bool,
) {
    let output = run_dump_config(config, format, version);
    let message = format!("'{type_name}' object has no attribute 'items'");
    assert_eq!(
        output.status.code(),
        Some(4),
        "format {format}, version {version}"
    );
    if format == "json" {
        assert!(
            output.stderr.is_empty(),
            "format {format}, version {version}"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            json!({
                "error": {
                    "code": -31999,
                    "data": "AttributeError",
                    "message": message
                }
            }),
            "format {format}, version {version}"
        );
    } else {
        assert!(
            output.stdout.is_empty(),
            "format {format}, version {version}"
        );
        assert_eq!(
            output.stderr,
            format!("Unrecoverable error encountered while running proselint: {message}\n")
                .as_bytes(),
            "format {format}, version {version}"
        );
    }
}

#[test]
fn dump_config_invalid_checks_values_match_python_items_errors() {
    for (value, type_name) in [
        ("null", "NoneType"),
        ("[]", "list"),
        (r#""invalid""#, "str"),
        ("false", "bool"),
        ("7", "int"),
        ("1.5", "float"),
    ] {
        let config = format!(r#"{{"checks":{value}}}"#);
        for format in ["json", "full", "compact"] {
            for version in [false, true] {
                assert_dump_config_checks_attribute_error(
                    config.as_bytes(),
                    format,
                    type_name,
                    version,
                );
            }
        }
    }
}

#[test]
fn dump_config_accepts_empty_checks_mapping_in_all_formats() {
    for format in ["json", "full", "compact"] {
        let output = run_dump_config(br#"{"checks":{}}"#, format, false);
        assert_eq!(output.status.code(), Some(0), "format {format}");
        assert!(output.stderr.is_empty(), "format {format}");
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            json!({"max_errors": 0, "checks": {}, "per_file_checks": {}}),
            "format {format}"
        );
    }
}

#[test]
fn dump_config_checks_error_precedes_per_file_error_before_version() {
    assert_dump_config_checks_attribute_error(
        br#"{"checks":[],"per_file_checks":null}"#,
        "json",
        "list",
        true,
    );
}
