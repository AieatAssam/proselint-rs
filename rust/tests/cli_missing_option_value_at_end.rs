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
            "proselint-rs-missing-option-at-end-{}-{serial}",
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

fn assert_missing_value(args: &[&str], usage: &[u8], error_prefix: &str, option: &str) {
    let output = Workspace::new().run(args);
    assert_eq!(output.status.code(), Some(2), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let mut expected = usage.to_vec();
    expected.extend_from_slice(
        format!("{error_prefix}: error: argument {option}: expected one argument\n").as_bytes(),
    );
    assert_eq!(output.stderr, expected);
}

const ROOT_USAGE: &[u8] = b"usage: proselint [options] <command>\n";
const CHECK_USAGE: &[u8] = b"usage: proselint [options] <command> check [-h] [--config CONFIG]\n                                           [--output-format {full,json,compact}]\n                                           [--verbose] [--version] [--demo]\n                                           [paths ...]\n";

#[test]
fn root_value_options_without_values_match_argparse() {
    for (option, error) in [
        ("--config", "--config"),
        ("--output-format", "--output-format/-o"),
        ("-o", "--output-format/-o"),
    ] {
        assert_missing_value(&[option], ROOT_USAGE, "proselint", error);
    }
}

#[test]
fn check_value_options_without_values_match_argparse() {
    for (option, error) in [
        ("--config", "--config"),
        ("--output-format", "--output-format/-o"),
        ("-o", "--output-format/-o"),
    ] {
        assert_missing_value(
            &["check", option],
            CHECK_USAGE,
            "proselint [options] <command> check",
            error,
        );
    }
}
