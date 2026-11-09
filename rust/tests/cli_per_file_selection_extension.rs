use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new(config: &str, files: &[(&str, &str)]) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-per-file-selection-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::write(path.join("selected config.json"), config).expect("write config");
        for (name, content) in files {
            fs::write(path.join(name), content).expect("write source file");
        }
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn check(config: &str, file: &str, content: &str) -> (i32, Value) {
    let workspace = Workspace::new(config, &[(file, content)]);
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
            file,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run CLI with explicit config and source");
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    (
        output.status.code().expect("CLI exited normally"),
        serde_json::from_slice(&output.stdout).expect("valid JSON output"),
    )
}

fn diagnostics(value: &Value) -> &Value {
    value["result"]
        .as_object()
        .expect("result map")
        .values()
        .next()
        .expect("source result")
        .get("diagnostics")
        .expect("diagnostics")
}

#[test]
fn per_file_override_enables_a_supported_check_disabled_globally() {
    let (code, value) = check(
        r#"{"max_errors":1000,"checks":{"psychology":{"p_equals_zero":false}},"per_file_checks":{"*.txt":{"psychology":{"p_equals_zero":true}}}}"#,
        "draft.txt",
        "p = 0.00",
    );
    assert_eq!(code, 1);
    assert_eq!(
        diagnostics(&value),
        &json!([{
            "check_path":"psychology.p_equals_zero",
            "message":"Unless p really equals zero, you should use more decimal places.",
            "pos":[1,1], "replacements":null, "span":[1,9]
        }])
    );
}

#[test]
fn per_file_map_merges_with_global_check_selection() {
    let (code, value) = check(
        r#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"*.txt":{"psychology":{"p_equals_zero":true}}}}"#,
        "draft.txt",
        "TODO; p = 0.00",
    );
    assert_eq!(code, 1);
    assert_eq!(
        diagnostics(&value),
        &json!([
            {"check_path":"annotations.misc", "message":"Annotation left in text.", "pos":[1,1], "replacements":null, "span":[1,5]},
            {"check_path":"psychology.p_equals_zero", "message":"Unless p really equals zero, you should use more decimal places.", "pos":[1,7], "replacements":null, "span":[7,15]}
        ])
    );
}

#[test]
fn nonmatching_per_file_pattern_falls_back_to_global_checks() {
    let (code, value) = check(
        r#"{"max_errors":1000,"checks":{"annotations":{"misc":true}},"per_file_checks":{"*.md":{"annotations":{"misc":false}}}}"#,
        "draft.txt",
        "TODO",
    );
    assert_eq!(code, 1);
    assert_eq!(
        diagnostics(&value),
        &json!([{
            "check_path":"annotations.misc", "message":"Annotation left in text.",
            "pos":[1,1], "replacements":null, "span":[1,5]
        }])
    );
}
