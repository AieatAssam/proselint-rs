#![cfg(unix)]

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn directory_symlink_loop_error_omits_dot_path_component() {
    use std::os::unix::fs::symlink;

    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let workspace = std::env::temp_dir().join(format!(
        "proselint-rs-symlink-loop-{}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir(&workspace).expect("create workspace");
    std::fs::write(
        workspace.join("config.json"),
        r#"{"checks":{"annotations":{"misc":true}}}"#,
    )
    .expect("write config");
    symlink("loop.md", workspace.join("loop.md")).expect("create self-referencing symlink");

    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
            ".",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI");
    let _ = std::fs::remove_dir_all(&workspace);

    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(
        actual["error"]["message"],
        format!(
            "Symlink loop from '{}'",
            workspace.join("loop.md").display()
        )
    );
}
