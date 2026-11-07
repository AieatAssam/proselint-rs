use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Fowler's Modern English Usage, as cited by the pinned Proselint check.
const WAXES: [&str; 4] = ["wax", "waxes", "waxed", "waxing"];
const MODIFIERS: [(&str, &str); 16] = [
    ("ebulliently", "ebullient"),
    ("ecstatically", "ecstatic"),
    ("eloquently", "eloquent"),
    ("enthusiastically", "enthusiastic"),
    ("euphorically", "euphoric"),
    ("indignantly", "indignant"),
    ("lyrically", "lyrical"),
    ("melancholically", "melancholic"),
    ("metaphorically", "metaphorical"),
    ("nostalgically", "nostalgic"),
    ("patriotically", "patriotic"),
    ("philosophically", "philosophical"),
    ("poetically", "poetic"),
    ("rhapsodically", "rhapsodic"),
    ("romantically", "romantic"),
    ("sentimentally", "sentimental"),
];
static FORMS: LazyLock<Vec<(&'static str, &'static str, Regex, Regex)>> = LazyLock::new(|| {
    MODIFIERS
        .iter()
        .flat_map(|(wrong, right)| {
            WAXES.iter().map(move |wax| {
                let phrase = format!("{wax} {wrong}");
                let activation = RegexBuilder::new(&format!(r"(?i)\b{phrase}\b"))
                    .build()
                    .expect("valid pinned waxed phrase");
                let raw = RegexBuilder::new(&format!("(?i){phrase}"))
                    .build()
                    .expect("valid pinned waxed phrase");
                (*wax, *right, activation, raw)
            })
        })
        .collect()
});

/// Return diagnostics for the pinned `misc.waxed` preferred forms.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let lowered = text.to_ascii_lowercase();
    if !WAXES.iter().any(|wax| lowered.contains(*wax)) {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quotes = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();
    for (index, (wax, replacement, activation, raw)) in FORMS.iter().enumerate() {
        if !activation.is_match(&padded) {
            continue;
        }
        let matches: Vec<_> = raw
            .find_iter(&padded)
            .filter_map(|matched| {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                    return None;
            }
            let surface = matched.as_str();
            let end = start + surface.chars().count();
            let preferred = format!("{wax} {replacement}");
            let (line, column) = crate::line_col_of(start, &line_bounds);
                Some(json!({
                    "check_path": "misc.waxed",
                    "message": format!("The modifier following 'waxed' must be an adj.: '{}' is correct.", preferred),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": preferred,
                }))
            })
            .collect();
        if let Some(first) = activation.find(&padded) {
            groups.push((first.start(), index, matches));
        }
    }
    groups.sort_unstable_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
