use proselint_rs::corporate_speak_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_case_insensitive_phrases_with_exact_surface_text() {
    let actual = diagnostics("AT THE END OF THE DAY; low-hanging fruit");
    assert_eq!(actual.len(), 2);
    assert_eq!(
        actual[0]["message"],
        "Minimize your use of corporate catchphrases like 'AT THE END OF THE DAY'."
    );
    assert_eq!(
        actual[1]["message"],
        "Minimize your use of corporate catchphrases like 'low-hanging fruit'."
    );
    assert_eq!(actual[0]["replacements"], json!(null));
}

#[test]
fn reports_raw_substrings_after_boundary_valid_activation() {
    let actual = diagnostics("xsynergy synergyX synergy");
    assert_eq!(actual.len(), 3);
    assert_eq!(
        actual
            .iter()
            .map(|item| item["span"].clone())
            .collect::<Vec<_>>(),
        vec![json!([2, 9]), json!([10, 17]), json!([19, 26])]
    );
}

#[test]
fn suppresses_quotes_and_uses_unicode_coordinates() {
    let actual = diagnostics("é “low-hanging fruit” but synergy");
    assert_eq!(actual.len(), 1);
    assert_eq!(
        actual[0]["message"],
        "Minimize your use of corporate catchphrases like 'synergy'."
    );
}

#[test]
fn leaves_nonmatching_text_unreported() {
    assert!(diagnostics("ordinary team communication").is_empty());
}
