use proselint_rs::redundancy_misc_after_the_deadline_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn preserves_raw_prefix_occurrences_across_batch_items() {
    let got = diagnostics("free gift; free gifts");
    assert_eq!(got.len(), 3);
    assert_eq!(
        got[0]["message"],
        "Redundancy. Use 'gift' instead of 'free gift'."
    );
    assert_eq!(
        got[1]["message"],
        "Redundancy. Use 'gift' instead of 'free gift'."
    );
    assert_eq!(
        got[2]["message"],
        "Redundancy. Use 'gifts' instead of 'free gifts'."
    );
    assert_eq!(got[1]["span"], json!([12, 21]));
    assert_eq!(got[2]["span"], json!([12, 22]));
}

#[test]
fn keeps_both_overlapping_consensus_findings_for_garner_default() {
    let got = diagnostics("general consensus of opinion");
    assert_eq!(got.len(), 2);
    assert_eq!(
        got[0]["message"],
        "Redundancy. Use 'consensus' instead of 'general consensus'."
    );
    assert_eq!(got[0]["span"], json!([1, 18]));
    assert_eq!(
        got[1]["message"],
        "Redundancy. Use 'consensus' instead of 'consensus of opinion'."
    );
    assert_eq!(got[1]["span"], json!([9, 29]));
}

#[test]
fn suppresses_quoted_match_and_counts_unicode_codepoints() {
    let got = diagnostics("é “bo staff”; bo staff");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["span"], json!([15, 23]));
    assert_eq!(got[0]["pos"], json!([1, 15]));
}
