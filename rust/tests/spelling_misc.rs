use proselint_rs::spelling_misc_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_case_insensitive_simple_forms_and_raw_prefix_occurrences() {
    let got = diagnostics("notalot alot ALot");
    assert_eq!(got.len(), 3);
    assert_eq!(
        got[0]["message"],
        "Misspelling. 'a lot' is the preferred spelling."
    );
    assert_eq!(got[0]["span"], json!([4, 8]));
    assert_eq!(got[1]["span"], json!([9, 13]));
    assert_eq!(got[2]["span"], json!([14, 18]));
    assert_eq!(got[0]["replacements"], "a lot");
}

#[test]
fn preserves_raw_till_regex_matches_and_message() {
    let got = diagnostics("we stopped 'til; we waited 'till");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0]["span"], json!([12, 16]));
    assert_eq!(got[1]["span"], json!([28, 33]));
    assert_eq!(
        got[0]["message"],
        "Misspelling. 'till' is the preferred spelling."
    );
    assert_eq!(got[1]["replacements"], "till");
}

#[test]
fn suppresses_quoted_simple_forms_and_uses_codepoint_coordinates() {
    let got = diagnostics("é “alot”; alot");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["span"], json!([11, 15]));
    assert_eq!(got[0]["pos"], json!([1, 11]));
}
