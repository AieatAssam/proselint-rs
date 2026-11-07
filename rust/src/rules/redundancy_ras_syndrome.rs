// Derived from pinned Proselint 0.16.0 `checks/redundancy/ras_syndrome.py` (BSD-3-Clause).
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const ITEMS: [(&str, &str); 20] = [
    ("abm missile", "ABM"),
    ("act test", "ACT"),
    ("abm missiles", "ABMs"),
    ("abs braking system", "ABS"),
    ("atm machine", "ATM"),
    ("cd disc", "CD"),
    ("cpi Index", "CPI"),
    ("gps system", "GPS"),
    ("gui interface", "GUI"),
    ("hiv virus", "HIV"),
    ("isbn number", "ISBN"),
    ("lcd display", "LCD"),
    ("pdf format", "PDF"),
    ("pin number", "PIN"),
    ("ras syndrome", "RAS"),
    ("rip in peace", "RIP"),
    ("please RSVP", "RSVP"),
    ("salT talks", "SALT"),
    ("sat test", "SAT"),
    ("upc codes", "UPC"),
];

static PATTERNS: LazyLock<Vec<(Regex, Regex, Option<&'static str>)>> = LazyLock::new(|| {
    ITEMS
        .iter()
        .map(|(item, _)| {
            let pattern = regex::escape(item);
            let activation = RegexBuilder::new(&format!(r"\b{pattern}\b"))
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid pinned RAS phrase");
            let raw = RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid pinned RAS phrase");
            // PreferredFormsSimple lowercases matches but retains these mixed-case source keys.
            let lowercase = item.to_lowercase();
            let replacement = ITEMS
                .iter()
                .find_map(|(key, replacement)| (*key == lowercase).then_some(*replacement));
            (activation, raw, replacement)
        })
        .collect()
});

/// Return diagnostics for the pinned `redundancy.ras_syndrome` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quotes = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();

    for (index, (activation, raw, replacement)) in PATTERNS.iter().enumerate() {
        let Some(first) = activation.find(&padded) else {
            continue;
        };
        let matches = raw
            .find_iter(&padded)
            .filter_map(|matched| {
                let start = padded[..matched.start()].chars().count();
                if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                    return None;
                }
                let surface = matched.as_str();
                let end = start + surface.chars().count();
                let shown = replacement.unwrap_or("None");
                let (line, column) = crate::line_col_of(start, &line_bounds);
                Some(json!({
                    "check_path": "redundancy.ras_syndrome",
                    "message": format!("RAS syndrome. Use '{shown}' instead of '{surface}'."),
                    "pos": [line, column],
                    "replacements": replacement,
                    "span": [start, end],
                }))
            })
            .collect::<Vec<_>>();
        groups.push((first.start(), index, matches));
    }

    groups.sort_unstable_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
