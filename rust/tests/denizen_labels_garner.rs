use proselint_rs::denizen_labels_garner_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_case_insensitive_labels_and_preserves_the_mixed_case_lookup_quirk() {
    let actual = diagnostics("Hong Kongite; MICHIGANITE; los Angelean");
    assert_eq!(
        actual
            .iter()
            .map(|item| item["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![json!("Hong Konger"), json!("Michigander"), json!(null)]
    );
    assert_eq!(
        actual[2]["message"],
        "'None' is the preferred denizen label."
    );
}

#[test]
fn reports_raw_occurrences_after_a_valid_boundary_activation() {
    let actual = diagnostics("xHong Kongite Hong KongiteX Hong Kongite");
    assert_eq!(actual.len(), 3);
    assert_eq!(
        actual
            .iter()
            .map(|item| item["span"].clone())
            .collect::<Vec<_>>(),
        vec![json!([2, 14]), json!([15, 27]), json!([29, 41])]
    );
}

#[test]
fn suppresses_paired_quotes_and_uses_unicode_coordinates() {
    let actual = diagnostics("Café “tusconite”; Hong Kongite");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["span"], json!([19, 31]));
    assert_eq!(actual[0]["pos"], json!([1, 19]));
}

#[test]
fn leaves_near_matches_unreported() {
    assert!(diagnostics("HongKongite, ordinary text").is_empty());
}
