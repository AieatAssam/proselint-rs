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
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
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

#[test]
fn absolute_per_file_glob_matches_a_relative_cli_path() {
    let workspace = Workspace::new();
    let pattern = workspace.0.join("*.txt");
    fs::write(
        workspace.0.join("selected config.json"),
        format!(
            r#"{{"max_errors":1000,"checks":{{"annotations":{{"misc":true}}}},"per_file_checks":{{"{}":{{"annotations":{{"misc":false}}}}}}}}"#,
            pattern.display()
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
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
        .expect("run native CLI on absolute per-file glob");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
}

#[test]
fn caret_is_literal_in_per_file_glob_character_class() {
    let output = run_with_config(
        r#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"[^a]*.txt":{"annotations":{"misc":false}}}}"#,
        "beta.txt",
        "TODO",
    );
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let source = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(source["diagnostics"].as_array().unwrap().len(), 1);
}

#[test]
fn per_file_globs_follow_python_path_match_normalization() {
    for (pattern, source) in [
        ("./*.txt", "draft.txt"),
        ("./a/*.txt", "a/draft.txt"),
        ("a//*.txt", "a/draft.txt"),
        ("a/*.txt/", "a/draft.txt"),
        ("[]a]*.txt", "]draft.txt"),
        ("[!]]*.txt", "adraft.txt"),
        ("[-a].txt", "a.txt"),
    ] {
        let config = serde_json::json!({
            "max_errors": 1000,
            "checks": {"annotations": {"misc": true}},
            "per_file_checks": {pattern: {"annotations": {"misc": false}}}
        })
        .to_string();
        let output = run_with_config(&config, source, "TODO");
        assert_eq!(
            output.status.code(),
            Some(0),
            "pattern {pattern:?}, source {source:?}, stderr: {:?}",
            output.stderr
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_per_file_glob_accepts_backslash_separators() {
    let config = serde_json::json!({
        "max_errors": 1000,
        "checks": {"annotations": {"misc": true}},
        "per_file_checks": {r"sub\*.txt": {"annotations": {"misc": false}}}
    })
    .to_string();
    let output = run_with_config(&config, "sub/draft.txt", "TODO");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
}

#[test]
fn slash_glob_does_not_match_an_ancestor_path_window() {
    let workspace = Workspace::new();
    let nested = workspace.0.join("parent/notes/file.txt");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("child.txt"), "TODO").unwrap();
    fs::write(
        workspace.0.join("selected config.json"),
        br#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"parent/notes/*.txt":{"annotations":{"misc":false}}}}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
            "parent/notes/file.txt/child.txt",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI on nested path");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let source = value["result"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(source["diagnostics"].as_array().unwrap().len(), 1);
}

fn run_with_config(config: &str, source_name: &str, source_text: &str) -> std::process::Output {
    run_with_config_format(config, source_name, source_text, Some("json"))
}

fn run_with_config_format(
    config: &str,
    source_name: &str,
    source_text: &str,
    format: Option<&str>,
) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("selected config.json"), config).unwrap();
    let source_path = workspace.0.join(source_name);
    fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    fs::write(source_path, source_text).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .current_dir(&workspace.0)
        .args(["check", "--config", "selected config.json"]);
    if let Some(format) = format {
        command.args(["--output-format", format]);
    }
    command
        .arg(source_name)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI with file-specific config")
}

fn run_dump_config_with_config(config: &str, format: &str) -> std::process::Output {
    run_dump_config_with_config_and_version(config, format, false)
}

fn run_dump_config_with_config_and_version(
    config: &str,
    format: &str,
    version: bool,
) -> std::process::Output {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("selected config.json"), config).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .current_dir(&workspace.0)
        .args(["dump-config", "--config", "selected config.json"]);
    if version {
        command.arg("--version");
    }
    command
        .args(["--output-format", format])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run dump-config with explicit config")
}

fn assert_dump_config_attribute_error(output: std::process::Output, format: &str, type_name: &str) {
    let message = format!("'{type_name}' object has no attribute 'items'");
    assert_eq!(output.status.code(), Some(4), "format {format}");
    if format == "json" {
        assert!(output.stderr.is_empty(), "format {format}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["error"],
            serde_json::json!({
                "code": -31999,
                "data": "AttributeError",
                "message": message
            }),
            "format {format}"
        );
    } else {
        assert!(output.stdout.is_empty(), "format {format}");
        assert_eq!(
            output.stderr,
            format!("Unrecoverable error encountered while running proselint: {message}\n")
                .as_bytes(),
            "format {format}"
        );
    }
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

#[test]
fn null_per_file_checks_matches_python_attribute_error() {
    let output = run_with_config(r#"{"checks":{},"per_file_checks":null}"#, "input.txt", "");
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["error"],
        serde_json::json!({
            "code": -31999,
            "data": "AttributeError",
            "message": "'NoneType' object has no attribute 'items'"
        })
    );
}

#[test]
fn null_per_file_checks_full_and_compact_errors_match_python() {
    for format in ["full", "compact"] {
        let output = run_with_config_format(
            r#"{"checks":{},"per_file_checks":null}"#,
            "input.txt",
            "",
            Some(format),
        );
        assert_eq!(output.status.code(), Some(4), "format {format}");
        assert!(output.stdout.is_empty(), "format {format}");
        assert_eq!(
            output.stderr,
            b"Unrecoverable error encountered while running proselint: 'NoneType' object has no attribute 'items'\n",
            "format {format}"
        );
    }
}

#[test]
fn dump_config_null_per_file_checks_matches_python_in_all_output_modes() {
    for format in ["json", "full", "compact"] {
        let output = run_dump_config_with_config(r#"{"checks":{},"per_file_checks":null}"#, format);
        assert_eq!(output.status.code(), Some(4), "format {format}");
        if format == "json" {
            assert!(output.stderr.is_empty());
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                value["error"],
                serde_json::json!({
                    "code": -31999,
                    "data": "AttributeError",
                    "message": "'NoneType' object has no attribute 'items'"
                })
            );
        } else {
            assert!(output.stdout.is_empty(), "format {format}");
            assert_eq!(
                output.stderr,
                b"Unrecoverable error encountered while running proselint: 'NoneType' object has no attribute 'items'\n",
                "format {format}"
            );
        }
    }
}

#[test]
fn dump_config_version_does_not_override_null_per_file_checks_error() {
    for format in ["json", "full", "compact"] {
        let output = run_dump_config_with_config_and_version(
            r#"{"checks":{},"per_file_checks":null}"#,
            format,
            true,
        );
        assert_eq!(output.status.code(), Some(4), "format {format}");
        if format == "json" {
            assert!(output.stderr.is_empty());
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                value["error"],
                serde_json::json!({
                    "code": -31999,
                    "data": "AttributeError",
                    "message": "'NoneType' object has no attribute 'items'"
                })
            );
        } else {
            assert!(output.stdout.is_empty(), "format {format}");
            assert_eq!(
                output.stderr,
                b"Unrecoverable error encountered while running proselint: 'NoneType' object has no attribute 'items'\n",
                "format {format}"
            );
        }
    }
}

#[test]
fn dump_config_invalid_per_file_checks_types_match_python() {
    for (value, type_name) in [
        ("[]", "list"),
        (r#""invalid""#, "str"),
        ("true", "bool"),
        ("17", "int"),
        ("1.5", "float"),
    ] {
        let config = format!(r#"{{"checks":{{}},"per_file_checks":{value}}}"#);
        for format in ["json", "full", "compact"] {
            assert_dump_config_attribute_error(
                run_dump_config_with_config(&config, format),
                format,
                type_name,
            );
        }
        assert_dump_config_attribute_error(
            run_dump_config_with_config_and_version(&config, "json", true),
            "json",
            type_name,
        );
    }
}

#[test]
fn dump_config_accepts_empty_per_file_checks_mapping() {
    let output = run_dump_config_with_config(r#"{"checks":{},"per_file_checks":{}}"#, "json");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"max_errors":0,"checks":{},"per_file_checks":{}})
    );
}

fn assert_check_attribute_error(output: std::process::Output, format: &str, type_name: &str) {
    let message = format!("'{type_name}' object has no attribute 'items'");
    assert_eq!(output.status.code(), Some(4), "format {format}");
    if format == "json" {
        assert!(output.stderr.is_empty(), "format {format}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["error"],
            serde_json::json!({
                "code": -31999,
                "data": "AttributeError",
                "message": message
            }),
            "format {format}"
        );
    } else {
        assert!(output.stdout.is_empty(), "format {format}");
        assert_eq!(
            output.stderr,
            format!("Unrecoverable error encountered while running proselint: {message}\n")
                .as_bytes(),
            "format {format}"
        );
    }
}

fn per_file_file_config(value: &str) -> String {
    format!(r#"{{"checks":{{}},"per_file_checks":{{"*.zzz":{value}}}}}"#)
}

#[test]
fn invalid_per_file_file_config_values_fail_eagerly_in_all_cli_modes() {
    for (value, type_name) in [
        ("null", "NoneType"),
        ("[]", "list"),
        (r#""bad""#, "str"),
        ("false", "bool"),
        ("7", "int"),
        ("1.5", "float"),
    ] {
        let config = per_file_file_config(value);
        for format in ["json", "full", "compact"] {
            assert_check_attribute_error(
                run_with_config_format(&config, "input.txt", "", Some(format)),
                format,
                type_name,
            );
            assert_dump_config_attribute_error(
                run_dump_config_with_config(&config, format),
                format,
                type_name,
            );
            assert_dump_config_attribute_error(
                run_dump_config_with_config_and_version(&config, format, true),
                format,
                type_name,
            );
        }
    }
}

#[test]
fn valid_per_file_file_config_mapping_remains_accepted() {
    let config = r#"{"checks":{},"per_file_checks":{"*.txt":{"misc":{"apologizing":true}}}}"#;
    let expected = serde_json::json!({
        "max_errors": 0,
        "checks": {},
        "per_file_checks": {"*.txt": {"misc.apologizing": true}}
    });
    for format in ["json", "full", "compact"] {
        let output = run_dump_config_with_config(config, format);
        assert_eq!(
            output.status.code(),
            Some(0),
            "format {format}: {:?}",
            output.stderr
        );
        assert!(output.stderr.is_empty(), "format {format}");
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            expected,
            "format {format}"
        );
    }
}

#[test]
fn nested_invalid_per_file_check_leaf_fails_before_version() {
    let config = r#"{"checks":{},"per_file_checks":{"*.txt":{"misc":{"apologizing":null}}}}"#;
    assert_dump_config_attribute_error(
        run_dump_config_with_config_and_version(config, "json", true),
        "json",
        "NoneType",
    );
}

#[test]
fn duplicate_per_file_pattern_uses_last_value() {
    let valid_after_invalid =
        r#"{"checks":{},"per_file_checks":{"*.txt":null,"*.txt":{"misc":{"apologizing":true}}}}"#;
    let output = run_dump_config_with_config(valid_after_invalid, "json");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());

    let invalid_after_valid =
        r#"{"checks":{},"per_file_checks":{"*.txt":{"misc":{"apologizing":true}},"*.txt":null}}"#;
    assert_dump_config_attribute_error(
        run_dump_config_with_config(invalid_after_valid, "json"),
        "json",
        "NoneType",
    );
}

#[test]
fn checks_error_precedes_nested_per_file_error_before_version() {
    let config = r#"{"checks":[],"per_file_checks":{"*.txt":null}}"#;
    assert_dump_config_attribute_error(
        run_dump_config_with_config_and_version(config, "json", true),
        "json",
        "list",
    );
}
