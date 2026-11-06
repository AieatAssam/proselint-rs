#[path = "../src/rules/spelling_able_ible.rs"]
mod spelling_able_ible;

use serde_json::{Value, json};

#[test]
fn detects_case_insensitive_forms_with_exact_coordinates_and_replacements() {
    assert_eq!(
        spelling_able_ible::diagnostics("é accessable and VISABLE"),
        vec![
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'accessible' is the preferred spelling.","span":[3,13],"pos":[1,3],"replacements":"accessible"}),
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'visible' is the preferred spelling.","span":[18,25],"pos":[1,18],"replacements":"visible"}),
        ] as Vec<Value>
    );
}

#[test]
fn retains_the_complete_pinned_mapping_table() {
    let input = spelling_able_ible::form_text_for_test();
    let actual = spelling_able_ible::diagnostics(&input);
    assert_eq!(actual.len(), 173);
    assert_eq!(actual[0]["replacements"], "actionable");
    assert_eq!(actual[1]["replacements"], "addable");
    assert!(actual.iter().any(|d| d["replacements"] == "accessible"));
    assert!(actual.iter().any(|d| d["replacements"] == "visible"));
}

#[test]
fn emits_raw_occurrences_after_boundary_activation() {
    let actual = spelling_able_ible::diagnostics("xaccessable accessablex AcCeSsAbLe accessable");
    assert_eq!(actual.len(), 4);
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![
            json!([2, 12]),
            json!([13, 23]),
            json!([25, 35]),
            json!([36, 46]),
        ]
    );
    assert!(actual.iter().all(|d| d["replacements"] == "accessible"));
}

#[test]
fn suppresses_paired_quotes_and_uses_codepoint_positions() {
    assert!(spelling_able_ible::diagnostics("“accessable” and ‘visable’").is_empty());
    let actual = spelling_able_ible::diagnostics("é accessable\nvisable");
    assert_eq!(actual[0]["span"], json!([3, 13]));
    assert_eq!(actual[0]["pos"], json!([1, 3]));
    assert_eq!(actual[1]["span"], json!([14, 21]));
    assert_eq!(actual[1]["pos"], json!([2, 1]));
}

#[test]
fn matches_re2_unicode_case_behavior_and_exact_lookup() {
    assert_eq!(
        spelling_able_ible::diagnostics("ſensable senſable visable vіsable"),
        vec![
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'None' is the preferred spelling.","span":[1,9],"pos":[1,1],"replacements":null}),
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'None' is the preferred spelling.","span":[10,18],"pos":[1,10],"replacements":null}),
            json!({"check_path":"spelling.able_ible","message":"-able vs. -ible. 'visible' is the preferred spelling.","span":[19,26],"pos":[1,19],"replacements":"visible"}),
        ] as Vec<Value>
    );
}

#[test]
fn matcher_order_uses_earliest_pattern_activation() {
    let actual = spelling_able_ible::diagnostics("visable actionible edable");
    assert_eq!(
        actual
            .iter()
            .map(|d| d["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![json!("visible"), json!("actionable"), json!("edible")]
    );
    assert_eq!(actual[0]["span"], json!([1, 8]));
}
