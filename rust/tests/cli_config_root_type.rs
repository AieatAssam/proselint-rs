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
            "proselint-rs-config-root-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(config: &str, format: &str) -> std::process::Output {
    run_args(
        config,
        &[
            "check",
            "--config",
            "config.json",
            "--output-format",
            format,
        ],
    )
}

fn run_args(config: &str, args: &[&str]) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("config.json"), config).expect("write config fixture");
    Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI with non-object config")
}

#[test]
fn non_object_config_is_loaded_before_version_and_dump_config_dispatch() {
    let expected = b"Unrecoverable error encountered while running proselint: 'list' object has no attribute 'get'\n";
    for args in [
        &["--config", "config.json", "--version"][..],
        &["dump-config", "--config", "config.json"][..],
    ] {
        let output = run_args("[]", args);
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, expected);
    }
}

#[test]
fn version_and_dump_config_json_paths_preserve_the_response_error() {
    let expected = json!({"error": {
        "code": -31999,
        "data": "AttributeError",
        "message": "'list' object has no attribute 'get'"
    }});
    for args in [
        &[
            "--output-format",
            "json",
            "--config",
            "config.json",
            "--version",
        ][..],
        &[
            "dump-config",
            "--output-format",
            "json",
            "--config",
            "config.json",
        ][..],
    ] {
        let output = run_args("[]", args);
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            expected
        );
        assert!(output.stderr.is_empty());
    }
}

fn assert_json_root_error(config: &str, python_type: &str) {
    let output = run(config, "json");
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).expect("JSON error response");
    assert_eq!(
        value,
        json!({"error": {
            "code": -31999,
            "data": "AttributeError",
            "message": format!("'{}' object has no attribute 'get'", python_type)
        }})
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn each_non_object_json_root_preserves_python_attribute_error() {
    let large_integer = "1".repeat(400);
    for (config, python_type) in [
        ("[]", "list"),
        ("null", "NoneType"),
        ("true", "bool"),
        ("1", "int"),
        ("1.0", "float"),
        (&large_integer, "int"),
        ("1e400", "float"),
        ("NaN", "float"),
        ("Infinity", "float"),
        ("-Infinity", "float"),
        ("\"x\"", "str"),
    ] {
        assert_json_root_error(config, python_type);
    }
}

#[test]
fn full_and_compact_report_array_root_error_on_stderr() {
    let expected = b"Unrecoverable error encountered while running proselint: 'list' object has no attribute 'get'\n";
    for format in ["full", "compact"] {
        let output = run("[]", format);
        assert_eq!(output.status.code(), Some(4), "format: {format}");
        assert!(
            output.stdout.is_empty(),
            "format: {format}, stdout: {:?}",
            output.stdout
        );
        assert_eq!(output.stderr, expected, "format: {format}");
    }
}
