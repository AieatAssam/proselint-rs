use regex::RegexBuilder;
use serde_json::{Value, json};

// Garner's Modern American Usage, as cited by the pinned Proselint check.
// Source: http://bit.ly/1T4alrY
// The upstream Proselint repository is BSD-3-Clause licensed.
const PATTERNS: [&str; 9] = [
    "preplan",
    "more than .{1,10} all",
    "appraisal valuations?",
    "(?:i|you|he|she|it|y'all|all y'all|you all|they) could care less",
    "least worst",
    "much-needed gaps?",
    "much-needed voids?",
    "no longer requires oxygen",
    "without scarcely",
];

/// Return diagnostics for the pinned `misc.illogic` existence check.
///
/// Fast matching first activates each source pattern with WORDS_IN_TEXT
/// boundaries, then reports all raw case-insensitive matches for that pattern.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let groups = PATTERNS
        .iter()
        .enumerate()
        .map(|(index, pattern)| {
            let raw = RegexBuilder::new(&format!("(?i)(?:{pattern})"))
                .build()
                .expect("valid pinned misc.illogic pattern");
            (index, raw)
        })
        .collect::<Vec<_>>();

    let mut activated_groups = Vec::new();
    for (index, raw) in groups {
        let Some(first) = raw.find_iter(&padded).find(|matched| {
            !is_ascii_word_before(&padded, matched.start())
                && !is_ascii_word_at(&padded, matched.end())
        }) else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for matched in raw.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < start);
            if quote_index % 2 == 1 {
                continue;
            }
            let surface = matched.as_str();
            let end = start + surface.chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            diagnostics.push((
                start,
                json!({
                    "check_path": "misc.illogic",
                    "message": format!("'{}' is illogical.", surface),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
        activated_groups.push((first.start(), index, diagnostics));
    }

    // The Fast engine emits the groups in order of their first activated
    // match, preserving source pattern order for ties.
    activated_groups.sort_unstable_by_key(|(start, index, _)| (*start, *index));
    activated_groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics.into_iter().map(|(_, value)| value))
        .collect()
}

fn is_ascii_word_before(text: &str, byte_index: usize) -> bool {
    byte_index > 0 && is_ascii_word(text.as_bytes()[byte_index - 1])
}

fn is_ascii_word_at(text: &str, byte_index: usize) -> bool {
    text.as_bytes()
        .get(byte_index)
        .is_some_and(|byte| is_ascii_word(*byte))
}

fn is_ascii_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
