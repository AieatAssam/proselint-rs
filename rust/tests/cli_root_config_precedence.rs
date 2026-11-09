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
            "proselint-rs-root-config-precedence-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        fs::create_dir_all(path.join("home/xdg")).expect("create isolated config directories");
        fs::write(path.join("bad.json"), "{").expect("write malformed config");
        Self(path)
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("home/xdg"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("run native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn root_config_is_overridden_by_subcommand_default() {
    for args in [
        &["--config", "bad.json", "dump-config", "--version"][..],
        &["--config=bad.json", "version"][..],
        &["--config", "bad.json", "check"][..],
    ] {
        let output = Workspace::new().run(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "args: {args:?}, stderr: {:?}",
            output.stderr
        );
        if args.last() == Some(&"check") {
            assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
        } else {
            assert_eq!(output.stdout, b"Proselint 0.16.0\n");
        }
        assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    }
}

#[test]
fn subcommand_config_value_is_not_overridden() {
    let output = Workspace::new().run(&["dump-config", "--config", "bad.json", "--version"]);

    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Expecting property name enclosed in double quotes")
    );
}
