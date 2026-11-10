use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const PER_FILE_OVERRIDE_OUTPUT: &[u8] =
    include_bytes!("fixtures/cli_config_missing_checks_per_file_override.bin");

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-missing-checks-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated HOME and XDG config");
        Self(path)
    }

    fn write_config(&self, config: &str) {
        fs::write(self.0.join("config.json"), config).expect("write config fixture");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("home/xdg"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn max_errors_only_config_means_no_checks_and_no_findings() {
    let workspace = Workspace::new();
    workspace.write_config(r#"{"max_errors":1}"#);

    let output = workspace.run(&[
        "check",
        "--demo",
        "--config",
        "config.json",
        "--output-format",
        "compact",
    ]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn missing_global_checks_still_allows_per_file_override() {
    let workspace = Workspace::new();
    workspace.write_config(
        r#"{"max_errors":1,"per_file_checks":{"draft.txt":{"misc":{"suddenly":true}}}}"#,
    );
    fs::write(workspace.0.join("draft.txt"), "Suddenly, we launched.")
        .expect("write positive suddenly fixture");

    let output = workspace.run(&[
        "check",
        "--config",
        "config.json",
        "--output-format",
        "compact",
        "draft.txt",
    ]);

    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, PER_FILE_OVERRIDE_OUTPUT);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
