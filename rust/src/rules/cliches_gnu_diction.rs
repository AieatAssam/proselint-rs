use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: GNU diction, https://directory.fsf.org/wiki/Diction; data SHA-256:
// a0c90a8267239fa17cc97621a66f896a2ebace9e12d72240ec8244208fbb5703.
// Reuse follows the user-authorized package BSD-3-Clause basis; this is not a legal guarantee.
static PATTERNS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    include_str!("../../data/gnu_diction.txt")
        .lines()
        .map(|phrase| {
            let activation = RegexBuilder::new(&format!(r"(?-u:\b){phrase}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned GNU diction phrase");
            let raw = RegexBuilder::new(phrase)
                .case_insensitive(true)
                .build()
                .expect("valid pinned GNU diction phrase");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `cliches.misc.gnu_diction` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut active = PATTERNS
        .iter()
        .enumerate()
        .filter_map(|(index, (activation, _))| {
            activation
                .find(&padded)
                .map(|matched| (matched.start(), index))
        })
        .collect::<Vec<_>>();
    active.sort_unstable();
    let mut findings = Vec::new();
    for (_, index) in active {
        let raw = &PATTERNS[index].1;
        for matched in raw.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            findings.push(json!({
                "check_path": "cliches.misc.gnu_diction",
                "message": format!("'{}' is a cliché", matched.as_str()),
                "span": [start, end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
    }
    findings
}
