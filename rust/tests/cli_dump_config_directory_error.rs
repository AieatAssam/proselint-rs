use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-dump-config-dir-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated config directories");
        fs::create_dir(path.join("config directory")).expect("create explicit config directory");
        Self(path)
    }

    fn directory(&self) -> String {
        self.0
            .join("config directory")
            .canonicalize()
            .expect("resolve config directory")
            .display()
            .to_string()
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("home/xdg"))
            .args(args)
            .output()
            .expect("run native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn directory_error(directory: &str) -> String {
    format!("[Errno Directories not permitted - found directory %s] {directory}")
}

#[test]
fn dump_config_directory_json_error_matches_python() {
    let workspace = Workspace::new();
    let directory = workspace.directory();
    let output = workspace.run(&[
        "dump-config",
        "--config",
        "config directory",
        "--output-format",
        "json",
    ]);
    let expected = format!(
        "{{\n  \"error\": {{\n    \"code\": -31999,\n    \"data\": \"OSError\",\n    \"message\": \"{}\"\n  }}\n}}",
        directory_error(&directory)
    );
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn dump_config_directory_full_error_matches_python() {
    let workspace = Workspace::new();
    let directory = workspace.directory();
    let output = workspace.run(&[
        "dump-config",
        "--config",
        "config directory",
        "--output-format",
        "full",
    ]);
    let expected = format!(
        "Unrecoverable error encountered while running proselint: {}\n",
        directory_error(&directory)
    );
    assert_eq!(output.status.code(), Some(4), "stdout: {:?}", output.stdout);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert_eq!(output.stderr, expected.as_bytes());
}
