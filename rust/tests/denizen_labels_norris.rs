use proselint_rs::denizen_labels_norris_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_case_insensitive_terms_with_source_replacements() {
    let actual = diagnostics("Manchesterian and MANCHESTERIANS; TROIS-RIVIÈRESTER");
    assert_eq!(
        actual
            .iter()
            .map(|item| item["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("Mancunian"),
            json!("Mancunian"),
            json!("Mancunians"),
            json!("Trifluvian"),
        ]
    );
    assert_eq!(actual[0]["message"], "Would you like 'Mancunian'?");
}

#[test]
fn reports_raw_occurrences_after_a_valid_boundary_activation() {
    let actual = diagnostics("xmanchesterian manchesterianX Manchesterian");
    assert_eq!(actual.len(), 3);
    assert_eq!(
        actual
            .iter()
            .map(|item| item["span"].clone())
            .collect::<Vec<_>>(),
        vec![json!([2, 15]), json!([16, 29]), json!([31, 44])]
    );
}

#[test]
fn suppresses_quotes_and_preserves_unicode_character_coordinates() {
    let actual = diagnostics("Café ‘manchesterian’; Manchesterian");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["span"], json!([23, 36]));
    assert_eq!(actual[0]["pos"], json!([1, 23]));
}

#[test]
fn leaves_near_matches_unreported() {
    assert!(diagnostics("manchestarian is not a denizen label").is_empty());
}
