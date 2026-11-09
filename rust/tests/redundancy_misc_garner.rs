use proselint_rs::redundancy_misc_garner_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_simple_forms_and_collapsed_duplicate_once() {
    let got = diagnostics("potable drinking water; fellow collaborator; fellow collaborators");
    assert_eq!(got.len(), 4);
    assert_eq!(
        got[0]["message"],
        "Redundancy. Use 'potable water' instead of 'potable drinking water'."
    );
    assert_eq!(
        got[1]["message"],
        "Redundancy. Use 'collaborator' instead of 'fellow collaborator'."
    );
    assert_eq!(
        got[2]["message"],
        "Redundancy. Use 'collaborator' instead of 'fellow collaborator'."
    );
    assert_eq!(
        got[3]["message"],
        "Redundancy. Use 'collaborators' instead of 'fellow collaborators'."
    );
    assert_eq!(got[0]["check_path"], "redundancy.misc.garner");
}

#[test]
fn preserves_both_optional_regex_patterns_and_case() {
    let got = diagnostics(
        "General consensus of opinion; consensus of opinion; Associate together; associate together in groups",
    );
    assert_eq!(got.len(), 4);
    assert_eq!(
        got[0]["message"],
        "Redundancy. Use 'consensus' instead of 'General consensus of opinion'."
    );
    assert_eq!(
        got[1]["message"],
        "Redundancy. Use 'consensus' instead of 'consensus of opinion'."
    );
    assert_eq!(
        got[2]["message"],
        "Redundancy. Use 'associate' instead of 'Associate together'."
    );
    assert_eq!(
        got[3]["message"],
        "Redundancy. Use 'associate' instead of 'associate together in groups'."
    );
}

#[test]
fn suppresses_quoted_matches_and_uses_padded_unicode_coordinates() {
    let got = diagnostics("é \"adequate enough\"; adequate enough");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["span"], json!([22, 37]));
    assert_eq!(got[0]["pos"], json!([1, 22]));
}
