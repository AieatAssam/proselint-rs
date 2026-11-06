#[path = "../src/rules/spelling_able_atable.rs"]
mod spelling_able_atable;

use serde_json::{Value, json};

#[test]
fn reports_pinned_forms_with_codepoint_coordinates() {
    assert_eq!(
        spelling_able_atable::diagnostics("é abbreviatable, navigatable"),
        vec![
            json!({"check_path":"spelling.able_atable","message":"-able vs. -atable. 'abbreviable' is the preferred spelling.","span":[3,16],"pos":[1,3],"replacements":"abbreviable"}),
            json!({"check_path":"spelling.able_atable","message":"-able vs. -atable. 'navigable' is the preferred spelling.","span":[18,29],"pos":[1,18],"replacements":"navigable"}),
        ] as Vec<Value>
    );
}

#[test]
fn activation_uses_word_boundaries_then_emits_raw_occurrences() {
    assert!(spelling_able_atable::diagnostics("xAbbreviatable abbreviatablex").is_empty());
    let actual = spelling_able_atable::diagnostics("xAbbreviatable abbreviatablex AbBrEvIaTaBlE");
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["span"], json!([2, 15]));
    assert_eq!(actual[1]["span"], json!([16, 29]));
    assert_eq!(actual[2]["span"], json!([31, 44]));
    assert!(actual.iter().all(|d| d["replacements"] == "abbreviable"));
}

#[test]
fn suppresses_only_paired_quotes_and_keeps_line_positions() {
    assert!(spelling_able_atable::diagnostics("“abbreviatable” and ‘navigatable’").is_empty());
    let actual = spelling_able_atable::diagnostics("é abbreviatable\nnavigatable");
    assert_eq!(actual[0]["span"], json!([3, 16]));
    assert_eq!(actual[0]["pos"], json!([1, 3]));
    assert_eq!(actual[1]["span"], json!([17, 28]));
    assert_eq!(actual[1]["pos"], json!([2, 1]));
}

#[test]
fn follows_fastset_activation_order_before_cli_truncation() {
    let actual = spelling_able_atable::diagnostics("navigatable abbreviatable operatable");
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["replacements"], "navigable");
    assert_eq!(actual[1]["replacements"], "abbreviable");
    assert_eq!(actual[2]["replacements"], "operable");

    let input = spelling_able_atable::diagnostics("abbreviatable navigatable abbreviatable");
    assert_eq!(
        input[..2]
            .iter()
            .map(|d| d["replacements"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["abbreviable", "abbreviable"]
    );
}

#[test]
fn retains_all_pinned_mapping_pairs() {
    assert_eq!(
        spelling_able_atable::form_text_for_test()
            .split_whitespace()
            .count(),
        68
    );
}

#[test]
fn clean_preferred_forms_do_not_match() {
    assert!(spelling_able_atable::diagnostics("abbreviable navigable operable").is_empty());
}
