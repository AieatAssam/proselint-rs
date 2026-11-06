#[path = "../src/rules/misplaced.rs"]
mod misplaced;

use serde_json::json;

#[test]
fn ordered_fast_patterns_keep_the_shorter_first_match() {
    let actual = misplaced::diagnostics("et. al et. al. et. al..");
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![json!([1, 7]), json!([8, 14]), json!([16, 22])]
    );
    assert_eq!(
        actual.iter().map(|d| d["pos"].clone()).collect::<Vec<_>>(),
        vec![json!([1, 1]), json!([1, 8]), json!([1, 16])]
    );
    assert!(
        actual
            .iter()
            .all(|d| d["message"] == "Misplaced punctuation. It's 'et al.'"
                && d["replacements"].is_null())
    );
}

#[test]
fn ascii_word_boundaries_and_quote_suppression_match_fast_semantics() {
    let actual = misplaced::diagnostics("xet. al. et. alx et al. et. al. \"et. al.\"");
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![json!([25, 31])]
    );
    assert_eq!(misplaced::diagnostics("éet. al.")[0]["span"], json!([2, 8]));
}

#[test]
fn unicode_positions_use_codepoints_and_both_forms_report() {
    let actual = misplaced::diagnostics("😀 et. al. café et. al");
    assert_eq!(
        actual.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![json!([3, 9]), json!([16, 22])]
    );
    assert_eq!(actual[0]["pos"], json!([1, 3]));
}
