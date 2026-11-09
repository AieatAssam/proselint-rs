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
            "proselint-rs-version-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated version workspace");
        let home = path.join("home");
        fs::create_dir_all(home.join("xdg")).expect("create isolated home and XDG config");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn version_subcommand_emits_exact_plain_text_without_config() {
    assert_version_output("version", b"Proselint 0.16.0\n");
}

#[test]
fn standalone_version_flag_reports_product_and_upstream_versions() {
    assert_version_output(
        "--version",
        b"proselint-rs 2026.11.08.1 (tracks Proselint 0.16.0)\n",
    );
}

fn assert_version_output(argument: &str, expected: &[u8]) {
    let workspace = Workspace::new();
    let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
    command
        .current_dir(&workspace.0)
        .env("HOME", workspace.0.join("home"))
        .env("XDG_CONFIG_HOME", workspace.0.join("home/xdg"))
        .arg(argument)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command.output().expect("run native CLI");

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, expected);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
