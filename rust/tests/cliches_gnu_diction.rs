use proselint_rs::cliches_gnu_diction_diagnostics as diagnostics;

#[test]
fn matches_all_pinned_entries_in_source_order() {
    let phrases = [
        "a matter of concern",
        "all things being equal",
        "as a last resort",
        "attached hereto",
        "by no means",
        "conspicuous by its absence",
        "easier said than done",
        "enclosed herewith",
        "if and when",
        "in reference to",
        "in short supply",
        "in the foreseeable future",
        "in the long run",
        "in the matter of",
        "it stands to reason",
        "many and diverse",
        "on the right track",
        "par for the course",
        "please feel free to",
        "pursuant to your request",
        "regarding the matter of",
        "slowly but surely",
        "this will acknowledge",
        "we are pleased to advice",
        "we regret to inform you",
        "we wish to state",
        "you are hereby advised that",
    ];
    let actual = diagnostics(&phrases.join("; "));
    let messages = actual
        .iter()
        .map(|item| item["message"].as_str().unwrap())
        .collect::<Vec<_>>();
    let expected = phrases
        .iter()
        .map(|phrase| format!("'{phrase}' is a cliché"))
        .collect::<Vec<_>>();
    assert_eq!(messages, expected);
}

#[test]
fn matches_case_insensitively_and_reports_original_surface() {
    let actual = diagnostics("A MATTER OF CONCERN!");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["message"], "'A MATTER OF CONCERN' is a cliché");
    assert_eq!(actual[0]["span"], serde_json::json!([1, 20]));
}

#[test]
fn uses_ascii_word_boundaries_but_raw_matches_after_activation() {
    assert!(diagnostics("xin short supply; in short supplyx").is_empty());
    let actual = diagnostics("xin short supply; in short supply");
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0]["message"], "'in short supply' is a cliché");
    assert_eq!(actual[1]["message"], "'in short supply' is a cliché");
}

#[test]
fn suppresses_quoted_matches_and_keeps_source_order() {
    let actual = diagnostics("'in the long run', but in the matter of and in the long run");
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0]["message"], "'in the long run' is a cliché");
    assert_eq!(actual[1]["message"], "'in the matter of' is a cliché");
}

#[test]
fn orders_active_patterns_by_first_boundary_valid_match() {
    let actual = diagnostics("in the matter of; in the long run");
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0]["message"], "'in the matter of' is a cliché");
    assert_eq!(actual[1]["message"], "'in the long run' is a cliché");
}

#[test]
fn reports_nothing_without_a_boundary_valid_activation() {
    assert!(diagnostics("plain prose has no fixed phrase").is_empty());
}
