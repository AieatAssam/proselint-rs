use proselint_rs::cliches_garner_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn preserves_raw_match_text_and_the_pinned_regex_alternation() {
    let actual = diagnostics("A FATE WORSE THAN DEATH; WheT your appetite!");
    assert_eq!(actual.len(), 2);
    assert_eq!(
        actual[0]["message"],
        "'A FATE WORSE THAN DEATH' is a cliché"
    );
    assert_eq!(actual[1]["message"], "'WheT your appetite' is a cliché");
    assert_eq!(actual[0]["replacements"], json!(null));
}

#[test]
fn reports_raw_substrings_after_boundary_valid_activation() {
    let actual = diagnostics("xblow a fuse; blow a fuseX; blow a fuse");
    assert_eq!(actual.len(), 3);
    assert!(actual.iter().all(|item| {
        item["message"] == "'blow a fuse' is a cliché" && item["replacements"].is_null()
    }));
}

#[test]
fn suppresses_quotes_and_preserves_pattern_order_for_shared_phrases() {
    let actual = diagnostics(
        "é “quantum leap” but a quantum jump; the cream rises to the top; the cream of the crop",
    );
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["message"], "'quantum jump' is a cliché");
    assert_eq!(
        actual[1]["message"],
        "'the cream rises to the top' is a cliché"
    );
    assert_eq!(actual[2]["message"], "'the cream of the crop' is a cliché");
}

#[test]
fn leaves_nonmatching_text_unreported() {
    assert!(diagnostics("plain language without a fixed expression").is_empty());
}
