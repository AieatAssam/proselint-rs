use serde_json::Value;
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
            "proselint-rs-empty-home-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated config workspace");
        fs::create_dir_all(path.join(".config/proselint")).expect("create cwd config directory");
        fs::write(
            path.join(".config/proselint/config.json"),
            b"{\"checks\":{}}\n",
        )
        .expect("write cwd config sentinel");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(workspace: &Workspace, home: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args(args)
        .env("HOME", home)
        .env_remove("XDG_CONFIG_HOME")
        .stdin(Stdio::null())
        .output()
        .expect("run CLI")
}

#[test]
fn empty_home_dump_config_ignores_cwd_dot_config() {
    let workspace = Workspace::new();
    let automatic = run(&workspace, "", &["dump-config"]);
    let bundled = run(&workspace, "", &["dump-config", "--default"]);

    assert_eq!(automatic.status.code(), Some(0));
    assert_eq!(bundled.status.code(), Some(0));
    assert_eq!(automatic.stdout, bundled.stdout);
    assert!(
        automatic.stderr.is_empty(),
        "stderr: {:?}",
        automatic.stderr
    );
}

#[test]
fn empty_home_check_ignores_cwd_dot_config() {
    let workspace = Workspace::new();
    fs::write(
        workspace.0.join("input.txt"),
        "Suddenly, something happened.\n",
    )
    .expect("write input");

    let automatic = run(
        &workspace,
        "",
        &["check", "input.txt", "--output-format", "json"],
    );
    let isolated_home = workspace.0.join("ordinary-home");
    let bundled = run(
        &workspace,
        isolated_home
            .to_str()
            .expect("temporary home path is UTF-8"),
        &["check", "input.txt", "--output-format", "json"],
    );

    assert_eq!(automatic.status.code(), bundled.status.code());
    let bundled_json: Value = serde_json::from_slice(&bundled.stdout)
        .unwrap_or_else(|error| panic!("JSON output: {error}; {:?}", bundled.stdout));
    assert!(
        !bundled_json["result"]
            .as_object()
            .expect("result object")
            .values()
            .next()
            .expect("one checked file")["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .is_empty()
    );
    assert_eq!(automatic.stdout, bundled.stdout);
    assert!(
        automatic.stderr.is_empty(),
        "stderr: {:?}",
        automatic.stderr
    );
}
