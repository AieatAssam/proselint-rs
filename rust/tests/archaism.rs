#[path = "../src/rules/archaism.rs"]
mod archaism;

use serde_json::{Value, json};

#[test]
fn emits_exact_message_null_replacement_and_positions() {
    let actual = archaism::diagnostics("ALACK, and in sooth.");
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0]["check_path"], "archaism");
    assert_eq!(actual[0]["message"], "'ALACK' is archaic.");
    assert_eq!(actual[0]["replacements"], Value::Null);
    assert_eq!(actual[0]["span"], json!([1, 6]));
    assert_eq!(actual[0]["pos"], json!([1, 1]));
}

#[test]
fn activates_on_case_insensitive_ascii_word_boundaries() {
    assert_eq!(archaism::diagnostics("xalack alackx").len(), 0);
    assert_eq!(archaism::diagnostics("éalack alacké").len(), 2);
    assert_eq!(archaism::diagnostics("ALACK alack").len(), 2);
}

#[test]
fn fast_set_orders_groups_by_first_activation_then_emits_raw_matches() {
    let actual = archaism::diagnostics("methinks then alack; methinks");
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["message"], "'methinks' is archaic.");
    assert_eq!(actual[1]["message"], "'methinks' is archaic.");
    assert_eq!(actual[2]["message"], "'alack' is archaic.");
}

#[test]
fn suppresses_archaisms_inside_paired_quotes() {
    assert!(archaism::diagnostics("'alack' but alack").len() == 1);
    assert!(archaism::diagnostics("“methinks”").is_empty());
}

#[test]
fn reports_padded_content_offsets_and_columns_after_newlines() {
    let actual = archaism::diagnostics("plain\nalack");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["span"], json!([7, 12]));
    assert_eq!(actual[0]["pos"], json!([2, 1]));
}

#[test]
fn excludes_comment_only_forms_and_finds_all_active_forms() {
    assert!(
        archaism::diagnostics("anon designer demean by the bye comptroller abortive").is_empty()
    );
    let all = archaism::form_text_for_test();
    assert_eq!(all.split(" | ").count(), 47);
    assert_eq!(archaism::diagnostics(&all).len(), 47);
}
