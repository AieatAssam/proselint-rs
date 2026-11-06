#[path = "../src/rules/hyperbole.rs"]
mod hyperbole;

use serde_json::{Value, json};

fn diagnostic(span: [usize; 2], line: usize, column: usize, text: &str) -> Value {
    json!({
        "check_path": "typography.punctuation.hyperbole",
        "message": format!("'{}' is hyperbolic.", text),
        "pos": [line, column],
        "replacements": null,
        "span": span,
    })
}

#[test]
fn hyperbole_raw_greedy_mixed_punctuation() {
    assert_eq!(
        hyperbole::diagnostics("Really!!!? and what??!"),
        vec![
            diagnostic([1, 11], 1, 1, "Really!!!?"),
            diagnostic([16, 23], 1, 16, "what??!"),
        ]
    );
}

#[test]
fn hyperbole_single_punctuation_negative() {
    assert!(hyperbole::diagnostics("What! Really?").is_empty());
}

#[test]
fn hyperbole_unicode_word_match() {
    assert_eq!(
        hyperbole::diagnostics("éclair!!!"),
        vec![diagnostic([2, 10], 1, 2, "clair!!!")]
    );
}
