#[path = "../src/rules/exclamation.rs"]
mod exclamation;

use serde_json::{Value, json};

fn diagnostic(span: [usize; 2], column: usize, suffix: bool) -> Value {
    json!({
        "check_path": "typography.punctuation.exclamation",
        "message": if suffix { "More than 30 ppm of exclamations. Keep them under control. Surpassed 30 ppm." } else { "More than 30 ppm of exclamations. Keep them under control." },
        "span": span,
        "pos": [1, column],
        "replacements": null,
    })
}

#[test]
fn exclamation_two_match_threshold() {
    assert_eq!(
        exclamation::diagnostics("a! a!"),
        vec![diagnostic([4, 6], 4, true)]
    );
}

#[test]
fn exclamation_padded_length_threshold_lower_edge() {
    let input = format!("a! a! a!{}", " ".repeat(66_656));
    assert_eq!(input.chars().count(), 66_664);
    assert_eq!(
        exclamation::diagnostics(&input),
        vec![diagnostic([4, 6], 4, true)]
    );
}

#[test]
fn exclamation_padded_length_threshold_upper_edge() {
    let input = format!("a! a! a!{}", " ".repeat(66_657));
    assert_eq!(input.chars().count(), 66_665);
    assert_eq!(
        exclamation::diagnostics(&input),
        vec![diagnostic([7, 9], 7, true)]
    );
}

#[test]
fn exclamation_quote_selected_then_suppressed() {
    assert_eq!(
        exclamation::diagnostics("wow! \"wow! \" wow!"),
        Vec::<Value>::new()
    );
}
