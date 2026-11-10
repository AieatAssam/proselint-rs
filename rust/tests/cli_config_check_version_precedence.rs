use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const ATTRIBUTE_ERROR: &str = "'NoneType' object has no attribute 'items'";
const COMPACT_ERROR: &[u8] = b"Unrecoverable error encountered while running proselint: 'NoneType' object has no attribute 'items'\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new(config: &str) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-config-version-precedence-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated config workspace");
        fs::write(path.join("config.json"), config).expect("write invalid config");
        Self(path)
    }

    fn run(&self, output_format: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
        command
            .current_dir(&self.0)
            .args(["check", "--config", "config.json", "--version"]);
        if let Some(format) = output_format {
            command.args(["--output-format", format]);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_config_error_precedes_version(config: &str) {
    for format in [None, Some("compact"), Some("json")] {
        let output = Workspace::new(config).run(format);
        assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
        if format == Some("json") {
            let result: Value =
                serde_json::from_slice(&output.stdout).expect("JSON AttributeError output");
            assert_eq!(
                result,
                json!({
                    "error": {
                        "code": -31999,
                        "data": "AttributeError",
                        "message": ATTRIBUTE_ERROR
                    }
                })
            );
            assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
        } else {
            assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
            assert_eq!(output.stderr, COMPACT_ERROR);
        }
    }
}

#[test]
fn checks_null_config_error_precedes_check_version() {
    assert_config_error_precedes_version(r#"{"checks":null}"#);
}

#[test]
fn per_file_checks_null_config_error_precedes_check_version() {
    assert_config_error_precedes_version(r#"{"per_file_checks":null}"#);
}
