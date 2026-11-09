use proselint_rs::phrasal_adjectives_examples_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn preserves_mapping_case_message_and_repeat_order() {
    let got = diagnostics("Across the board discounts; ACROSS THE BOARD DISCOUNTS");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0]["span"], json!([1, 27]));
    assert_eq!(
        got[0]["message"],
        "Hyphenate 'Across the board discounts', a phrasal adjective, as 'across-the-board discounts'."
    );
    assert_eq!(got[0]["replacements"], "across-the-board discounts");
    assert_eq!(
        got[1]["message"],
        "Hyphenate 'ACROSS THE BOARD DISCOUNTS', a phrasal adjective, as 'across-the-board discounts'."
    );
}

#[test]
fn preserves_the_us_regex_key_none_lookup_quirk() {
    let got = diagnostics("U.S. led campaign");
    assert_eq!(got.len(), 1);
    assert_eq!(
        got[0]["message"],
        "Hyphenate 'U.S. led campaign', a phrasal adjective, as 'None'."
    );
    assert_eq!(got[0]["replacements"], json!(null));
}

#[test]
fn applies_word_boundaries_quotes_and_codepoint_coordinates() {
    assert!(diagnostics("_across the board discounts_; xacross the board discountsy").is_empty());
    let got = diagnostics("é ‘across the board discounts’; across the board discounts");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["span"], json!([33, 59]));
    assert_eq!(got[0]["pos"], json!([1, 33]));
}
