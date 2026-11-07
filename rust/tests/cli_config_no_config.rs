use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-no-config-default-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated workspace");
        for name in ["home", "xdg"] {
            std::fs::create_dir(path.join(name)).expect("create isolated config home");
        }
        Self(path)
    }

    fn run(&self, input: &[u8]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_proselint"))
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("xdg"))
            .args(["check"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start native CLI");
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input)
            .expect("write stdin");
        child.wait_with_output().expect("wait for CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn no_config_found_uses_default_enabled_rule() {
    let output = Workspace::new().run(b"This is very good.");
    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"<stdin>:1:9: weasel_words.very: Substitute 'damn' every time you're inclined to write 'very'; your editor will delete it and the writing will be just as it should be.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_keeps_clean_input_clean() {
    let output = Workspace::new().run(b"An ordinary sentence.");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn no_config_found_keeps_default_disabled_curly_quotes_check_disabled() {
    let output = Workspace::new().run(b"The phrase \"word\" is quoted.");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
