use proselint_rs::nonwords_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn preserves_literal_alternative_replacement_strings() {
    let actual = diagnostics(
        "discomforture; dispersement; doubtlessly; relative inexpense; squelch; undoubtably; unrelentlessly",
    );
    assert_eq!(
        actual
            .iter()
            .map(|item| item["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("discomfort' or 'discomfiture"),
            json!("disbursement' or 'dispersal"),
            json!("doubtless' or 'undoubtedly"),
            json!("relatively low price' or 'affordability"),
            json!("quell' or 'quash"),
            json!("undoubtedly' or 'indubitably"),
            json!("unrelentingly' or relentlessly"),
        ]
    );
    assert_eq!(
        actual[0]["message"],
        "Nonword - try 'discomfort' or 'discomfiture'."
    );
}

#[test]
fn irregardlessness_requires_activation_then_reports_raw_substrings() {
    assert!(diagnostics("irregardlessness").is_empty());
    let actual = diagnostics("irregardless, irregardlessness");
    assert_eq!(actual.len(), 2);
    assert_eq!(
        actual
            .iter()
            .map(|item| item["span"].clone())
            .collect::<Vec<_>>(),
        vec![json!([1, 13]), json!([15, 27])]
    );
}

#[test]
fn suppresses_paired_quotes_and_matches_case_insensitively() {
    let actual = diagnostics("‘AFFRONTERY’ then Analyzation");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["replacements"], "analysis");
}

#[test]
fn leaves_near_matches_unreported() {
    assert!(diagnostics("irregardlessness and other text").is_empty());
}

#[test]
fn preserves_pattern_group_order_for_max_errors_selection() {
    let actual = diagnostics("thusly; affrontery; thusly; analyzation.");
    assert_eq!(
        actual
            .iter()
            .map(|item| item["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("thus"),
            json!("thus"),
            json!("effrontery"),
            json!("analysis"),
        ]
    );
}
