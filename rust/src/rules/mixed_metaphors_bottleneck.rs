// Derived from pinned Proselint 0.16.0 `checks/mixed_metaphors.py` (BSD-3-Clause).
// The source rule attributes the usage guidance to Sir Ernest Gowers.
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const CHECK_PATH: &str = "mixed_metaphors.bottleneck";
const MESSAGE: &str = "Mixed metaphor — bottles with big necks are easy to pass through.";
const PHRASES_IN_ORDER: [&str; 7] = [
    "biggest bottleneck",
    "big bottleneck",
    "large bottleneck",
    "largest bottleneck",
    "world-wide bottleneck",
    "huge bottleneck",
    "massive bottleneck",
];

static PHRASES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    PHRASES_IN_ORDER
        .iter()
        .map(|phrase| {
            RegexBuilder::new(&format!(r"\b{phrase}\b"))
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid bottleneck phrase pattern")
        })
        .collect()
});

/// Return diagnostics for pinned `mixed_metaphors.bottleneck`.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();
    for (index, phrase) in PHRASES.iter().enumerate() {
        let Some(first) = phrase.find(&padded) else {
            continue;
        };
        let diagnostics = phrase
            .find_iter(&padded)
            .filter_map(|matched| {
                let start = padded[..matched.start()].chars().count();
                if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                    return None;
                }
                let end = start + matched.as_str().chars().count();
                let (line, column) = crate::line_col_of(start, &line_bounds);
                Some(json!({
                    "check_path": CHECK_PATH,
                    "message": MESSAGE,
                    "pos": [line, column],
                    "replacements": Value::Null,
                    "span": [start, end],
                }))
            })
            .collect::<Vec<_>>();
        groups.push((first.start(), index, diagnostics));
    }
    groups.sort_unstable_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
