use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MISSING_CONFIG_ERROR: &[u8] = b"usage: proselint [options] <command> check [-h] [--config CONFIG]\n                                           [--output-format {full,json,compact}]\n                                           [--verbose] [--version] [--demo]\n                                           [paths ...]\nproselint [options] <command> check: error: argument --config: expected one argument\n";
const INVALID_OUTPUT_FORMAT_ERROR: &[u8] = b"usage: proselint [options] <command> check [-h] [--config CONFIG]\n                                           [--output-format {full,json,compact}]\n                                           [--verbose] [--version] [--demo]\n                                           [paths ...]\nproselint [options] <command> check: error: argument --output-format/-o: invalid OutputFormat value: 'invalid'\n";
const TOP_LEVEL_HELP: &[u8] = b"usage: proselint [options] <command>\n\nproselint, a linter for prose.\n\noptions:\n  -h, --help            show this help message and exit\n  --config CONFIG       path to a configuration file (`.proselintrc.json`)\n  --output-format {full,json,compact}, -o {full,json,compact}\n                        the format to display results in\n  --verbose, -v         enable verbose logging\n  --version             display the current version and exit\n\ncommands:\n  \n    version             display the current version and exit\n    check               run proselint against paths\n    dump-config         display the loaded configuration and exit\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-help-precedence-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str], cwd: Option<&Workspace>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(workspace) = cwd {
        command.current_dir(&workspace.0);
    }
    command.output().expect("run native CLI")
}

#[test]
fn missing_check_config_value_precedes_help() {
    let output = run(&["check", "--config", "--help"], None);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, MISSING_CONFIG_ERROR);
}

#[test]
fn invalid_check_output_format_precedes_help() {
    let output = run(&["check", "--output-format", "invalid", "--help"], None);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, INVALID_OUTPUT_FORMAT_ERROR);
}

#[test]
fn top_level_config_value_check_precedes_help() {
    let output = run(&["--config", "check", "--help"], None);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    assert_eq!(output.stdout, TOP_LEVEL_HELP);
}

#[test]
fn check_delimiter_allows_existing_help_named_file() {
    let workspace = Workspace::new();
    std::fs::write(workspace.0.join("--help"), b"").expect("create input file named --help");

    let output = run(&["check", "--", "--help"], Some(&workspace));
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");
}
