use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const TERMS: [&str; 9] = [
    "fag",
    "faggot",
    "dyke",
    "sodomite",
    "homosexual agenda",
    "gay agenda",
    "transvestite",
    "homosexual lifestyle",
    "gay lifestyle",
];
const MESSAGE: &str = "Offensive term. Remove it or consider the context.";

// Source: GLAAD Media Reference Guide, 9th Edition (http://www.glaad.org/reference).
// Rule data retained under the project's BSD-3-Clause policy; this is not a legal guarantee.
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    TERMS
        .iter()
        .map(|term| {
            RegexBuilder::new(term)
                .case_insensitive(true)
                .build()
                .expect("valid offensive-term pattern")
        })
        .collect()
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut findings = Vec::new();
    for pattern in PATTERNS.iter() {
        if !pattern.find_iter(&padded).any(|matched| {
            !is_word_byte(
                padded
                    .as_bytes()
                    .get(matched.start().wrapping_sub(1))
                    .copied(),
            ) && !is_word_byte(padded.as_bytes().get(matched.end()).copied())
        }) {
            continue;
        }
        for matched in pattern.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            findings.push((
                start,
                json!({
                    "check_path": "social_awareness.lgbtq.offensive",
                    "message": MESSAGE,
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": Value::Null,
                }),
            ));
        }
    }
    findings.sort_by_key(|(start, _)| *start);
    findings.into_iter().map(|(_, finding)| finding).collect()
}

fn is_word_byte(byte: Option<u8>) -> bool {
    byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
