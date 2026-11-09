use proselint_rs::terms_venery_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn suggests_pinned_collective_terms_case_insensitively() {
    let actual = diagnostics("GROUP OF ALLIGATORS; BUNCH OF WOMBATS");
    assert_eq!(
        actual
            .iter()
            .map(|item| item["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("congregation of alligators"),
            json!("wisdom of wombats"),
        ]
    );
    assert_eq!(
        actual[0]["message"],
        "The venery term is 'congregation of alligators'."
    );
}

#[test]
fn trigger_is_raw_but_phrase_matches_need_activation() {
    assert!(diagnostics("subgroup; groupwork of wombats; bunchline").is_empty());
    let actual = diagnostics("group of wombats; subgroup of wombats");
    assert_eq!(actual.len(), 2);
    assert_eq!(
        actual
            .iter()
            .map(|item| item["span"].clone())
            .collect::<Vec<_>>(),
        vec![json!([1, 17]), json!([22, 38])]
    );
}

#[test]
fn suppresses_quotes_and_ignores_unsupported_animals() {
    let actual =
        diagnostics("A “group of alligators” but a bunch of wombats and group of unicorns.");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["replacements"], "wisdom of wombats");
}

#[test]
fn avoids_work_without_a_generic_trigger() {
    assert!(diagnostics("alligators and wombats").is_empty());
}
