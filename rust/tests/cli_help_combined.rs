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
fn trailing_path_after_help_is_ignored() {
    assert_help(&["check", "--help", "ghost.md"]);
}

#[test]
fn subcommand_config_error_is_preempted_by_help() {
    assert_help(&[
        "check",
        "--config",
        "missing config.json",
        "--help",
        "ghost.md",
    ]);
}

#[test]
fn global_config_error_is_preempted_by_check_help() {
    assert_help(&["--config", "missing config.json", "check", "--help"]);
}

fn assert_help(args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .args(args)
        .output()
        .expect("run native CLI");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, CHECK_HELP);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
