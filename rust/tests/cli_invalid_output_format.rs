use std::process::{Command, Output, Stdio};

const INVALID_OUTPUT_FORMAT_ERROR: &[u8] = b"usage: proselint [options] <command> check [-h] [--config CONFIG]\n                                           [--output-format {full,json,compact}]\n                                           [--verbose] [--version] [--demo]\n                                           [paths ...]\nproselint [options] <command> check: error: argument --output-format/-o: invalid OutputFormat value: 'invalid'\n";

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI")
}

#[test]
fn invalid_long_output_format_without_help_matches_argparse() {
    let output = run(&["check", "--output-format", "invalid"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(output.stderr, INVALID_OUTPUT_FORMAT_ERROR);
}

#[test]
fn invalid_short_output_format_without_help_matches_argparse() {
    let output = run(&["check", "-o", "invalid"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(output.stderr, INVALID_OUTPUT_FORMAT_ERROR);
}

#[test]
fn invalid_output_format_uses_version_subcommand_usage() {
    let expected = b"usage: proselint [options] <command> version [-h] [--config CONFIG]\n                                             [--output-format {full,json,compact}]\n                                             [--verbose] [--version]\nproselint [options] <command> version: error: argument --output-format/-o: invalid OutputFormat value: 'invalid'\n";
    let output = run(&["version", "--output-format", "invalid"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, expected);
}

#[test]
fn invalid_output_format_uses_dump_config_subcommand_usage() {
    let expected = b"usage: proselint [options] <command> dump-config [-h] [--config CONFIG]\n                                                 [--output-format {full,json,compact}]\n                                                 [--verbose] [--version]\n                                                 [--default]\nproselint [options] <command> dump-config: error: argument --output-format/-o: invalid OutputFormat value: 'invalid'\n";
    let output = run(&["dump-config", "--output-format", "invalid"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, expected);
}

#[test]
fn config_value_that_looks_like_a_subcommand_does_not_change_check_usage() {
    for value in ["version", "dump-config"] {
        let output = run(&["--config", value, "check", "--output-format", "invalid"]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, INVALID_OUTPUT_FORMAT_ERROR);
    }
}

#[test]
fn config_value_that_looks_like_check_keeps_top_level_usage() {
    let expected = b"usage: proselint [options] <command>\nproselint: error: argument --output-format/-o: invalid OutputFormat value: 'invalid'\n";
    let output = run(&["--config", "check", "--output-format", "invalid"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, expected);
}
