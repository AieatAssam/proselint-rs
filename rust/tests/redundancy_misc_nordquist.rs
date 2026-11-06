use serde_json::{Value, json};
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
            "proselint-rs-nordquist-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(input: &str, max_errors: usize) -> (i32, Value) {
    let workspace = Workspace::new();
    let dir = workspace.path();
    std::fs::write(dir.join("config.json"), format!(r#"{{"max_errors":{max_errors},"checks":{{"redundancy":{{"misc":{{"nordquist":true}}}}}}}}"#)).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(dir)
        .args([
            "check",
            "--config",
            "config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}

fn d(span: [usize; 2], pos: usize, message: &str, replacement: Option<&str>) -> Value {
    json!({"check_path":"redundancy.misc.nordquist","message":message,"span":span,"pos":[1,pos],"replacements":replacement})
}

#[test]
fn nordquist_cli_oracle_cases() {
    let cases = [
        (
            "absolutely essential",
            vec![d(
                [1, 21],
                1,
                "Redundancy. Use 'essential' instead of 'absolutely essential'.",
                Some("essential"),
            )],
        ),
        (
            "Absolutely Necessary",
            vec![d(
                [1, 21],
                1,
                "Redundancy. Use 'necessary' instead of 'Absolutely Necessary'.",
                Some("necessary"),
            )],
        ),
        (
            "absolutely necessary; absolutely essential; absolutely necessary",
            vec![
                d(
                    [1, 21],
                    1,
                    "Redundancy. Use 'necessary' instead of 'absolutely necessary'.",
                    Some("necessary"),
                ),
                d(
                    [23, 43],
                    23,
                    "Redundancy. Use 'essential' instead of 'absolutely essential'.",
                    Some("essential"),
                ),
                d(
                    [45, 65],
                    45,
                    "Redundancy. Use 'necessary' instead of 'absolutely necessary'.",
                    Some("necessary"),
                ),
            ],
        ),
        (
            "a.m. in the morning",
            vec![d(
                [1, 20],
                1,
                "Redundancy. Use 'None' instead of 'a.m. in the morning'.",
                None,
            )],
        ),
        (
            "p.m. at night",
            vec![d(
                [1, 14],
                1,
                "Redundancy. Use 'None' instead of 'p.m. at night'.",
                None,
            )],
        ),
        ("xabsolutely essential; absolutely essentialx", vec![]),
        (
            "'absolutely essential' and absolutely necessary",
            vec![d(
                [28, 48],
                28,
                "Redundancy. Use 'necessary' instead of 'absolutely necessary'.",
                Some("necessary"),
            )],
        ),
        ("'absolutely essential'", vec![]),
        (
            "Absolutely essential",
            vec![d(
                [1, 21],
                1,
                "Redundancy. Use 'essential' instead of 'Absolutely essential'.",
                Some("essential"),
            )],
        ),
        (
            "absolutely necessary; absolutely essential; absolutely necessary",
            vec![
                d(
                    [1, 21],
                    1,
                    "Redundancy. Use 'necessary' instead of 'absolutely necessary'.",
                    Some("necessary"),
                ),
                d(
                    [45, 65],
                    45,
                    "Redundancy. Use 'necessary' instead of 'absolutely necessary'.",
                    Some("necessary"),
                ),
            ],
        ),
        ("", vec![]),
    ];
    for (i, (input, diagnostics)) in cases.iter().enumerate() {
        let (status, result) = run(input, if i == 9 { 2 } else { 100 });
        assert_eq!(
            status,
            if diagnostics.is_empty() { 0 } else { 1 },
            "case {i}"
        );
        assert_eq!(
            result,
            json!({"result":{"<stdin>":{"diagnostics":diagnostics}}}),
            "case {i}: {input}"
        );
    }
}
