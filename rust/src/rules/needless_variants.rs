use regex::RegexBuilder;
use serde_json::{Value, json};
use std::sync::LazyLock;

// Attributed upstream to Garner's Modern American Usage; the package-wide BSD-3-Clause reuse basis is user-approved, not a row-specific license determination or legal guarantee.
const RAW: &str = include_str!("../../data/needless_variants.txt");
const BATCH_COUNT: usize = 150;
static FORMS: LazyLock<Vec<Vec<(&str, &str)>>> = LazyLock::new(|| {
    let entries = RAW
        .lines()
        .map(|line| {
            line.split_once(',')
                .expect("pinned needless variant row has a comma")
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
    let quotes = crate::paired_quote_bounds(&padded);
    let lines = crate::splitlines_bounds(&padded);
    let mut found = Vec::new();
    for batch in FORMS.iter() {
        for (term, replacement) in batch {
            let activation =
                RegexBuilder::new(&format!(r"(?-u:\b){}(?-u:\b)", regex::escape(term)))
                    .case_insensitive(true)
                    .build()
                    .unwrap();
            if !activation.is_match(&padded) {
                continue;
            }
            let raw = RegexBuilder::new(&regex::escape(term))
                .case_insensitive(true)
                .build()
                .unwrap();
            for matched in raw.find_iter(&padded) {
                let start = padded[..matched.start()].chars().count();
                if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                    continue;
                }
                let original = matched.as_str();
                let end = start + original.chars().count();
                let (line, column) = crate::line_col_of(start, &lines);
                found.push((start, found.len(), json!({
                    "check_path": "needless_variants",
                    "message": format!("Needless variant. '{}' is the preferred form.", replacement),
                    "span": [start, end], "pos": [line, column], "replacements": replacement,
                })));
            }
        }
    }
    found.into_iter().map(|(_, _, value)| value).collect()
}
