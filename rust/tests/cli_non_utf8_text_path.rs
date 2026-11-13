#[cfg(unix)]
mod unix {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    struct Workspace(PathBuf);

    impl Workspace {
        fn new() -> Self {
            let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "proselint-rs-non-utf8-text-{}-{serial}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create workspace");
            std::fs::create_dir(path.join("home")).expect("create isolated home");
            std::fs::create_dir(path.join("xdg")).expect("create isolated XDG config");
            std::fs::write(
                path.join("config.json"),
                br#"{"max_errors":1000,"checks":{"annotations":{"misc":true}}}"#,
            )
            .expect("write config");
            let name = OsString::from_vec(b"bad\xff.txt".to_vec());
            std::fs::write(path.join(name), b"TODO").expect("write source");
            Self(path.canonicalize().expect("canonicalize workspace"))
        }

        fn run(&self, format: &str) -> std::process::Output {
            self.run_path(format, OsString::from_vec(b"bad\xff.txt".to_vec()))
        }

        fn run_path(&self, format: &str, path: OsString) -> std::process::Output {
            Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
                .current_dir(&self.0)
                .args([
                    "check",
                    "--config",
                    "config.json",
                    "--output-format",
                    format,
                ])
                .arg(path)
                .env("HOME", self.0.join("home"))
                .env("XDG_CONFIG_HOME", self.0.join("xdg"))
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("run CLI")
        }
    }

    impl Drop for Workspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn full_output_preserves_non_utf8_filename_bytes() {
        let workspace = Workspace::new();
        let output = workspace.run("full");
        let mut expected = workspace.0.as_os_str().as_encoded_bytes().to_vec();
        expected
            .extend_from_slice(b"/bad\xff.txt:1:1: annotations.misc: Annotation left in text.\n");
        assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
        assert_eq!(output.stdout, expected);
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn compact_output_preserves_non_utf8_filename_bytes() {
        let workspace = Workspace::new();
        let output = workspace.run("compact");
        assert_eq!(output.status.code(), Some(1), "stderr: {:?}", output.stderr);
        assert_eq!(
            output.stdout,
            b"bad\xff.txt:1:1: annotations.misc: Annotation left in text.\n"
        );
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn absolute_per_file_pattern_normalizes_dot_components() {
        let workspace = Workspace::new();
        let source = workspace.0.join("draft.txt");
        std::fs::write(&source, b"TODO").unwrap();
        let pattern = format!("{}/./*.txt", workspace.0.display());
        std::fs::write(
            workspace.0.join("config.json"),
            serde_json::json!({
                "max_errors": 1000,
                "checks": {"annotations": {"misc": true}},
                "per_file_checks": {pattern: {"annotations": {"misc": false}}}
            })
            .to_string(),
        )
        .unwrap();
        let output = workspace.run_path("full", OsString::from("draft.txt"));
        assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
        assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
        assert!(output.stderr.is_empty());
    }
}
