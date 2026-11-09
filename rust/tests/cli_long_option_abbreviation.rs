use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const SELECTED_RULE_OUTPUT: &[u8] = include_bytes!("fixtures/demo_compact_selected_rule.bin");

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-long-option-abbrev-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create isolated workspace");
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated config directories");
        fs::write(
            path.join("selected.json"),
            r#"{"max_errors":1000,"checks":{"dates_times":{"am_pm":{"lowercase_periods":true}}}}"#,
        )
        .expect("write selected demo rule");
        fs::write(path.join("input.txt"), "Hello world.\n").expect("write input");
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("home/xdg"))
            .args(args)
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
fn unique_check_option_prefix_runs_demo_as_compact() {
    let output = Workspace::new().run(&[
        "check",
        "--dem",
        "--config",
        "selected.json",
        "--output-format",
        "compact",
    ]);
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, SELECTED_RULE_OUTPUT);
}

#[test]
fn unique_dump_config_option_prefix_outputs_default_config() {
    let workspace = Workspace::new();
    let abbreviated = workspace.run(&["dump-config", "--def"]);
    let full = workspace.run(&["dump-config", "--default"]);
    assert_eq!(
        abbreviated.status.code(),
        Some(0),
        "stderr: {:?}",
        abbreviated.stderr
    );
    assert_eq!(abbreviated.stdout, full.stdout);
    assert!(
        abbreviated.stderr.is_empty(),
        "stderr: {:?}",
        abbreviated.stderr
    );
}

#[test]
fn unique_global_check_option_prefix_selects_full_output() {
    let output = Workspace::new().run(&["check", "--out", "full", "input.txt"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}

#[test]
fn unique_root_config_option_prefix_accepts_config_before_command() {
    let output = Workspace::new().run(&["--con", "selected.json", "check", "input.txt"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
}

#[test]
fn ambiguous_root_option_prefix_matches_argparse_diagnostic() {
    let output = Workspace::new().run(&["--ver"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        b"usage: proselint [options] <command>\nproselint: error: ambiguous option: --ver could match --verbose, --version\n"
    );
}

#[test]
fn unique_check_help_prefix_matches_full_help() {
    let workspace = Workspace::new();
    let abbreviated = workspace.run(&["check", "--he"]);
    let full = workspace.run(&["check", "--help"]);
    assert_eq!(abbreviated.status.code(), Some(0));
    assert_eq!(abbreviated.stdout, full.stdout);
    assert!(
        abbreviated.stderr.is_empty(),
        "stderr: {:?}",
        abbreviated.stderr
    );
}
