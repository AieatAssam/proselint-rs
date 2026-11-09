use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Garner's Modern American Usage, http://bit.ly/1T4alrY.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const PATTERNS: [&str; 26] = [
    "acknowledging yours of",
    "beg to advise",
    "enclosed herewith",
    "enclosed please find",
    "further to yours of",
    "further to your letter",
    "in regard to",
    r"inst\.",
    "in the amount of",
    "of even date",
    "pending receipt of",
    "please be advised that",
    "please return same",
    "pleasure of a reply",
    r"prox\.",
    "pursuant to your request",
    "regarding the matter",
    "regret to inform",
    "thanking you in advance",
    "the undersigned",
    "this acknowledges your letter",
    r"ult\.",
    "we are pleased to note",
    "with regard to",
    "your favor has come to hand",
    "yours of even date",
];

static MATCHERS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .map(|pattern| {
            let activation = RegexBuilder::new(&format!(r"(?-u:\b){pattern}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned commercialese activation pattern");
            let raw = RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .expect("valid pinned commercialese pattern");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `industrial_language.commercialese` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();

    for (index, (activation, raw)) in MATCHERS.iter().enumerate() {
        let Some(first) = activation.find(&padded) else {
            continue;
        };
        let diagnostics: Vec<Value> = raw
            .find_iter(&padded)
            .map(|matched| {
                let start = padded[..matched.start()].chars().count();
                let end = start + matched.as_str().chars().count();
                let (line, column) = crate::line_col_of(start, &line_bounds);
                json!({
                    "check_path": "industrial_language.commercialese",
                    "message": format!("'{}' is commercialese.", matched.as_str()),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": null,
                })
            })
            .collect();
        groups.push((first.start(), index, diagnostics));
    }

    groups.sort_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
