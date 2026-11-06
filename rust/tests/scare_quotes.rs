#[path = "../src/rules/scare_quotes.rs"]
mod scare_quotes;

use serde_json::{Value, json};

fn diagnostic(span: [usize; 2], column: usize) -> Value {
    json!({
        "check_path": "misc.scare_quotes",
        "message": "Misuse of 'scare quotes'. Delete them.",
        "pos": [1, column],
        "replacements": null,
        "span": span,
    })
}

#[test]
fn scare_quotes_positive_and_boundary_controls() {
    let actual = scare_quotes::diagnostics(
        "the 'take-home message'; otherthe 'take-home message'; the 'take-home message'x",
    );
    assert_eq!(actual, vec![diagnostic([1, 24], 1)]);
}

#[test]
fn scare_quotes_unicode_word_boundaries() {
    let actual = scare_quotes::diagnostics(
        "éthe 'take-home message' | e\u{301}the 'take-home message' | 😀 the 'take-home message'",
    );
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[0]["span"], json!([2, 25]));
    assert_eq!(actual[1]["span"], json!([30, 53]));
    assert_eq!(actual[2]["span"], json!([58, 81]));
}

#[test]
fn scare_quotes_quote_suppression() {
    let actual = scare_quotes::diagnostics("“the 'take-home message'” and the 'take-home message'");
    assert_eq!(actual, vec![diagnostic([31, 54], 31)]);
}
