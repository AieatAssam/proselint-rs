use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
const VERSION_HELP: &[u8] = include_bytes!("fixtures/command_help_version.bin");
const DUMP_CONFIG_HELP: &[u8] = include_bytes!("fixtures/command_help_dump_config.bin");

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-command-help-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated help workspace");
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated HOME and XDG config");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn version_subcommand_help_matches_pinned_bytes() {
    assert_help(&["version", "--help"], VERSION_HELP);
}

#[test]
fn dump_config_subcommand_help_matches_pinned_bytes() {
    assert_help(&["dump-config", "--help"], DUMP_CONFIG_HELP);
}

fn assert_help(args: &[&str], expected_stdout: &[u8]) {
    let workspace = Workspace::new();
    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace.0)
        .env("HOME", workspace.0.join("home"))
        .env("XDG_CONFIG_HOME", workspace.0.join("home/xdg"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected_stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
