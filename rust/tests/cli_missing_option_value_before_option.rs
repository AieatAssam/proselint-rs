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
            "proselint-rs-missing-option-value-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated workspace");
        Self(path)
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_proselint"))
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

fn assert_missing_value(args: &[&str], usage: &[u8], error_prefix: &str, error: &str) {
    let output = Workspace::new().run(args);
    assert_eq!(output.status.code(), Some(2), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let mut expected = usage.to_vec();
    expected.extend_from_slice(
        format!("{error_prefix}: error: argument {error}: expected one argument\n").as_bytes(),
    );
    assert_eq!(output.stderr, expected);
}

const ROOT_USAGE: &[u8] = b"usage: proselint [options] <command>\n";
const CHECK_USAGE: &[u8] = b"usage: proselint [options] <command> check [-h] [--config CONFIG]\n                                           [--output-format {full,json,compact}]\n                                           [--verbose] [--version] [--demo]\n                                           [paths ...]\n";

#[test]
fn root_config_followed_by_option_like_token_reports_missing_value() {
    assert_missing_value(
        &["--config", "--bogus", "check"],
        ROOT_USAGE,
        "proselint",
        "--config",
    );
}

#[test]
fn check_config_followed_by_option_like_token_reports_missing_value() {
    assert_missing_value(
        &["check", "--config", "--bogus"],
        CHECK_USAGE,
        "proselint [options] <command> check",
        "--config",
    );
}

#[test]
fn output_format_followed_by_option_like_token_reports_missing_value() {
    assert_missing_value(
        &["check", "--output-format", "--bogus"],
        CHECK_USAGE,
        "proselint [options] <command> check",
        "--output-format/-o",
    );
}

#[test]
fn short_output_format_followed_by_option_like_token_reports_missing_value() {
    assert_missing_value(
        &["check", "-o", "--bogus"],
        CHECK_USAGE,
        "proselint [options] <command> check",
        "--output-format/-o",
    );
}

#[test]
fn output_format_help_tokens_are_reported_as_missing_values_before_help() {
    for args in [
        ["check", "--output-format", "--help"],
        ["check", "-o", "--help"],
    ] {
        assert_missing_value(
            &args,
            CHECK_USAGE,
            "proselint [options] <command> check",
            "--output-format/-o",
        );
    }
}

#[test]
fn config_accepts_argparse_negative_number_and_dash_values() {
    for value in ["-", "-1", "-1.5", "-١"] {
        let output = Workspace::new().run(&["check", "--config", value]);
        assert_eq!(
            output.status.code(),
            Some(4),
            "value {value}: {:?}",
            output.stderr
        );
        assert!(
            output.stdout.is_empty(),
            "value {value}: {:?}",
            output.stdout
        );
        assert_eq!(
            output.stderr,
            format!("Unrecoverable error encountered while running proselint: Configuration file '{value}' could not be read.\n").as_bytes()
        );
    }
}

#[test]
fn output_format_accepts_argparse_negative_number_and_dash_values() {
    for value in ["-", "-1", "-.5", "-١"] {
        let output = Workspace::new().run(&["check", "--output-format", value]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "value {value}: {:?}",
            output.stderr
        );
        assert!(
            output.stdout.is_empty(),
            "value {value}: {:?}",
            output.stdout
        );
        let expected = format!(
            "{CHECK_USAGE_TEXT}proselint [options] <command> check: error: argument --output-format/-o: invalid OutputFormat value: '{value}'\n",
            CHECK_USAGE_TEXT = String::from_utf8_lossy(CHECK_USAGE),
        );
        assert_eq!(output.stderr, expected.as_bytes());
    }
}
