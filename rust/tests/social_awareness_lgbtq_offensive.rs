use proselint_rs::social_awareness_lgbtq_offensive_diagnostics as diagnostics;
use serde_json::{Value, json};

#[test]
fn reports_all_terms_in_source_pattern_order() {
    let text = "fag faggot dyke sodomite homosexual agenda gay agenda transvestite homosexual lifestyle gay lifestyle";
    let got = diagnostics(text);
    assert_eq!(got.len(), 10);
    assert_eq!(
        got.iter().map(|d| d["span"].clone()).collect::<Vec<_>>(),
        vec![
            json!([1, 4]),
            json!([5, 8]),
            json!([5, 11]),
            json!([12, 16]),
            json!([17, 25]),
            json!([26, 43]),
            json!([44, 54]),
            json!([55, 67]),
            json!([68, 88]),
            json!([89, 102])
        ]
    );
    assert!(got.iter().all(|d| d["message"]
        == "Offensive term. Remove it or consider the context."
        && d["replacements"] == Value::Null));
}

#[test]
fn uses_ascii_word_boundaries_case_insensitively_and_suppresses_quotes() {
    assert_eq!(diagnostics("xdyke dykeish HOMOSEXUAL AGENDA").len(), 1);
    assert_eq!(diagnostics("\"dyke\" and dyke").len(), 1);
}

#[test]
fn positions_count_unicode_codepoints_and_sort_by_offset() {
    let got = diagnostics("é 😊 dyke; FAG");
    assert_eq!(got[0]["span"], json!([5, 9]));
    assert_eq!(got[0]["pos"], json!([1, 5]));
    assert_eq!(got[1]["span"], json!([11, 14]));
}
