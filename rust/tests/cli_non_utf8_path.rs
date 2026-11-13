#[cfg(unix)]
#[test]
fn json_file_uri_preserves_non_utf8_path_bytes() {
    use std::ffi::OsStr;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
    let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let workspace = std::env::temp_dir().join(format!(
        "proselint-rs-cli-non-utf8-{}-{serial}",
        std::process::id()
    ));
    fs::create_dir(&workspace).expect("create test workspace");
    let source = PathBuf::from(OsStr::from_bytes(b"bad\xff.txt"));
    fs::write(workspace.join(&source), b"ordinary prose").expect("write non-UTF-8 path");
    fs::write(
        workspace.join("empty.json"),
        br#"{"max_errors":1000,"checks":{}}"#,
    )
    .expect("write explicit all-disabled config");

    let output = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(&workspace)
        .args(["check", "--config", "empty.json", "--output-format", "json"])
        .arg(&source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let source_key = actual["result"]
        .as_object()
        .expect("result map")
        .keys()
        .next()
        .expect("source entry");
    assert!(
        source_key.ends_with("/bad%FF.txt"),
        "source URI should preserve invalid path byte: {source_key}"
    );

    fs::remove_dir_all(workspace).expect("remove test workspace");
}
