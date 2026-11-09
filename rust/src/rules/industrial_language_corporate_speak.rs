use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Travis Bradberry for Inc.com, http://bit.ly/1IxWnto.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const PHRASES: [&str; 25] = [
    "at the end of the day",
    "back to the drawing board",
    "hit the ground running",
    "get the ball rolling",
    "low-hanging fruit",
    "thrown under the bus",
    "think outside the box",
    "let's touch base",
    "get my manager's blessing",
    "it's on my radar",
    "ping me",
    "i don't have the bandwidth",
    "no brainer",
    "par for the course",
    "bang for your buck",
    "synergy",
    "move the goal post",
    "apples to apples",
    "win-win",
    "circle back around",
    "all hands on deck",
    "take this offline",
    "drill-down",
    "elephant in the room",
    "on my plate",
];

static PATTERNS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    PHRASES
        .iter()
        .map(|phrase| {
            let escaped = regex::escape(phrase);
            let activation = RegexBuilder::new(&format!(r"(?-u:\b){escaped}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned corporate catchphrase");
            let raw = RegexBuilder::new(&escaped)
                .case_insensitive(true)
                .build()
                .expect("valid pinned corporate catchphrase");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `industrial_language.corporate_speak` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();
    for (index, (activation, raw)) in PATTERNS.iter().enumerate() {
        let Some(first) = activation.find(&padded) else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for matched in raw.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "industrial_language.corporate_speak",
                "message": format!(
                    "Minimize your use of corporate catchphrases like '{}'.",
                    matched.as_str()
                ),
                "span": [start, end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
        groups.push((first.start(), index, diagnostics));
    }
    groups.sort_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
