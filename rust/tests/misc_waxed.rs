use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);
impl Workspace {
    fn new(max_errors: usize) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-waxed-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"waxed":true}}}}}}"#),
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
fn run_check(input: &str, max_errors: usize) -> Output {
    let workspace = Workspace::new(max_errors);
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
fn diagnostic(span: [usize; 2], replacement: &str) -> Value {
    json!({
        "check_path":"misc.waxed",
        "message":format!("The modifier following 'waxed' must be an adj.: '{}' is correct.", replacement),
        "pos":[1,span[0]],
        "replacements":replacement,
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
fn all_sixty_four_modifier_pairs_have_the_pinned_replacements() {
    let modifiers = [
        ("ebulliently", "ebullient"),
        ("ecstatically", "ecstatic"),
        ("eloquently", "eloquent"),
        ("enthusiastically", "enthusiastic"),
        ("euphorically", "euphoric"),
        ("indignantly", "indignant"),
        ("lyrically", "lyrical"),
        ("melancholically", "melancholic"),
        ("metaphorically", "metaphorical"),
        ("nostalgically", "nostalgic"),
        ("patriotically", "patriotic"),
        ("philosophically", "philosophical"),
        ("poetically", "poetic"),
        ("rhapsodically", "rhapsodic"),
        ("romantically", "romantic"),
        ("sentimentally", "sentimental"),
    ];
    let waxes = ["wax", "waxes", "waxed", "waxing"];
    let phrases: Vec<_> = waxes
        .iter()
        .flat_map(|wax| modifiers.iter().map(move |(bad, good)| (*wax, *bad, *good)))
        .collect();
    let input = phrases
        .iter()
        .map(|(wax, bad, _)| format!("{wax} {bad}"))
        .collect::<Vec<_>>()
        .join("; ");
    let mut start = 1;
    let expected = phrases
        .iter()
        .map(|(wax, bad, good)| {
            let replacement = format!("{wax} {good}");
            let diagnostic = diagnostic([start, start + wax.len() + 1 + bad.len()], &replacement);
            start += wax.len() + 1 + bad.len() + 2;
            diagnostic
        })
        .collect();
    assert_result(&input, 1, expected);
}

#[test]
fn correct_adjectives_and_longer_neighbor_words_are_clean() {
    assert_result(
        "He waxed enthusiastic, waxes lyrical, wax is ebullient, and waxing patriotic; unwaxed enthusiastically; waxedlyrical; waxworks.",
        0,
        vec![],
    );
}

#[test]
fn raw_activation_and_simple_form_case_behavior_match_the_pinned_check() {
    assert_result(
        "WAXED LYRICALLY; unwaxed enthusiastically; waxedlyrical; waxworks; waxing enthusiastically.",
        1,
        vec![
            diagnostic([1, 16], "waxed lyrical"),
            diagnostic([68, 91], "waxing enthusiastic"),
        ],
    );
}

#[test]
fn quoted_forms_are_suppressed() {
    assert_result("He said, \"waxed lyrically\".", 0, vec![]);
}

#[test]
fn max_errors_is_applied_in_source_order() {
    let out = run_check(
        "waxed lyrically; waxed ecstatically; waxed nostalgically.",
        1,
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {:?}", out.stderr);
    assert!(out.stderr.is_empty(), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[diagnostic([1,16],"waxed lyrical")]}}})
    );
}

#[test]
fn max_errors_preserves_pinned_modifier_group_order() {
    let input = "waxed lyrically; waxed ecstatically; waxed lyrically";
    let out = run_check(input, 2);
    assert_eq!(out.status.code(), Some(1), "stderr: {:?}", out.stderr);
    let actual: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":[
            diagnostic([1,16],"waxed lyrical"),
            diagnostic([38,53],"waxed lyrical")
        ]}}})
    );
}

#[test]
fn unicode_prefix_uses_python_character_columns() {
    assert_result(
        "é😀 Waxed philosophically.",
        1,
        vec![diagnostic([4, 25], "waxed philosophical")],
    );
}
