use proselint_rs::social_awareness_sexism_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_both_source_mapping_sets_with_exact_templates() {
    let got = diagnostics(
        "anchorman; anchorwoman; chairman; chairwoman; draftman; draftwoman; ombudsman; ombudswoman; tribesman; tribeswoman; policeman; policewoman; fireman; firewoman; mailman; mailwoman; herstory; womyn; poetess; authoress; waitress; lady lawyer; woman doctor; female booksalesman; female airman; executrix; prosecutrix; testatrix; man and wife; chairmen and chairs; men and girls; comedienne; confidante; woman scientist; women scientists; anchorperson; chairperson; draftperson; ombudsperson; tribesperson; policeperson; fireperson; mailperson",
    );
    assert_eq!(got.len(), 43);
    assert_eq!(
        got[0]["message"],
        "Gender bias. Use 'anchor' instead of 'anchorman'."
    );
    assert_eq!(
        got[35]["message"],
        "Not a preferred form. Use 'anchor' instead of 'anchorperson'."
    );
    assert_eq!(got[0]["check_path"], "social_awareness.sexism");
}

#[test]
fn preserves_case_boundaries_quotes_coordinates_and_match_order() {
    let got = diagnostics("é WOMAN DOCTOR; preanchorwoman; \"chairperson\"; anchorwoman\n");
    assert_eq!(got.len(), 3);
    assert_eq!(got[0]["span"], json!([3, 15]));
    assert_eq!(got[0]["pos"], json!([1, 3]));
    assert_eq!(got[0]["replacements"], "doctor");
    assert_eq!(
        got[1]["message"],
        "Gender bias. Use 'anchor' instead of 'anchorwoman'."
    );
    assert_eq!(got[1]["span"], json!([20, 31]));
}
