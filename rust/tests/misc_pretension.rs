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
            "proselint-rs-pretension-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            br#"{"max_errors":1000,"checks":{"misc":{"pretension":true}}}"#,
        )
        .expect("write selected config");
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
    child.wait_with_output().expect("wait for CLI")
}
fn diagnostic(span: [usize; 2], column: usize, term: &str) -> Value {
    json!({"check_path":"misc.pretension","message":format!("Jargon words like '{term}' are the hallmarks of a pretentious ass."),"pos":[1,column],"replacements":null,"span":span})
}
fn assert_result(input: &str, exit: i32, diagnostics: Vec<Value>) {
    let out = run_check(input);
    assert_eq!(out.status.code(), Some(exit), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}
#[test]
fn detects_each_of_the_four_terms() {
    assert_result(
        "reconceptualize; demassification; attitudinally; judgmentally.",
        1,
        vec![
            diagnostic([1, 16], 1, "reconceptualize"),
            diagnostic([18, 33], 18, "demassification"),
            diagnostic([35, 48], 35, "attitudinally"),
            diagnostic([50, 62], 50, "judgmentally"),
        ],
    );
}
#[test]
fn matching_is_case_insensitive_and_message_uses_surface_text() {
    assert_result(
        "RECONCEPTUALIZE; DeMassification; ATTITUDINALLY; Judgmentally.",
        1,
        vec![
            diagnostic([1, 16], 1, "RECONCEPTUALIZE"),
            diagnostic([18, 33], 18, "DeMassification"),
            diagnostic([35, 48], 35, "ATTITUDINALLY"),
            diagnostic([50, 62], 50, "Judgmentally"),
        ],
    );
}
#[test]
fn longer_word_boundaries_are_clean() {
    assert_result(
        "xreconceptualize; reconceptualizex; demassifications; attitudinallyx; xjudgmentally.",
        0,
        vec![],
    );
}
#[test]
fn punctuation_boundaries_allow_each_term() {
    assert_result(
        "(reconceptualize)! [demassification]? {attitudinally}, <judgmentally>.",
        1,
        vec![
            diagnostic([2, 17], 2, "reconceptualize"),
            diagnostic([21, 36], 21, "demassification"),
            diagnostic([40, 53], 40, "attitudinally"),
            diagnostic([57, 69], 57, "judgmentally"),
        ],
    );
}
#[test]
fn quoted_terms_are_suppressed() {
    assert_result(
        "Quoted \"reconceptualize\" and \"demassification\"; outside attitudinally and judgmentally.",
        1,
        vec![
            diagnostic([57, 70], 57, "attitudinally"),
            diagnostic([75, 87], 75, "judgmentally"),
        ],
    );
}
#[test]
fn repeated_terms_keep_source_order() {
    assert_result(
        "judgmentally; reconceptualize; judgmentally; demassification; attitudinally.",
        1,
        vec![
            diagnostic([1, 13], 1, "judgmentally"),
            diagnostic([15, 30], 15, "reconceptualize"),
            diagnostic([32, 44], 32, "judgmentally"),
            diagnostic([46, 61], 46, "demassification"),
            diagnostic([63, 76], 63, "attitudinally"),
        ],
    );
}
#[test]
fn unicode_prefixes_keep_oracle_codepoint_coordinates() {
    assert_result(
        "é before reconceptualize; e\u{301} before judgmentally.",
        1,
        vec![
            diagnostic([10, 25], 10, "reconceptualize"),
            diagnostic([37, 49], 37, "judgmentally"),
        ],
    );
}
#[test]
fn selected_config_excludes_unrelated_default_checks() {
    assert_result(
        "A very good sentence: reconceptualize the plan, TODO.",
        1,
        vec![diagnostic([23, 38], 23, "reconceptualize")],
    );
}

#[test]
fn unicode_casefold_regression_pretension() {
    assert_result(
        "demaſsification",
        1,
        vec![
            json!({"check_path":"misc.pretension","message":"Jargon words like 'demaſsification' are the hallmarks of a pretentious ass.","pos":[1,1],"replacements":null,"span":[1,16]}),
        ],
    );
}
