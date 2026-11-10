use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::LazyLock;

// Source: The Basic Spelling Vocabulary List, https://tinyurl.com/5n6nczv2.
// The upstream package's BSD-3-Clause license does not establish separate rights to this list.
const ELEMENTARY_RAW: &str = include_str!("data/restricted-elementary.txt");
// Source: The Up-Goer Five Text Editor, https://splasho.com/upgoer5/.
// The upstream package's BSD-3-Clause license does not establish separate rights to this list.
const TOP1000_RAW: &str = include_str!("data/restricted-top1000.txt");

static ELEMENTARY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ELEMENTARY_RAW.lines().collect());
static TOP1000: LazyLock<HashSet<&'static str>> = LazyLock::new(|| TOP1000_RAW.lines().collect());
static TOKENIZER: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?-u:\b)[A-Za-z_][A-Za-z_'-]+[A-Za-z_](?-u:\b)")
        .case_insensitive(true)
        .build()
        .expect("valid pinned restricted-word tokenizer")
});

pub fn elementary_diagnostics(text: &str) -> Vec<Value> {
    diagnostics(
        text,
        "restricted.elementary",
        "'{}' is not a word kids learn in elementary school.",
        &ELEMENTARY,
    )
}

pub fn top1000_diagnostics(text: &str) -> Vec<Value> {
    diagnostics(
        text,
        "restricted.top1000",
        "'{}' is not in the top 1000 most common words.",
        &TOP1000,
    )
}

fn diagnostics(
    text: &str,
    check_path: &str,
    message: &str,
    allowed: &HashSet<&'static str>,
) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quotes = crate::paired_quote_bounds(&padded);
    let lines = crate::splitlines_bounds(&padded);
    TOKENIZER
        .find_iter(&padded)
        .filter_map(|matched| {
            let original = matched.as_str();
            if allowed.contains(original.to_ascii_lowercase().as_str()) {
                return None;
            }
            let start = padded[..matched.start()].chars().count() + 1;
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                return None;
            }
            let end = padded[..matched.end()].chars().count();
            let (line, column) = crate::line_col_of(start, &lines);
            Some(json!({
                "check_path": check_path,
                "message": message.replace("{}", original),
                "span": [start, end],
                "pos": [line, column],
                "replacements": null,
            }))
        })
        .collect()
}
