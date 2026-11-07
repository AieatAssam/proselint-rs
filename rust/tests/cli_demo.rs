use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const SELECTED_RULE_OUTPUT: &[u8] = include_bytes!("fixtures/demo_compact_selected_rule.bin");
const MISSING_CONFIG_STDERR: &[u8] =
    include_bytes!("fixtures/demo_compact_config_error.stderr.bin");
const SELECTED_CONFIG: &str =
    r#"{"max_errors":1000,"checks":{"dates_times":{"am_pm":{"lowercase_periods":true}}}}"#;

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("proselint-rs-demo-{}-{serial}", std::process::id()));
        fs::create_dir(&path).expect("create isolated demo workspace");
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated HOME and XDG config");
        Self(path)
    }

    fn selected_config(&self) {
        fs::write(self.0.join("selected config.json"), SELECTED_CONFIG)
            .expect("write selected rule config");
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn demo_compact_uses_pinned_sample_with_selected_rule() {
    let workspace = Workspace::new();
    workspace.selected_config();
    let output = run(
        &workspace,
        &[
            "check",
            "--demo",
            "--config",
            "selected config.json",
            "--output-format",
            "compact",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, SELECTED_RULE_OUTPUT);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn demo_compact_ignores_extra_positional_path() {
    let workspace = Workspace::new();
    workspace.selected_config();
    let output = run(
        &workspace,
        &[
            "check",
            "--demo",
            "not-a-real-demo-input.md",
            "--config",
            "selected config.json",
            "--output-format",
            "compact",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, SELECTED_RULE_OUTPUT);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn demo_config_error_occurs_before_demo_selection() {
    let workspace = Workspace::new();
    let output = run(
        &workspace,
        &[
            "check",
            "--demo",
            "--config",
            "missing config.json",
            "--output-format",
            "compact",
        ],
    );

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, MISSING_CONFIG_STDERR);
}

fn run(workspace: &Workspace, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .env("HOME", workspace.0.join("home"))
        .env("XDG_CONFIG_HOME", workspace.0.join("home/xdg"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI")
}
