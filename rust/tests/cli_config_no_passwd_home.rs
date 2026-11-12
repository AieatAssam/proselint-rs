use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-no-passwd-home-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated workspace");
        fs::write(path.join("input.txt"), b"Hello world.").expect("write input");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_without_passwd_home(
    workspace: &Workspace,
    args: &[&str],
    xdg_home: Option<&str>,
) -> std::process::Output {
    let mut command = Command::new("unshare");
    command.args([
        "--user",
        "--map-user=424242",
        "--map-group=424242",
        "--",
        "/usr/bin/env",
        "-u",
        "HOME",
    ]);
    if let Some(xdg_home) = xdg_home {
        command.arg(format!("XDG_CONFIG_HOME={xdg_home}"));
    } else {
        command.args(["-u", "XDG_CONFIG_HOME"]);
    }
    command
        .arg(env!("CARGO_BIN_EXE_proselint-rs"))
        .args(args)
        .current_dir(&workspace.0)
        .output()
        .expect("run CLI under a UID with no passwd home")
}

#[test]
#[cfg(target_os = "linux")]
fn missing_passwd_home_is_a_startup_error() {
    let workspace = Workspace::new();
    let output = run_without_passwd_home(
        &workspace,
        &["check", "--output-format", "json", "input.txt"],
        None,
    );

    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(output.stderr, b"Could not determine home directory\n");
}

#[test]
#[cfg(target_os = "linux")]
fn explicit_config_does_not_require_a_passwd_home() {
    let workspace = Workspace::new();
    let config = workspace.0.join("explicit.json");
    fs::write(&config, br#"{"checks":{}}"#).expect("write explicit config");
    let config = config.to_str().expect("temporary path is UTF-8");
    let output = run_without_passwd_home(
        &workspace,
        &[
            "check",
            "--config",
            config,
            "--output-format",
            "compact",
            "input.txt",
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
#[cfg(target_os = "linux")]
fn passwd_home_is_required_before_cli_dispatch_even_with_xdg_config() {
    let workspace = Workspace::new();
    let xdg_home = workspace.0.join("xdg");
    fs::create_dir(&xdg_home).expect("create XDG config home");
    let xdg_home = xdg_home.to_str().expect("temporary path is UTF-8");
    let output = run_without_passwd_home(&workspace, &["--version"], Some(xdg_home));

    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(output.stderr, b"Could not determine home directory\n");
}
