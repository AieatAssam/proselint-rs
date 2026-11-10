use proselint_rs::needless_variants_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn maps_case_insensitively_with_exact_message_and_span() {
    let got = diagnostics("ABOLISHMENT");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["check_path"], "needless_variants");
    assert_eq!(
        got[0]["message"],
        "Needless variant. 'abolition' is the preferred form."
    );
    assert_eq!(got[0]["span"], json!([1, 12]));
    assert_eq!(got[0]["replacements"], "abolition");
}

#[test]
fn raw_matches_follow_activation_and_quote_suppression() {
    let got = diagnostics("abolishment xabolishment 'abolishment' éabolishmenté");
    assert_eq!(got.len(), 3, "{got:#?}");
    assert_eq!(got[0]["span"], json!([1, 12]));
    assert_eq!(got[1]["span"], json!([14, 25]));
    assert_eq!(got[2]["span"], json!([41, 52]));
}

#[test]
fn preserves_pattern_order_before_positional_sorting() {
    let got = diagnostics("ABOLISHMENT; accessary; abolishment");
    assert_eq!(got.len(), 3);
    assert_eq!(got[0]["span"], json!([1, 12]));
    assert_eq!(got[1]["span"], json!([25, 36]));
    assert_eq!(got[2]["span"], json!([14, 23]));
}
