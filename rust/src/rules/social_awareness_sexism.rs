use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const FORMS: [(&str, &str); 35] = [
    ("anchorman", "anchor"),
    ("anchorwoman", "anchor"),
    ("chairman", "chair"),
    ("chairwoman", "chair"),
    ("draftman", "drafter"),
    ("draftwoman", "drafter"),
    ("ombudsman", "ombuds"),
    ("ombudswoman", "ombuds"),
    ("tribesman", "tribe member"),
    ("tribeswoman", "tribe member"),
    ("policeman", "police officer"),
    ("policewoman", "police officer"),
    ("fireman", "firefighter"),
    ("firewoman", "firefighter"),
    ("mailman", "mail carrier"),
    ("mailwoman", "mail carrier"),
    ("herstory", "history"),
    ("womyn", "women"),
    ("poetess", "poet"),
    ("authoress", "author"),
    ("waitress", "waiter"),
    ("lady lawyer", "lawyer"),
    ("woman doctor", "doctor"),
    ("female booksalesman", "bookseller"),
    ("female airman", "air pilot"),
    ("executrix", "executor"),
    ("prosecutrix", "prosecutor"),
    ("testatrix", "testator"),
    ("man and wife", "husband and wife"),
    ("chairmen and chairs", "chairs"),
    ("men and girls", "men and women"),
    ("comedienne", "comedian"),
    ("confidante", "confidant"),
    ("woman scientist", "scientist"),
    ("women scientists", "scientists"),
];
const PREFERRED: [(&str, &str); 8] = [
    ("anchorperson", "anchor"),
    ("chairperson", "chair"),
    ("draftperson", "drafter"),
    ("ombudsperson", "ombuds"),
    ("tribesperson", "tribe member"),
    ("policeperson", "police officer"),
    ("fireperson", "firefighter"),
    ("mailperson", "mail carrier"),
];
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .chain(PREFERRED.iter())
        .map(|(term, _)| {
            RegexBuilder::new(&regex::escape(term))
                .case_insensitive(true)
                .build()
                .unwrap()
        })
        .collect()
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quotes = crate::paired_quote_bounds(&padded);
    let lines = crate::splitlines_bounds(&padded);
    let forms: Vec<_> = FORMS.iter().chain(PREFERRED.iter()).collect();
    let mut active = Vec::new();
    for (index, pattern) in PATTERNS.iter().enumerate() {
        let Some(activation) = pattern
            .find_iter(&padded)
            .find(|m| {
                !is_word(padded.as_bytes().get(m.start().wrapping_sub(1)).copied())
                    && !is_word(padded.as_bytes().get(m.end()).copied())
            })
            .map(|m| m.start())
        else {
            continue;
        };
        let mut found = Vec::new();
        for matched in pattern.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let original = matched.as_str();
            let replacement = forms[index].1;
            let message = if index < FORMS.len() {
                "Gender bias"
            } else {
                "Not a preferred form"
            };
            let (line, column) = crate::line_col_of(start, &lines);
            found.push((start, json!({
                "check_path": "social_awareness.sexism",
                "message": format!("{message}. Use '{}' instead of '{}'.", replacement, original),
                "span": [start, end], "pos": [line, column], "replacements": replacement
            })));
        }
        active.push((activation, index, found));
    }
    active.sort_by_key(|(activation, index, _)| (*activation, *index));
    active
        .into_iter()
        .flat_map(|(_, _, found)| found)
        .map(|(_, value)| value)
        .collect()
}

fn is_word(byte: Option<u8>) -> bool {
    byte.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
}
