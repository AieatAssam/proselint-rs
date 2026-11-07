// Derived from pinned Proselint 0.16.0 `checks/mixed_metaphors.py` (BSD-3-Clause).
// Phrase guidance cites Garner’s Modern American Usage in the pinned source.
// One regex key does not equal its matched text, so the pinned map lookup returns None.
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const ITEMS: [(&str, &str); 5] = [
    ("cream rises to the crop", "cream rises to the top"),
    ("button your seatbelts", "fasten your seatbelts"),
    ("a minute to decompose", "a minute to decompress"),
    (
        "sharpest marble in the (shed|box)",
        "sharpest tool in the shed",
    ),
    ("not rocket surgery", "not rocket science"),
];

static PATTERNS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    ITEMS
        .iter()
        .map(|(pattern, _)| {
            let activation = RegexBuilder::new(&format!(r"\b{pattern}\b"))
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid pinned mixed-metaphor pattern");
            let raw = RegexBuilder::new(pattern)
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid pinned mixed-metaphor pattern");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for pinned `mixed_metaphors.misc`.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();

    for (index, (activation, raw)) in PATTERNS.iter().enumerate() {
        let Some(first) = activation.find(&padded) else {
            continue;
        };
        let diagnostics = raw
            .find_iter(&padded)
            .filter_map(|matched| {
                let start = padded[..matched.start()].chars().count();
                if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                    return None;
                }
                let original = matched.as_str().to_lowercase();
                let replacement = ITEMS
                    .iter()
                    .find_map(|(key, replacement)| (*key == original).then_some(*replacement));
                let message_value = replacement.unwrap_or("None");
                let end = start + matched.as_str().chars().count();
                let (line, column) = crate::line_col_of(start, &line_bounds);
                Some(json!({
                    "check_path": "mixed_metaphors.misc",
                    "message": format!("Mixed metaphor. Try '{message_value}'."),
                    "pos": [line, column],
                    "replacements": replacement,
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
