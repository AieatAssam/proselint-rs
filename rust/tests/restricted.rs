use proselint_rs::{restricted_elementary_diagnostics, restricted_top1000_diagnostics};
use serde_json::json;

const INPUT: &str = "known known; don't can't it's; child's well-known; xylophone xylophone";

#[test]
fn elementary_flags_only_disallowed_tokens_in_text_order() {
    let actual = restricted_elementary_diagnostics(INPUT);
    assert_eq!(
        actual,
        vec![
            json!({
                "check_path": "restricted.elementary",
                "message": "'child's' is not a word kids learn in elementary school.",
                "pos": [1, 33],
                "replacements": null,
                "span": [33, 39],
            }),
            json!({
                "check_path": "restricted.elementary",
                "message": "'well-known' is not a word kids learn in elementary school.",
                "pos": [1, 41],
                "replacements": null,
                "span": [41, 50],
            }),
            json!({
                "check_path": "restricted.elementary",
                "message": "'xylophone' is not a word kids learn in elementary school.",
                "pos": [1, 53],
                "replacements": null,
                "span": [53, 61],
            }),
            json!({
                "check_path": "restricted.elementary",
                "message": "'xylophone' is not a word kids learn in elementary school.",
                "pos": [1, 63],
                "replacements": null,
                "span": [63, 71],
            }),
        ]
    );
}

#[test]
fn top1000_flags_only_disallowed_tokens_in_text_order() {
    let actual = restricted_top1000_diagnostics(INPUT);
    assert_eq!(
        actual,
        vec![
            json!({
                "check_path": "restricted.top1000",
                "message": "'child's' is not in the top 1000 most common words.",
                "pos": [1, 33],
                "replacements": null,
                "span": [33, 39],
            }),
            json!({
                "check_path": "restricted.top1000",
                "message": "'well-known' is not in the top 1000 most common words.",
                "pos": [1, 41],
                "replacements": null,
                "span": [41, 50],
            }),
            json!({
                "check_path": "restricted.top1000",
                "message": "'xylophone' is not in the top 1000 most common words.",
                "pos": [1, 53],
                "replacements": null,
                "span": [53, 61],
            }),
            json!({
                "check_path": "restricted.top1000",
                "message": "'xylophone' is not in the top 1000 most common words.",
                "pos": [1, 63],
                "replacements": null,
                "span": [63, 71],
            }),
        ]
    );
}

#[test]
fn suppresses_paired_quotes_and_keeps_codepoint_spans() {
    let input = "“xylophone” é xylophone";
    for (actual, check_path, message) in [
        (
            restricted_elementary_diagnostics(input),
            "restricted.elementary",
            "'xylophone' is not a word kids learn in elementary school.",
        ),
        (
            restricted_top1000_diagnostics(input),
            "restricted.top1000",
            "'xylophone' is not in the top 1000 most common words.",
        ),
    ] {
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0]["check_path"], check_path);
        assert_eq!(actual[0]["message"], message);
        assert_eq!(actual[0]["pos"], json!([1, 16]));
        assert_eq!(actual[0]["span"], json!([16, 24]));
        assert_eq!(actual[0]["replacements"], json!(null));
    }
}
