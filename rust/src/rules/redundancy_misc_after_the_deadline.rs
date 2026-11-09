use regex::RegexBuilder;
use serde_json::{Value, json};
use std::sync::LazyLock;

// This pinned list has unknown list-specific provenance; do not imply a separate source attribution.
const RAW: &str = include_str!("data/after-the-deadline.txt");
const BATCH_COUNT: usize = 150;
static FORMS: LazyLock<Vec<Vec<(&str, &str)>>> = LazyLock::new(|| {
    let entries = RAW
        .lines()
        .map(|line| {
            line.split_once(',')
                .expect("pinned redundancy row has a comma")
        })
        .collect::<Vec<_>>();
    entries
        .chunks(BATCH_COUNT)
        .map(|batch| batch.to_vec())
        .collect()
});
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let lower = padded.to_ascii_lowercase();
    let quotes = crate::paired_quote_bounds(&padded);
    let lines = crate::splitlines_bounds(&padded);
    let mut found = Vec::new();
    for batch in FORMS.iter() {
        for (term, _) in batch {
            if !has_regex_meta(term) && !lower.contains(&term.to_ascii_lowercase()) {
                continue;
            }
            let activation = RegexBuilder::new(&format!(r"(?-u:\b){term}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .unwrap();
            if !activation.is_match(&padded) {
                continue;
            }
            let raw = RegexBuilder::new(term)
                .case_insensitive(true)
                .build()
                .unwrap();
            for matched in raw.find_iter(&padded) {
                let start = padded[..matched.start()].chars().count();
                if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                    continue;
                }
                let original = matched.as_str().trim();
                let replacement = batch
                    .iter()
                    .find(|(key, _)| *key == original.to_lowercase())
                    .map(|(_, replacement)| *replacement);
                let end = start + matched.as_str().chars().count();
                let (line, column) = crate::line_col_of(start, &lines);
                found.push((
                    start,
                    found.len(),
                    json!({
                        "check_path": "redundancy.misc.after_the_deadline",
                        "message": format!(
                            "Redundancy. Use '{}' instead of '{}'.",
                            replacement.unwrap_or("None"),
                            original
                        ),
                        "span": [start, end],
                        "pos": [line, column],
                        "replacements": replacement,
                    }),
                ));
            }
        }
    }
    found.sort_by_key(|(start, order, _)| (*start, *order));
    found.into_iter().map(|(_, _, value)| value).collect()
}

fn has_regex_meta(term: &str) -> bool {
    term.bytes().any(|byte| b"\\.^$*+?{}[]|()".contains(&byte))
}
