use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);
impl Workspace {
    fn new(max_errors: usize) -> Self {
        Self::with_config(format!(
            r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"illogic":true}}}}}}"#
        ))
    }
    fn with_config(config: String) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-illogic-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(path.join("selected config.json"), config).expect("write selected config");
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
fn run_check(input: &str, max_errors: usize) -> Output {
    let workspace = Workspace::new(max_errors);
    run_in_workspace(&workspace, input)
}
fn run_in_workspace(workspace: &Workspace, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint-rs"))
        .current_dir(workspace.path())
        .args([
            "check",
            "--config",
            "selected config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native CLI");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input.as_bytes())
        .expect("write input");
    child.wait_with_output().expect("wait for CLI")
}
fn diagnostic(span: [usize; 2], matched: &str) -> Value {
    json!({
        "check_path":"misc.illogic",
        "message":format!("'{}' is illogical.", matched),
        "pos":[1,span[0]],
        "replacements":null,
        "span":span
    })
}
fn assert_result(input: &str, exit: i32, diagnostics: Vec<Value>) {
    let out = run_check(input, 1000);
    assert_eq!(out.status.code(), Some(exit), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}

#[test]
fn all_nine_pinned_patterns_report_exact_spans_and_messages() {
    assert_result(
        "Preplan. More than two all of us. Appraisal valuation. I could care less. Least worst. Much-needed gap. Much-needed void. No longer requires oxygen. Without scarcely.",
        1,
        vec![
            diagnostic([1, 8], "Preplan"),
            diagnostic([10, 27], "More than two all"),
            diagnostic([35, 54], "Appraisal valuation"),
            diagnostic([56, 73], "I could care less"),
            diagnostic([75, 86], "Least worst"),
            diagnostic([88, 103], "Much-needed gap"),
            diagnostic([105, 121], "Much-needed void"),
            diagnostic([123, 148], "No longer requires oxygen"),
            diagnostic([150, 166], "Without scarcely"),
        ],
    );
}

#[test]
fn case_insensitive_word_boundaries_and_regex_width_are_pinned() {
    assert_result(
        "PREPLAN; preplanning; xpreplan; preplanx; more than x all; more than 12345678901 all",
        1,
        vec![
            diagnostic([1, 8], "PREPLAN"),
            diagnostic([10, 17], "preplan"),
            diagnostic([24, 31], "preplan"),
            diagnostic([33, 40], "preplan"),
            diagnostic([43, 58], "more than x all"),
        ],
    );
}

#[test]
fn quoted_matches_are_suppressed_and_unicode_columns_are_character_based() {
    assert_result("\"preplan\" and 'without scarcely'", 0, vec![]);
    assert_result("é😀 Preplan.", 1, vec![diagnostic([4, 11], "Preplan")]);
}

#[test]
fn pattern_groups_keep_first_activation_order_before_max_errors() {
    let out = run_check("least worst; preplan; preplan", 2);
    assert_eq!(out.status.code(), Some(1), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[
            diagnostic([1, 12], "least worst"),
            diagnostic([14, 21], "preplan")
        ]}}})
    );
}

#[test]
fn parent_diagnostic_coexists_with_same_module_children() {
    let workspace = Workspace::with_config(
        r#"{"max_errors":1000,"checks":{"misc":{"illogic":true,"illogic.coin":true,"illogic.collusion":true}}}"#.to_owned(),
    );
    let out = run_in_workspace(
        &workspace,
        "preplan. To coin a phrase from history. Without your collusion.",
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[
            diagnostic([1, 8], "preplan"),
            json!({"check_path":"misc.illogic.coin","message":"You can't coin an existing phrase. Did you mean 'borrow'?","pos":[1,10],"replacements":null,"span":[10,31]}),
            json!({"check_path":"misc.illogic.collusion","message":"It's impossible to defraud yourself. Try 'aquiescence'.","pos":[1,41],"replacements":null,"span":[41,63]})
        ]}}})
    );
}
