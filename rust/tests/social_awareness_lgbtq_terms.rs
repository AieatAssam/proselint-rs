use proselint_rs::social_awareness_lgbtq_terms_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn reports_all_replacements_and_source_casing() {
    let got = diagnostics(
        "homosexual man; homosexual men; homosexual woman; homosexual women; homosexual people; homosexual couple; sexual preference; admitted homosexual; avowed homosexual; special rights",
    );
    assert_eq!(got.len(), 10);
    assert_eq!(
        got.iter()
            .map(|d| d["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            "gay man",
            "gay men",
            "lesbian",
            "lesbians",
            "gay people",
            "gay couple",
            "sexual orientation",
            "openly gay",
            "openly gay",
            "equal rights"
        ]
        .into_iter()
        .map(|s| json!(s))
        .collect::<Vec<_>>()
    );
    assert_eq!(
        got[0]["message"],
        "Possibly offensive term. Consider using 'gay man' instead of 'homosexual man'."
    );
}

#[test]
fn case_boundaries_raw_occurrences_and_quotes_match_upstream() {
    let got = diagnostics("HOMOSEXUAL MAN, prehomosexual man, and \"sexual preference\"");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0]["span"], json!([1, 15]));
    assert_eq!(
        got[0]["message"],
        "Possibly offensive term. Consider using 'gay man' instead of 'HOMOSEXUAL MAN'."
    );
}

#[test]
fn counts_unicode_codepoints_and_activates_in_source_order() {
    let got = diagnostics("é sexual preference; homosexual man");
    assert_eq!(got[0]["span"], json!([3, 20]));
    assert_eq!(got[0]["pos"], json!([1, 3]));
    assert_eq!(got[1]["span"], json!([22, 36]));
}
