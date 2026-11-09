use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: GLAAD Media Reference Guide, 9th Edition (http://www.glaad.org/reference).
// Rule data retained under the project's BSD-3-Clause policy; this is not a legal guarantee.
const FORMS: [(&str, &str); 10] = [
    ("homosexual man", "gay man"),
    ("homosexual men", "gay men"),
    ("homosexual woman", "lesbian"),
    ("homosexual women", "lesbians"),
    ("homosexual people", "gay people"),
    ("homosexual couple", "gay couple"),
    ("sexual preference", "sexual orientation"),
    ("admitted homosexual", "openly gay"),
    ("avowed homosexual", "openly gay"),
    ("special rights", "equal rights"),
];
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            RegexBuilder::new(&regex::escape(term))
                .case_insensitive(true)
                .build()
                .unwrap()
        })
        .collect()
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut active = Vec::new();
    for (index, pattern) in PATTERNS.iter().enumerate() {
        let Some(activation) = pattern
            .find_iter(&padded)
            .find(|m| {
                !is_word(padded.as_bytes().get(m.start().wrapping_sub(1)).copied())
                    && !is_word(padded.as_bytes().get(m.end()).copied())
            })
            .map(|m| m.start())
        else {
            continue;
        };
        let mut found = Vec::new();
        for matched in pattern.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let original = matched.as_str();
            let replacement = FORMS[index].1;
            let (line, column) = crate::line_col_of(start, &line_bounds);
            found.push((
                start,
                json!({
                    "check_path": "social_awareness.lgbtq.terms",
                "message": format!("Possibly offensive term. Consider using '{}' instead of '{}'.", replacement, original),
                    "span": [start, end], "pos": [line, column], "replacements": replacement
                }),
            ));
        }
        active.push((activation, index, found));
    }
    active.sort_by_key(|(activation, index, _)| (*activation, *index));
    active
        .into_iter()
        .flat_map(|(_, _, found)| found)
        .map(|(_, value)| value)
        .collect()
}

fn is_word(byte: Option<u8>) -> bool {
    byte.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
}
