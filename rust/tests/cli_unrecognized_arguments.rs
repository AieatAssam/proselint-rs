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
            "proselint-rs-unrecognized-args-{}-{serial}",
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

fn assert_unrecognized(args: &[&str], values: &str) {
    let output = Workspace::new().run(args);
    assert_eq!(output.status.code(), Some(2), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let expected = format!(
        "usage: proselint [options] <command>\nproselint: error: unrecognized arguments: {values}\n"
    );
    assert_eq!(output.stderr, expected.as_bytes());
}

#[test]
fn root_unknown_option_matches_argparse_error() {
    assert_unrecognized(&["--not-a-real-option"], "--not-a-real-option");
}

#[test]
fn check_unknown_option_matches_root_usage_and_error() {
    assert_unrecognized(&["check", "--not-a-real-option"], "--not-a-real-option");
}

#[test]
fn version_unknown_option_matches_root_usage_and_error() {
    assert_unrecognized(&["version", "--not-a-real-option"], "--not-a-real-option");
}

#[test]
fn dump_config_unknown_option_matches_root_usage_and_error() {
    assert_unrecognized(
        &["dump-config", "--not-a-real-option"],
        "--not-a-real-option",
    );
}

#[test]
fn short_unknown_option_matches_argparse_error() {
    assert_unrecognized(&["check", "-Z"], "-Z");
}

#[test]
fn unknown_option_with_attached_value_is_reported_as_one_argument() {
    assert_unrecognized(
        &["check", "--not-a-real-option=value"],
        "--not-a-real-option=value",
    );
}

#[test]
fn unknown_long_option_starting_with_o_is_not_a_short_o_option() {
    assert_unrecognized(&["check", "--other"], "--other");
}

#[test]
fn multiple_unknown_options_are_reported_in_argument_order() {
    assert_unrecognized(
        &["check", "--first-unknown", "--second-unknown"],
        "--first-unknown --second-unknown",
    );
}

#[test]
fn an_option_for_another_subcommand_is_unrecognized() {
    assert_unrecognized(&["version", "--demo"], "--demo");
    assert_unrecognized(&["check", "--default"], "--default");
}

#[test]
fn help_takes_precedence_over_a_later_unknown_option() {
    let output = Workspace::new().run(&["check", "--help", "--not-a-real-option"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .starts_with("usage: proselint [options] <command> check"),
        "stdout: {:?}",
        output.stdout
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn double_dash_keeps_a_flag_looking_path_as_a_path() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("--not-a-real-option"), b"")
        .expect("write dash-prefixed input file");
    let output = workspace.run(&["check", "--", "--not-a-real-option"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
