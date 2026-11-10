use proselint_rs::uncomparables_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_standard_comparison_with_exact_surface_and_span() {
    let got = diagnostics("It is very unique.");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["check_path"], "uncomparables");
    assert_eq!(
        got[0]["message"],
        "Comparison of an uncomparable: 'very unique' is not comparable."
    );
    assert_eq!(got[0]["span"], json!([7, 18]));
}

#[test]
fn fancy_checks_require_word_boundaries_and_ignore_at_least() {
    let got = diagnostics("at least unique; least uniqueish; most unique!");
    assert_eq!(got.len(), 1, "{got:#?}");
    assert_eq!(
        got[0]["message"],
        "Comparison of an uncomparable: 'most unique' is not comparable."
    );
    assert_eq!(got[0]["span"], json!([35, 46]));
}

#[test]
fn preserves_fast_then_fancy_registration_order_and_suppresses_quotes() {
    let got = diagnostics("most unique; very unique; 'less false'");
    assert_eq!(got.len(), 2, "{got:#?}");
    assert_eq!(
        got[0]["message"],
        "Comparison of an uncomparable: 'very unique' is not comparable."
    );
    assert_eq!(
        got[1]["message"],
        "Comparison of an uncomparable: 'most unique' is not comparable."
    );
}

#[test]
fn permits_documented_more_exceptions_and_fast_unbounded_suffix() {
    let got = diagnostics("more perfect; more possible; very unique; very uniqueish");
    assert_eq!(got.len(), 2, "{got:#?}");
    assert_eq!(
        got[0]["message"],
        "Comparison of an uncomparable: 'very unique' is not comparable."
    );
}

#[test]
fn fast_limit_order_follows_earliest_pattern_activation() {
    let got = diagnostics("very unique; extremely absolute");
    assert_eq!(
        got[0]["message"],
        "Comparison of an uncomparable: 'very unique' is not comparable."
    );
}

#[test]
fn fancy_findings_follow_text_order() {
    let got = diagnostics("most unique; least absolute");
    let fancy = got
        .iter()
        .filter(|item| item["message"].as_str().unwrap().starts_with("Comparison"))
        .collect::<Vec<_>>();
    assert_eq!(
        fancy
            .iter()
            .map(|item| item["message"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "Comparison of an uncomparable: 'most unique' is not comparable.",
            "Comparison of an uncomparable: 'least absolute' is not comparable.",
        ]
    );
}
