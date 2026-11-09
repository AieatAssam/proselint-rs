use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-user-config-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("child/grandchild")).expect("create isolated workspace");
        Self(path)
    }

    fn cwd(&self) -> PathBuf {
        self.0.join("child/grandchild")
    }

    fn write(&self, path: impl AsRef<Path>, contents: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn run(
        &self,
        xdg: Option<&str>,
        home: Option<&Path>,
        explicit: Option<&str>,
    ) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
        command
            .current_dir(self.cwd())
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("HOME")
            .args(["check", "--output-format", "json"]);
        if let Some(xdg) = xdg {
            command.env("XDG_CONFIG_HOME", xdg);
        }
        if let Some(home) = home {
            command.env("HOME", home);
        }
        if let Some(config) = explicit {
            command.args(["--config", config]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run native CLI");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"Our policy states: More research is needed.")
            .unwrap();
        child.wait_with_output().expect("wait for native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn disabled() -> &'static str {
    r#"{"checks":{}}"#
}

fn apologizing() -> &'static str {
    r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#
}

fn assert_diagnostics(output: &std::process::Output, status: i32, count: usize) {
    assert_eq!(
        output.status.code(),
        Some(status),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["result"]["<stdin>"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        count
    );
}

#[test]
fn discovers_xdg_user_config() {
    let workspace = Workspace::new();
    workspace.write("child/grandchild/xdg/proselint/config.json", apologizing());
    assert_diagnostics(&workspace.run(Some("xdg"), None, None), 1, 1);
}

#[test]
fn home_fallback_when_xdg_unset_or_empty() {
    for xdg in [None, Some("")] {
        let workspace = Workspace::new();
        let home = workspace.0.join("home");
        workspace.write("home/.config/proselint/config.json", apologizing());
        assert_diagnostics(&workspace.run(xdg, Some(&home), None), 1, 1);
    }
}

#[test]
fn cwd_and_ancestor_configs_precede_user_config() {
    let workspace = Workspace::new();
    workspace.write("child/grandchild/xdg/proselint/config.json", apologizing());
    workspace.write("child/grandchild/proselint.json", disabled());
    assert_diagnostics(&workspace.run(Some("xdg"), None, None), 0, 0);

    let workspace = Workspace::new();
    workspace.write("child/grandchild/xdg/proselint/config.json", apologizing());
    workspace.write("proselint.json", disabled());
    assert_diagnostics(&workspace.run(Some("xdg"), None, None), 0, 0);
}

#[test]
fn malformed_first_user_config_errors_without_fallback() {
    let workspace = Workspace::new();
    workspace.write("child/grandchild/xdg/proselint/config.json", "{");
    workspace.write("home/.config/proselint/config.json", disabled());
    let output = workspace.run(Some("xdg"), Some(&workspace.0.join("home")), None);
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["error"]["data"],
        "JSONDecodeError"
    );
}

#[test]
fn explicit_config_overrides_user_config() {
    let workspace = Workspace::new();
    workspace.write("child/grandchild/xdg/proselint/config.json", apologizing());
    workspace.write("child/grandchild/explicit.json", disabled());
    assert_diagnostics(
        &workspace.run(Some("xdg"), None, Some("explicit.json")),
        0,
        0,
    );
}

#[test]
fn relative_xdg_path_is_relative_to_process_cwd() {
    let workspace = Workspace::new();
    workspace.write(
        "child/grandchild/relative/proselint/config.json",
        apologizing(),
    );
    assert_diagnostics(&workspace.run(Some("relative"), None, None), 1, 1);
}
