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
            "proselint-rs-file-symlink-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create file-symlink workspace");
        fs::write(
            path.join("config.json"),
            br#"{"max_errors":10,"checks":{"misc":{"apologizing":true}}}"#,
        )
        .expect("write selected check config");
        fs::write(
            path.join("target.txt"),
            b"Our policy states: More research is needed.",
        )
        .expect("write symlink target");
        Self(path.canonicalize().expect("canonicalize workspace"))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
#[test]
fn compact_output_uses_resolved_file_symlink_name() {
    let workspace = Workspace::new();
    std::os::unix::fs::symlink("target.txt", workspace.0.join("alias.md"))
        .expect("create file symlink");

    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "compact",
            "alias.md",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI");

    assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"target.txt:1:20: misc.apologizing: Excessive apologizing.\n"
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}
