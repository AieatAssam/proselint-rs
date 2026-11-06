#[path = "../src/rules/spelling_athletes.rs"]
mod spelling_athletes;

use serde_json::{Value, json};

#[test]
fn maps_known_names_and_keeps_the_raw_escaped_entry_null() {
    let actual =
        spelling_athletes::diagnostics("dwayne wade; brett farve; mike kryzewski; j.j. reddick");
    assert_eq!(actual.len(), 4);
    assert_eq!(actual[0]["replacements"], "Dwyane Wade");
    assert_eq!(actual[1]["replacements"], "Brett Favre");
    assert_eq!(actual[2]["replacements"], "Mike Krzyzewski");
    assert_eq!(actual[3]["replacements"], Value::Null);
    assert_eq!(
        actual[3]["message"],
        "Misspelling of athlete's name. 'None' is the preferred form."
    );
    assert_eq!(actual[3]["span"], json!([43, 55]));
}

#[test]
fn clean_forms_and_regex_wildcard_near_misses_do_not_match() {
    assert!(
        spelling_athletes::diagnostics("Dwyane Wade Mark Buehrle J.J. Redick jXjX reddick")
            .is_empty()
    );
    assert_eq!(
        spelling_athletes::diagnostics("DWAYNE WADE; dwayne wade").len(),
        2
    );
}

#[test]
fn word_activation_uses_ascii_boundaries_and_raw_emission() {
    assert!(spelling_athletes::diagnostics("xdwayne wade dwayne wadex").is_empty());
    let actual = spelling_athletes::diagnostics("(dwayne wade), [stephon curry]! édwayne wade");
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["span"], json!([2, 13]));
    assert_eq!(actual[1]["span"], json!([34, 45]));
    assert_eq!(actual[2]["span"], json!([17, 30]));
    assert_eq!(actual[2]["pos"], json!([1, 17]));
}

#[test]
fn suppresses_paired_quotes_and_orders_by_earliest_activation() {
    assert!(spelling_athletes::diagnostics("\"dwayne wade\" and 'stephon curry'").is_empty());
    let actual = spelling_athletes::diagnostics("stephon curry; dwayne wade; mikka kiprusoff");
    assert_eq!(
        actual
            .iter()
            .map(|d| d["replacements"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Stephen Curry", "Dwyane Wade", "Miikka Kiprusoff"]
    );
}

#[test]
fn preserves_all_seventeen_pinned_mapping_pairs() {
    assert_eq!(
        spelling_athletes::form_text_for_test()
            .split_whitespace()
            .count(),
        34
    );
}
