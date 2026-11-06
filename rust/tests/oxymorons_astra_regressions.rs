use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-oxymorons-astra-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(input: &str, max_errors: usize) -> (i32, Value) {
    let workspace = Workspace::new();
    std::fs::write(
        workspace.0.join("config.json"),
        format!(r#"{{"max_errors":{max_errors},"checks":{{"oxymorons":true}}}}"#),
    )
    .expect("write selected-check config");
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
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
        .expect("run native CLI");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for native CLI");
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code().expect("CLI exit code"),
        serde_json::from_slice(&output.stdout).expect("CLI JSON output"),
    )
}

#[test]
fn max_errors_truncates_pattern_group_order_before_position_sort() {
    let (code, output) = run("amateur expert; organized mess; amateur expert", 2);
    assert_eq!(code, 1);
    assert_eq!(
        output["result"]["<stdin>"]["diagnostics"],
        json!([
            {"check_path":"oxymorons","message":"'amateur expert' is an oxymoron.","span":[1,15],"pos":[1,1],"replacements":null},
            {"check_path":"oxymorons","message":"'amateur expert' is an oxymoron.","span":[33,47],"pos":[1,33],"replacements":null}
        ])
    );
}

#[test]
fn long_s_is_not_an_ascii_word_boundary_for_activation() {
    let (code, output) = run("increasingly leſſ; organized meſſ", 1000);
    assert_eq!(code, 0);
    assert_eq!(output["result"]["<stdin>"]["diagnostics"], json!([]));
}

#[test]
fn optional_ascii_s_can_backtrack_at_long_s_boundary() {
    let (code, output) = run("advancing backwardſX", 1000);
    assert_eq!(code, 1);
    assert_eq!(
        output["result"]["<stdin>"]["diagnostics"],
        json!([
            {"check_path":"oxymorons","message":"'advancing backwardſ' is an oxymoron.","span":[1,20],"pos":[1,1],"replacements":null}
        ])
    );
}
