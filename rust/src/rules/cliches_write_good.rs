use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const BATCH_COUNT: usize = 150;

static PATTERNS: LazyLock<Vec<Vec<Regex>>> = LazyLock::new(|| {
    include_str!("../../data/third_party/no_cliches_write_good.txt")
        .lines()
        .collect::<Vec<_>>()
        .chunks(BATCH_COUNT)
        .map(|batch| {
            batch
                .iter()
                .map(|pattern| {
                    RegexBuilder::new(pattern)
                        .case_insensitive(true)
                        .build()
                        .expect("valid pinned write-good cliché regex")
                })
                .collect()
        })
        .collect()
});

/// Return diagnostics for the five pinned `cliches.misc.write_good` checks.
///
/// Each 150-item Fast set activates raw case-insensitive regex matches with
/// RE2's ASCII `WORDS_IN_TEXT` boundaries. Findings follow batch and pattern
/// registration order, with each pattern's raw matches in text order.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut findings = Vec::new();

    for batch in PATTERNS.iter() {
        for regex in batch {
            let matches = regex.find_iter(&padded).collect::<Vec<_>>();
            let active = matches.iter().any(|matched| {
                !is_ascii_word_before(&padded, matched.start())
                    && !is_ascii_word_at(&padded, matched.end())
            });
            if active {
                for matched in matches {
                    let start = padded[..matched.start()].chars().count();
                    if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                        continue;
                    }
                    let surface = matched.as_str();
                    let end = start + surface.chars().count();
                    let (line, column) = crate::line_col_of(start, &line_bounds);
                    findings.push(json!({
                        "check_path": "cliches.misc.write_good",
                        "message": format!("'{surface}' is a cliché"),
                        "span": [start, end],
                        "pos": [line, column],
                        "replacements": null,
                    }));
                }
            }
        }
    }

    findings
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
