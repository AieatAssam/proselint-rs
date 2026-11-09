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
            "proselint-rs-composition-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated test workspace");
        std::fs::write(
            path.join("selected config.json"),
            format!(r#"{{"max_errors":{max_errors},"checks":{{"misc":{{"composition":true}}}}}}"#),
        )
        .expect("write config selecting only misc.composition");
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
    child.wait_with_output().expect("wait for native CLI")
}

fn diagnostic(span: [usize; 2], replacement: &str, matched: &str) -> Value {
    json!({
        "check_path":"misc.composition",
        "message":format!("Try '{replacement}' instead of '{matched}'."),
        "span":span,
        "pos":[1,span[0]],
        "replacements":replacement,
    })
}

fn diagnostic_with_none_replacement(span: [usize; 2], matched: &str) -> Value {
    json!({
        "check_path":"misc.composition",
        "message":format!("Try 'None' instead of '{matched}'."),
        "span":span,
        "pos":[1,span[0]],
        "replacements":null,
    })
}

fn assert_result(input: &str, max_errors: usize, expected_exit: i32, diagnostics: Vec<Value>) {
    let output = run_check(input, max_errors);
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
        json!({"result":{"<stdin>":{"diagnostics":diagnostics}}})
    );
}

#[test]
fn reports_all_simple_forms_and_keeps_cross_registration_duplicates() {
    assert_result(
        "not honest; not important; did not remember; did not pay attention to; did not have much confidence in; the question as to whether; there is no doubt but that; used for fuel purposes; he is a man who; in a hasty manner; this is a subject that; a strange one; the reason why is that; owing to the fact that; in spite of the fact that; call your attention to the fact that; was unaware of the fact; did not succeed; not succeed; the fact that i had arrived.",
        1000,
        1,
        vec![
            diagnostic([1, 11], "dishonest", "not honest"),
            diagnostic([13, 26], "unimportant", "not important"),
            diagnostic([28, 44], "forgot", "did not remember"),
            diagnostic([46, 70], "ignored", "did not pay attention to"),
            diagnostic([46, 70], "ignored", "did not pay attention to"),
            diagnostic([72, 103], "distrusted", "did not have much confidence in"),
            diagnostic([72, 103], "distrusted", "did not have much confidence in"),
            diagnostic([105, 131], "whether", "the question as to whether"),
            diagnostic([133, 159], "no doubt", "there is no doubt but that"),
            diagnostic([161, 183], "used for fuel", "used for fuel purposes"),
            diagnostic([185, 200], "he", "he is a man who"),
            diagnostic([202, 219], "hastily", "in a hasty manner"),
            diagnostic([221, 243], "this subject", "this is a subject that"),
            diagnostic([245, 258], "strange.", "a strange one"),
            diagnostic([260, 282], "because", "the reason why is that"),
            diagnostic([284, 306], "because / since", "owing to the fact that"),
            diagnostic([308, 333], "although / though", "in spite of the fact that"),
            diagnostic(
                [335, 371],
                "remind you / notify you",
                "call your attention to the fact that",
            ),
            diagnostic(
                [373, 396],
                "did not know / was unaware",
                "was unaware of the fact",
            ),
            diagnostic([398, 413], "failed", "did not succeed"),
            diagnostic([402, 413], "fail", "not succeed"),
            diagnostic([415, 426], "fail", "not succeed"),
            diagnostic([428, 455], "my arrival", "the fact that i had arrived"),
        ],
    );
}

#[test]
fn reports_regex_capture_variants_and_original_match_text() {
    assert_result(
        "She did not pay any attention to it. They did not have faith in us. We had not succeeded.",
        1000,
        1,
        vec![
            diagnostic([5, 33], "ignored", "did not pay any attention to"),
            diagnostic([43, 64], "distrusted", "did not have faith in"),
            diagnostic([72, 89], "failed", "had not succeeded"),
        ],
    );
}

