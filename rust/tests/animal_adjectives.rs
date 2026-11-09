use proselint_rs::animal_adjectives_diagnostics as diagnostics;
use serde_json::{Value, json};

#[test]
fn reports_mapped_terms_case_insensitively_in_source_item_order() {
    let actual = diagnostics("wolf-like and HAWK-LIKE; wild animal-like");
    assert_eq!(
        actual,
        vec![
            json!({"check_path":"terms.animal_adjectives","message":"There's a word for this: 'lupine'.","span":[1,10],"pos":[1,1],"replacements":"lupine"}),
            json!({"check_path":"terms.animal_adjectives","message":"There's a word for this: 'accipitrine'.","span":[15,24],"pos":[1,15],"replacements":"accipitrine"}),
            json!({"check_path":"terms.animal_adjectives","message":"There's a word for this: 'ferine'.","span":[26,42],"pos":[1,26],"replacements":"ferine"}),
        ] as Vec<Value>
    );
}

#[test]
fn respects_word_boundaries_and_suppresses_paired_quotes() {
    assert!(diagnostics("prehawk-like hawk-likeX").is_empty());
    let actual = diagnostics("“wolf-like” then bird-like");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["replacements"], "avine");
}

#[test]
fn reports_raw_occurrences_after_a_boundary_valid_activation() {
    let actual =
        diagnostics("HAWK-LIKE, xhawk-like hawk-likeX; mongoose-like fox-like; goose-like ox-like");
    assert_eq!(
        actual
            .iter()
            .map(|item| item["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("accipitrine"),
            json!("accipitrine"),
            json!("accipitrine"),
            json!("viverrine"),
            json!("anserine"),
            json!("vulpine"),
            json!("taurine"),
            json!("anserine"),
            json!("taurine"),
        ]
    );
}

#[test]
fn uses_unicode_codepoint_coordinates_and_ignores_unmapped_text() {
    let actual = diagnostics("é fox-like, ordinary");
    assert_eq!(actual[0]["span"], json!([3, 11]));
    assert_eq!(actual[0]["pos"], json!([1, 3]));
    assert!(diagnostics("birdlikes are ordinary").is_empty());
}
