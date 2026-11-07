use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-dump-config-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create isolated workspace");
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_proselint"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .expect("run native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn pinned_default() -> Value {
    json!({
        "checks": {
            "annotations": true,
            "archaism": true,
            "cliches": true,
            "dates_times": true,
            "hedging": true,
            "industrial_language": true,
            "lexical_illusions": true,
            "malapropisms": true,
            "misc": true,
            "mixed_metaphors": true,
            "mondegreens": true,
            "needless_variants": true,
            "nonwords": true,
            "oxymorons": true,
            "psychology": true,
            "redundancy": true,
            "restricted": false,
            "skunked_terms": true,
            "social_awareness": true,
            "spelling": true,
            "terms": true,
            "typography": false,
            "uncomparables": true,
            "weasel_words": true
        },
        "file_checks": {},
        "max_errors": 1000
    })
}

fn assert_default_output(output: Output) {
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        pinned_default()
    );
}

#[test]
fn dump_config_default_outputs_pinned_default_configuration() {
    assert_default_output(Workspace::new().run(&["dump-config", "--default"]));
}

#[test]
fn dump_config_default_ignores_a_valid_explicit_configuration() {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("explicit.json"), r#"{"checks":{}}"#).unwrap();
    assert_default_output(workspace.run(&[
        "dump-config",
        "--config",
        "explicit.json",
        "--default",
    ]));
}

#[test]
fn dump_config_outputs_flattened_explicit_configuration() {
    let workspace = Workspace::new();
    fs::write(
        workspace.0.join("explicit.json"),
        r#"{"max_errors":12,"checks":{"misc":{"apologizing":true,"many_a":false},"cliches":false},"per_file_checks":{"draft.md":{"typography":{"symbols":true}}}}"#,
    )
    .unwrap();
    let output = workspace.run(&["dump-config", "--config", "explicit.json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({
            "checks": {
                "cliches": false,
                "misc.apologizing": true,
                "misc.many_a": false
            },
            "max_errors": 12,
            "per_file_checks": {
                "draft.md": {
                    "cliches": false,
                    "misc.apologizing": true,
                    "misc.many_a": false,
                    "typography.symbols": true
                }
            }
        })
    );
}

#[test]
fn dump_config_escapes_non_ascii_check_keys_and_per_file_pattern() {
    let workspace = Workspace::new();
    fs::write(
        workspace.0.join("explicit.json"),
        r#"{"max_errors":7,"checks":{"café":{"雪":true,"naïve":false}},"per_file_checks":{"résumé-*.md":{"møde":{"façade":true}}}}"#,
    )
    .unwrap();
    let output = workspace.run(&["dump-config", "--config", "explicit.json"]);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        output.stdout,
        br#"{
    "checks": {
        "caf\u00e9.na\u00efve": false,
        "caf\u00e9.\u96ea": true
    },
    "max_errors": 7,
    "per_file_checks": {
        "r\u00e9sum\u00e9-*.md": {
            "caf\u00e9.na\u00efve": false,
            "caf\u00e9.\u96ea": true,
            "m\u00f8de.fa\u00e7ade": true
        }
    }
}
"#
    );
}
