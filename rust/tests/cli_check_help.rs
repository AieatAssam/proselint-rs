use std::process::Command;

const CHECK_HELP: &[u8] = br#"usage: proselint [options] <command> check [-h] [--config CONFIG]
                                           [--output-format {full,json,compact}]
                                           [--verbose] [--version] [--demo]
                                           [paths ...]

positional arguments:
  paths                 target paths to lint

options:
  -h, --help            show this help message and exit
  --config CONFIG       path to a configuration file (`.proselintrc.json`)
  --output-format {full,json,compact}, -o {full,json,compact}
                        the format to display results in
  --verbose, -v         enable verbose logging
  --version             display the current version and exit
  --demo                run proselint against the demo file
"#;

#[test]
fn check_long_help_matches_pinned_bytes() {
    assert_help(&["check", "--help"]);
}

#[test]
fn check_short_help_matches_pinned_bytes() {
    assert_help(&["check", "-h"]);
}

fn assert_help(args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .args(args)
        .output()
        .expect("run native CLI");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, CHECK_HELP);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
