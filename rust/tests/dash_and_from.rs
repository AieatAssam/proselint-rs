#[path = "../src/rules/dash_and_from.rs"]
mod dash_and_from;

use serde_json::{Value, json};

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "dates_times.dates.dash_and_from",
        "message": "When specifying a date range, write 'from X to Y'.",
        "pos": [1, column],
        "replacements": null,
        "span": span,
    })
}

#[test]
fn reports_pinned_range_forms_and_case_insensitively() {
    assert_eq!(
        dash_and_from::diagnostics("From 1990,1991; FROM 2000—2001."),
        vec![diagnostic([1, 15], 1), diagnostic([17, 31], 17)]
    );
}

#[test]
fn rejects_dates_times_and_valid_prose_ranges() {
    assert!(dash_and_from::diagnostics(
        "from 1990-01-01; from 1990/01/02; from 1990:01; from 1990 to 1991; from 1990 1991; from 1990.1991"
    )
    .is_empty());
}

#[test]
fn keeps_raw_substring_matches_and_unicode_codepoint_coordinates() {
    assert_eq!(
        dash_and_from::diagnostics("éfrom 1990-1991; from 2000—2001é"),
        vec![diagnostic([2, 16], 2), diagnostic([18, 32], 18)]
    );
}

#[test]
fn uses_unicode_digits_but_excludes_unicode_word_separators() {
    assert_eq!(
        dash_and_from::diagnostics("from ٢٠٠٠-٢٠٠١"),
        vec![diagnostic([1, 15], 1)]
    );
    assert!(dash_and_from::diagnostics("from 1990é1991").is_empty());
}

#[test]
fn matches_pinned_unicode_marks_and_join_controls_as_separators() {
    for separator in ['\u{0301}', '\u{203f}', '\u{200c}'] {
        assert_eq!(
            dash_and_from::diagnostics(&format!("from 1990{separator}1991")),
            vec![diagnostic([1, 15], 1)],
            "separator U+{:04X}",
            separator as u32
        );
    }
}

#[test]
fn suppresses_matches_inside_default_paired_quotes() {
    assert!(dash_and_from::diagnostics("A \"from 1990-1991\" range.").is_empty());
    assert_eq!(
        dash_and_from::diagnostics("A \"quoted from 1990-1991\" and from 2000-2001."),
        vec![diagnostic([31, 45], 31)]
    );
}
