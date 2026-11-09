use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const CHECK_USAGE: &str = "usage: proselint [options] <command> check [-h] [--config CONFIG]\n                                           [--output-format {full,json,compact}]\n                                           [--verbose] [--version] [--demo]\n                                           [paths ...]\n";
const DUMP_CONFIG_USAGE: &str = "usage: proselint [options] <command> dump-config [-h] [--config CONFIG]\n                                                 [--output-format {full,json,compact}]\n                                                 [--verbose] [--version]\n                                                 [--default]\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-cli-attached-option-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated config directories");
        fs::write(
            path.join("config.json"),
            r#"{"max_errors":1000,"checks":{}}"#,
        )
        .expect("write valid empty configuration");
        fs::write(path.join("input.txt"), "Hello world.\n").expect("write clean input");
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

fn assert_error(args: &[&str], expected_stderr: &str) {
    let output = Workspace::new().run(args);
    assert_eq!(output.status.code(), Some(2), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(output.stderr, expected_stderr.as_bytes());
}

#[test]
fn attached_ambiguous_prefix_preserves_the_full_option_token() {
    assert_error(
        &["--ver=value"],
        "usage: proselint [options] <command>\nproselint: error: ambiguous option: --ver=value could match --verbose, --version\n",
    );
}

#[test]
fn attached_unique_root_flag_prefix_reports_argparse_error() {
    assert_error(
        &["--vers=value"],
        "usage: proselint [options] <command>\nproselint: error: argument --version: ignored explicit argument 'value'\n",
    );
}

#[test]
fn attached_check_flag_prefix_reports_argparse_error() {
    assert_error(
        &["check", "--dem=value"],
        &format!(
            "{CHECK_USAGE}proselint [options] <command> check: error: argument --demo: ignored explicit argument 'value'\n"
        ),
    );
}

#[test]
fn attached_dump_config_flag_prefix_reports_argparse_error() {
    assert_error(
        &["dump-config", "--def=value"],
        &format!(
            "{DUMP_CONFIG_USAGE}proselint [options] <command> dump-config: error: argument --default: ignored explicit argument 'value'\n"
        ),
    );
}

#[test]
fn attached_output_format_prefix_remains_valid() {
    let output = Workspace::new().run(&["check", "--out=full", "input.txt"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn attached_root_config_prefix_remains_valid() {
    let output = Workspace::new().run(&["--con=config.json", "check", "input.txt"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