#[test]
fn retains_duplicate_simple_and_regex_findings_at_the_same_span() {
    assert_result(
        "He did not pay attention to her.",
        1000,
        1,
        vec![
            diagnostic([4, 28], "ignored", "did not pay attention to"),
            diagnostic([4, 28], "ignored", "did not pay attention to"),
        ],
    );
}

#[test]
fn preserves_nested_simple_matches_without_error_limit() {
    assert_result(
        "did not succeed",
        1000,
        1,
        vec![
            diagnostic([1, 16], "failed", "did not succeed"),
            diagnostic([5, 16], "fail", "not succeed"),
        ],
    );
}

#[test]
fn max_errors_is_applied_before_position_sorting() {
    assert_result(
        "did not succeed",
        1,
        1,
        vec![diagnostic([5, 16], "fail", "not succeed")],
    );
}

#[test]
fn matches_case_insensitively_and_preserves_surface_text() {
    assert_result(
        "NOT HONEST. They DID NOT PAY ANY ATTENTION TO it.",
        1000,
        1,
        vec![
            diagnostic([1, 11], "dishonest", "NOT HONEST"),
            diagnostic([18, 46], "ignored", "DID NOT PAY ANY ATTENTION TO"),
        ],
    );
}

#[test]
fn correct_forms_and_ascii_word_boundaries_are_clean() {
    assert_result(
        "dishonest dishonestness not honesty; xnot honest not honestish. He is honest.",
        1000,
        0,
        vec![],
    );
}

#[test]
fn suppresses_quoted_simple_and_regex_findings_only() {
    assert_result(
        "\"not honest\" and not honest; \"did not pay any attention to\" then did not pay attention to.",
        1000,
        1,
        vec![
            diagnostic([18, 28], "dishonest", "not honest"),
            diagnostic([66, 90], "ignored", "did not pay attention to"),
            diagnostic([66, 90], "ignored", "did not pay attention to"),
        ],
    );
}

#[test]
fn source_orders_findings_and_keeps_tied_duplicates() {
    assert_result(
        "not honest; did not pay attention to her; not important.",
        1000,
        1,
        vec![
            diagnostic([1, 11], "dishonest", "not honest"),
            diagnostic([13, 37], "ignored", "did not pay attention to"),
            diagnostic([13, 37], "ignored", "did not pay attention to"),
            diagnostic([43, 56], "unimportant", "not important"),
        ],
    );
}

#[test]
fn uses_python_codepoint_coordinates_after_unicode_prefix() {
    assert_result(
        "😀 café — not honest; did not pay any attention to me.",
        1000,
        1,
        vec![
            diagnostic([10, 20], "dishonest", "not honest"),
            diagnostic([22, 50], "ignored", "did not pay any attention to"),
        ],
    );
}

#[test]
fn selected_config_runs_composition_with_other_default_rules_disabled() {
    assert_result(
        "Suddenly, not honest. The question as to whether the plan works.",
        1000,
        1,
        vec![
            diagnostic([11, 21], "dishonest", "not honest"),
            diagnostic([23, 49], "whether", "The question as to whether"),
        ],
    );
}

#[test]
fn fastset_order_for_reverse_phrase_order_is_visible_at_error_limit() {
    let input = "not important; not honest";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([1, 14], "unimportant", "not important")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([1, 14], "unimportant", "not important"),
            diagnostic([16, 26], "dishonest", "not honest"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([1, 14], "unimportant", "not important"),
            diagnostic([16, 26], "dishonest", "not honest"),
        ],
    );
}

#[test]
fn fastset_groups_repeated_occurrences_by_returned_pattern_index() {
    let input = "not honest; not important; not honest";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([1, 11], "dishonest", "not honest")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([1, 11], "dishonest", "not honest"),
            diagnostic([28, 38], "dishonest", "not honest"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([1, 11], "dishonest", "not honest"),
            diagnostic([13, 26], "unimportant", "not important"),
            diagnostic([28, 38], "dishonest", "not honest"),
        ],
    );
}

