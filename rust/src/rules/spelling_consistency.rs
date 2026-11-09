use regex::{Match, Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Intelligent Editing Ltd., http://bit.ly/1x3hYj7.
const TERM_PAIRS: [(&str, &str); 14] = [
    ("advisor", "adviser"),
    ("analyse", "analyze"),
    ("centre", "center"),
    ("chocolatey", "chocolaty"),
    ("colour", "color"),
    ("emphasise", "emphasize"),
    ("finalise", "finalize"),
    ("focussed", "focused"),
    ("labour", "labor"),
    ("learnt", "learned"),
    ("organise", "organize"),
    ("organised", "organized"),
    ("organising", "organizing"),
    ("recognise", "recognize"),
];

static MATCHERS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    TERM_PAIRS
        .iter()
        .map(|(left, right)| {
            let left = RegexBuilder::new(&format!(r"(?-u:\b){left}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned consistency term");
            let right = RegexBuilder::new(&format!(r"(?-u:\b){right}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned consistency term");
            (left, right)
        })
        .collect()
});

/// Return diagnostics for the pinned `spelling.consistency` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut results = Vec::new();

    for (pair_index, (left, right)) in MATCHERS.iter().enumerate() {
        let left_matches: Vec<Match<'_>> = left.find_iter(&padded).collect();
        let right_matches: Vec<Match<'_>> = right.find_iter(&padded).collect();
        let (majority, minority_matches): (&str, &[Match<'_>]) =
            if left_matches.len() > right_matches.len() {
                (TERM_PAIRS[pair_index].0, &right_matches)
            } else {
                // The pinned zip-longest algorithm gives the right-hand spelling
                // the majority when the two spellings occur equally often.
                (TERM_PAIRS[pair_index].1, &left_matches)
            };

        for matched in minority_matches {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            results.push(json!({
                "check_path": "spelling.consistency",
                "message": format!(
                    "Inconsistent spelling of '{}' (vs. '{}').",
                    majority,
                    matched.as_str()
                ),
                "span": [start, end],
                "pos": [line, column],
                "replacements": majority,
            }));
        }
    }

    results
}
