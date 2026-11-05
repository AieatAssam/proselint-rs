use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-professions-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"professions":true}}}"#,
        )
        .expect("write config selecting only misc.professions");
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

fn run_check(input: &str) -> Output {
    let workspace = Workspace::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_proselint"))
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
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], column: usize, replacement: &str) -> Value {
    json!({
        "check_path": "misc.professions",
        "message": format!("'{replacement}' is the name of that job."),
        "pos": [1, column],
        "replacements": replacement,
        "span": span,
    })
}

fn assert_result(input: &str, expected_exit: i32, diagnostics: Vec<Value>) {
    let output = run_check(input);
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        actual,
        json!({"result": {"<stdin>": {"diagnostics": diagnostics}}})
    );
}

#[test]
fn recommends_cobbler_for_shoe_repair_guy() {
    assert_result(
        "I really need a shoe repair guy.",
        1,
        vec![diagnostic([17, 32], 17, "cobbler")],
    );
}

#[test]
fn recommends_geometer_for_geometrist() {
    assert_result(
        "A geometrist joined the team.",
        1,
        vec![diagnostic([3, 13], 3, "geometer")],
    );
}

#[test]
fn maps_case_variants_in_source_order() {
    assert_result(
        "SHOE REPAIR GUY; Geometrist; shoe repair guy",
        1,
        vec![
            diagnostic([1, 16], 1, "cobbler"),
            diagnostic([18, 28], 18, "geometer"),
            diagnostic([30, 45], 30, "cobbler"),
        ],
    );
}

#[test]
fn word_padding_rejects_embedded_profession_terms() {
    assert_result(
        "xshoe repair guy; shoe repair guys; geometristic",
        0,
        vec![],
    );
}

#[test]
fn suppresses_profession_matches_inside_quotes() {
    assert_result("I called him a 'shoe repair guy'.", 0, vec![]);
}

#[test]
fn selected_config_does_not_enable_annotations() {
    assert_result(
        "A shoe repair guy and TODO.",
        1,
        vec![diagnostic([3, 18], 3, "cobbler")],
    );
}

#[test]
fn fastset_activation_preserves_embedded_raw_copy() {
    assert_result(
        "geometrist xgeometrist",
        1,
        vec![
            diagnostic([1, 11], 1, "geometer"),
            diagnostic([13, 23], 13, "geometer"),
        ],
    );
}

#[test]
fn unicode_casefold_regression_professions() {
    assert_result(
        "ſhoe repair guy shoe repair guy",
        1,
        vec![
            json!({"check_path":"misc.professions","message":"'None' is the name of that job.","pos":[1,1],"replacements":null,"span":[1,16]}),
            json!({"check_path":"misc.professions","message":"'cobbler' is the name of that job.","pos":[1,17],"replacements":"cobbler","span":[17,32]}),
        ],
    );
}