#[test]
fn fastset_nested_suffix_pattern_precedes_later_full_match() {
    let input = "not succeed; did not succeed";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([1, 12], "fail", "not succeed")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([1, 12], "fail", "not succeed"),
            diagnostic([18, 29], "fail", "not succeed"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([1, 12], "fail", "not succeed"),
            diagnostic([14, 29], "failed", "did not succeed"),
            diagnostic([18, 29], "fail", "not succeed"),
        ],
    );
}

#[test]
fn fastset_nested_suffix_pattern_precedes_earlier_full_match() {
    let input = "did not succeed; not succeed";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([5, 16], "fail", "not succeed")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([5, 16], "fail", "not succeed"),
            diagnostic([18, 29], "fail", "not succeed"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([1, 16], "failed", "did not succeed"),
            diagnostic([5, 16], "fail", "not succeed"),
            diagnostic([18, 29], "fail", "not succeed"),
        ],
    );
}

#[test]
fn quoted_occurrence_activates_pattern_but_is_filtered_before_limit() {
    let input = "\"not honest\"; not important; not honest";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([30, 40], "dishonest", "not honest")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([15, 28], "unimportant", "not important"),
            diagnostic([30, 40], "dishonest", "not honest"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([15, 28], "unimportant", "not important"),
            diagnostic([30, 40], "dishonest", "not honest"),
        ],
    );
}

#[test]
fn valid_activation_emits_raw_occurrence_inside_a_longer_word() {
    let input = "xnot honest; not important; not honest";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([14, 27], "unimportant", "not important")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([2, 12], "dishonest", "not honest"),
            diagnostic([14, 27], "unimportant", "not important"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([2, 12], "dishonest", "not honest"),
            diagnostic([14, 27], "unimportant", "not important"),
            diagnostic([29, 39], "dishonest", "not honest"),
        ],
    );
}

#[test]
fn simple_registration_precedes_earlier_regex_duplicate_until_unlimited() {
    let input = "did not pay attention to her; not important";
    assert_result(
        input,
        1,
        1,
        vec![diagnostic([1, 25], "ignored", "did not pay attention to")],
    );
    assert_result(
        input,
        2,
        1,
        vec![
            diagnostic([1, 25], "ignored", "did not pay attention to"),
            diagnostic([31, 44], "unimportant", "not important"),
        ],
    );
    assert_result(
        input,
        1000,
        1,
        vec![
            diagnostic([1, 25], "ignored", "did not pay attention to"),
            diagnostic([1, 25], "ignored", "did not pay attention to"),
            diagnostic([31, 44], "unimportant", "not important"),
        ],
    );
}

#[test]
fn ascii_word_boundaries_follow_re2_rules_next_to_non_ascii_letters() {
    assert_result(
        "énot honest",
        1000,
        1,
        vec![diagnostic([2, 12], "dishonest", "not honest")],
    );
    assert_result(
        "not honesté",
        1000,
        1,
        vec![diagnostic([1, 11], "dishonest", "not honest")],
    );
}

#[test]
fn combining_mark_is_a_re2_nonword_boundary() {
    assert_result(
        "not honest\u{0301}",
        1000,
        1,
        vec![diagnostic([1, 11], "dishonest", "not honest")],
    );
}

#[test]
fn re2_long_s_casefold_match_can_have_no_preferred_form_key() {
    assert_result(
        "not honeſt",
        1000,
        1,
        vec![diagnostic_with_none_replacement([1, 11], "not honeſt")],
    );
}

#[test]
fn regex_long_s_match_uses_its_registered_replacement() {
    assert_result(
        "not ſucceeded",
        1000,
        1,
        vec![diagnostic([1, 14], "failed", "not ſucceeded")],
    );
}

#[test]
fn regex_ascii_word_boundaries_match_next_to_unicode_letters() {
    assert_result(
        "énot succeeded; not succeededé",
        1000,
        1,
        vec![
            diagnostic([2, 15], "failed", "not succeeded"),
            diagnostic([17, 30], "failed", "not succeeded"),
        ],
    );
}
