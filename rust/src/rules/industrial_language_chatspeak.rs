use regex::Regex;
use serde_json::{Value, json};
use std::sync::LazyLock;

const ITEMS: [&str; 23] = [
    "2day", "4U", "AFAIK", "AFK", "AFK", "ASAP", "B4", "brb", "btw", "cya", "GR8", "lol", "LOL",
    "LUV", "OMG", "rofl", "roftl", "sum1", "SWAK", "THNX", "THX", "TTYL", "XOXO",
];

static MATCHERS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    ITEMS
        .iter()
        .map(|item| {
            let escaped = regex::escape(item);
            let activation = regex::RegexBuilder::new(&format!(r"(?-u:\b){escaped}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned chatspeak activation");
            let raw = regex::RegexBuilder::new(&escaped)
                .case_insensitive(true)
                .build()
                .expect("valid pinned chatspeak item");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `industrial_language.chatspeak` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();
    for (index, (activation, raw)) in MATCHERS.iter().enumerate() {
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
                let end = start + matched.as_str().chars().count();
                let (line, column) = crate::line_col_of(start, &line_bounds);
                Some(json!({
                    "check_path": "industrial_language.chatspeak",
                    "message": format!("'{}' is chatspeak. Write it out.", matched.as_str()),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": null,
                }))
            })
            .collect::<Vec<_>>();
        groups.push((first.start(), index, diagnostics));
    }
    groups.sort_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
