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
            "proselint-rs-commercialese-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"industrial_language":{"commercialese":true}}}"#,
        )
        .expect("write config selecting industrial_language.commercialese");
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
        "unexpected CLI status {:?}, stderr: {:?}",
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
fn reports_an_exact_commercialese_finding() {
    assert_eq!(
        diagnostics("enclosed please find"),
        vec![json!({
            "check_path": "industrial_language.commercialese",
            "message": "'enclosed please find' is commercialese.",
            "pos": [1, 1],
            "replacements": null,
            "span": [1, 21],
        })]
    );
}

#[test]
fn escaped_abbreviations_activate_then_report_raw_occurrences() {
    let actual = diagnostics("Inst.X prox.Y ult.Z; inst. prox. ult.; instx prox ultx.");
    assert_eq!(actual.len(), 6);
    assert_eq!(
        actual
            .iter()
            .map(|item| item["message"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "'Inst.' is commercialese.",
            "'prox.' is commercialese.",
            "'ult.' is commercialese.",
            "'inst.' is commercialese.",
            "'prox.' is commercialese.",
            "'ult.' is commercialese.",
        ]
    );
    assert_eq!(
        actual
            .iter()
            .map(|item| item["span"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!([1, 6]),
            json!([8, 13]),
            json!([15, 19]),
            json!([22, 27]),
            json!([28, 33]),
            json!([34, 38]),
        ]
    );
}
