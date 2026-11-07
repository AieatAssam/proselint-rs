use regex::{Match, Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// The pinned check is attributed to Garner's Modern American Usage.
const ONE_SPACE: &str = r"[\.\?!] \w";
const TWO_SPACES: &str = r"[\.\?!]  \w";

static ONE_SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"[\.\?!] (?-u:\w)")
        .build()
        .expect("valid one-space spacing pattern")
});

static TWO_SPACES_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"[\.\?!]  (?-u:\w)")
        .build()
        .expect("valid two-space spacing pattern")
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let one_space_matches = ONE_SPACE_REGEX.find_iter(&padded).collect::<Vec<_>>();
    let two_space_matches = TWO_SPACES_REGEX.find_iter(&padded).collect::<Vec<_>>();

    // Upstream zips the match streams and treats ties as one-space minority.
    let (minority, replacement): (&[Match<'_>], &str) =
        if two_space_matches.len() < one_space_matches.len() {
            (&two_space_matches, ONE_SPACE)
        } else {
            (&one_space_matches, TWO_SPACES)
        };

    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    minority
        .iter()
        .filter_map(|matched| {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                return None;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            Some(json!({
                "check_path": "typography.punctuation.spacing",
                "message": "Inconsistent spacing after period (1 vs. 2 spaces).",
                "span": [start, end],
                "pos": [line, column],
                "replacements": replacement,
            }))
        })
        .collect()
}
