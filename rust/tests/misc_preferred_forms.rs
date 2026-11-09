use proselint_rs::misc_preferred_forms_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_simple_mapping_and_exact_message() {
    let got = diagnostics("imprimature");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["check_path"], "misc.preferred_forms");
    assert_eq!(got[0]["message"], "'imprimatur' is the preferred form.");
    assert_eq!(got[0]["span"], json!([1, 12]));
    assert_eq!(got[0]["replacements"], "imprimatur");
}

#[test]
fn uses_strict_ascii_boundaries_and_suppresses_quotes() {
    assert!(
        diagnostics(
            "ximprimature imprimaturex 'imprimature imprimature' -imprimature imprimature-"
        )
        .is_empty()
    );
    assert!(diagnostics("She wrote \"imprimature\" in the margin.").is_empty());
    let got = diagnostics("éimprimature imprimatureé —imprimature—");
    assert_eq!(got.len(), 3, "{got:#?}");
    assert_eq!(got[0]["span"], json!([2, 13]));
    assert_eq!(got[1]["span"], json!([14, 25]));
    assert_eq!(got[2]["span"], json!([28, 39]));
}

#[test]
fn keeps_case_sensitive_raw_meantime_patterns_and_clause_offset() {
    let capital =
        diagnostics("Meantime, next. meantime, lower. xMeantime, no boundary. Meantime,attached.");
    assert_eq!(capital.len(), 1);
    assert_eq!(capital[0]["message"], "'Meanwhile,' is the preferred form.");
    assert_eq!(capital[0]["span"], json!([1, 10]));

    let clause = diagnostics(
        "Alpha; meantime, next. Beta, meantime, later. Gamma;Meantime, upper. Delta; meantime,attached. ; meantime, start.",
    );
    assert_eq!(clause.len(), 3);
    assert_eq!(clause[0]["message"], "'meanwhile,' is the preferred form.");
    assert_eq!(clause[0]["span"], json!([8, 17]));
    assert_eq!(clause[1]["span"], json!([30, 39]));
    assert_eq!(clause[2]["message"], "'Meanwhile,' is the preferred form.");
    assert_eq!(clause[2]["span"], json!([53, 62]));
}
