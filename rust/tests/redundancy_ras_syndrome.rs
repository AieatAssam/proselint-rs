use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);
impl Workspace {
    fn new(max_errors: usize) -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-ras-syndrome-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(
                r#"{{"max_errors":{max_errors},"checks":{{"redundancy":{{"ras_syndrome":true}}}}}}"#
            ),
        )
        .expect("write selected config");
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(input: &str, max_errors: usize) -> Output {
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
        .expect("write stdin");
    child.wait_with_output().expect("wait for CLI")
}

fn diagnostic(start: usize, end: usize, original: &str, replacement: Option<&str>) -> Value {
    let shown = replacement.unwrap_or("None");
    json!({
        "check_path":"redundancy.ras_syndrome",
        "message":format!("RAS syndrome. Use '{shown}' instead of '{original}'."),
        "pos":[1,start],
        "replacements":replacement,
        "span":[start,end]
    })
}

fn assert_case(input: &str, max_errors: usize, exit: i32, expected: Vec<Value>) {
    let output = run(input, max_errors);
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(
        actual,
        json!({"result":{"<stdin>":{"diagnostics":expected}}})
    );
}

#[test]
fn all_twenty_source_entries_match_the_pinned_output() {
    let input = "abm missile; act test; abm missiles; abs braking system; atm machine; cd disc; cpi Index; gps system; gui interface; hiv virus; isbn number; lcd display; pdf format; pin number; ras syndrome; rip in peace; please RSVP; salT talks; sat test; upc codes.";
    let expected = [
        (1, 12, "abm missile", Some("ABM")),
        (14, 22, "act test", Some("ACT")),
        (24, 35, "abm missile", Some("ABM")),
        (24, 36, "abm missiles", Some("ABMs")),
        (38, 56, "abs braking system", Some("ABS")),
        (58, 69, "atm machine", Some("ATM")),
        (71, 78, "cd disc", Some("CD")),
        (80, 89, "cpi Index", None),
        (91, 101, "gps system", Some("GPS")),
        (103, 116, "gui interface", Some("GUI")),
        (118, 127, "hiv virus", Some("HIV")),
        (129, 140, "isbn number", Some("ISBN")),
        (142, 153, "lcd display", Some("LCD")),
        (155, 165, "pdf format", Some("PDF")),
        (167, 177, "pin number", Some("PIN")),
        (179, 191, "ras syndrome", Some("RAS")),
        (193, 205, "rip in peace", Some("RIP")),
        (207, 218, "please RSVP", None),
        (220, 230, "salT talks", None),
        (232, 240, "sat test", Some("SAT")),
        (242, 251, "upc codes", Some("UPC")),
    ];
    assert_case(
        input,
        100,
        1,
        expected
            .into_iter()
            .map(|(start, end, original, replacement)| {
                diagnostic(start, end, original, replacement)
            })
            .collect(),
    );
}

#[test]
fn correct_acronym_forms_are_clean() {
    assert_case(
        "ABM; ACT; ABMs; ABS; ATM; CD; CPI; GPS; GUI; HIV; ISBN; LCD; PDF; PIN; RAS; RIP; RSVP; SALT; SAT; UPC.",
        100,
        0,
        vec![],
    );
}

#[test]
fn case_insensitive_forms_keep_each_occurrence() {
    assert_case(
        "ABM missile; abm missile; PIN number; pin NUMBER.",
        100,
        1,
        vec![
            diagnostic(1, 12, "ABM missile", Some("ABM")),
            diagnostic(14, 25, "abm missile", Some("ABM")),
            diagnostic(27, 37, "PIN number", Some("PIN")),
            diagnostic(39, 49, "pin NUMBER", Some("PIN")),
        ],
    );
}

#[test]
fn mixed_case_source_keys_keep_the_pinned_missing_replacement() {
    let forms = [
        (1, 10, "cpi Index"),
        (12, 23, "please RSVP"),
        (25, 35, "salT talks"),
        (37, 46, "CPI INDEX"),
        (48, 59, "PLEASE RSVP"),
        (61, 71, "SALT TALKS"),
    ];
    assert_case(
        "cpi Index; please RSVP; salT talks; CPI INDEX; PLEASE RSVP; SALT TALKS.",
        100,
        1,
        forms
            .into_iter()
            .map(|(start, end, original)| diagnostic(start, end, original, None))
            .collect(),
    );
}

#[test]
fn paired_quotes_suppress_findings() {
    assert_case("He said \"pin number\" and 'abm missile'.", 100, 0, vec![]);
}

#[test]
fn unicode_prefix_uses_python_character_columns() {
    assert_case(
        "é😀 pin number.",
        100,
        1,
        vec![diagnostic(4, 14, "pin number", Some("PIN"))],
    );
}

#[test]
fn max_errors_keeps_the_first_active_pattern_group() {
    assert_case(
        "pin number; atm machine; PIN NUMBER",
        2,
        1,
        vec![
            diagnostic(1, 11, "pin number", Some("PIN")),
            diagnostic(26, 36, "PIN NUMBER", Some("PIN")),
        ],
    );
}

#[test]
fn plural_form_activates_without_the_singular_raw_overlap() {
    assert_case(
        "abm missiles",
        100,
        1,
        vec![diagnostic(1, 13, "abm missiles", Some("ABMs"))],
    );
}
