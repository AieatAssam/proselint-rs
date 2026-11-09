use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Garner's Modern American Usage (http://bit.ly/1T4alrY); data retained under package BSD-3-Clause basis.
const FORMS: [(&str, &str); 87] = [
    ("adequate enough", "adequate"),
    ("self-admitted", "admitted"),
    ("sworn affidavit", "affidavit"),
    ("mutual agreement", "agreement"),
    ("former alumnus", "alumnus"),
    ("directly antithetical", "antithetical"),
    ("approximately about", "approximately"),
    ("temporary bivouac", "bivouac"),
    ("bivouac camp", "bivouac"),
    ("blend together", "blend"),
    ("but nevertheless", "but"),
    ("accused of a charge", "charged with..."),
    ("circumstances surrounding", "circumstances of"),
    ("surrounding circumstances", "circumstances"),
    ("close proximity", "close"),
    ("collaborate together", "collaborate"),
    ("fellow collaborator", "collaborator"),
    ("fellow collaborators", "collaborators"),
    ("collocated together", "collocated"),
    ("fellow colleagues", "colleagues"),
    ("combine together", "combine"),
    ("self-complacent", "complacent"),
    ("self-confessed", "confessed"),
    ("connect together", "connect"),
    ("consolidate together", "consolidate"),
    ("still continues to", "continues to"),
    ("mutually contradictory", "contradictory"),
    ("mutual cooperation", "cooperation"),
    ("couple together", "couple"),
    ("serious crisis", "crisis"),
    ("entirely eliminate", "eliminate"),
    ("most especially", "especially"),
    ("actual fact", "fact"),
    ("true facts", "facts"),
    ("future forecast", "forecast"),
    ("founding forefathers", "founding fathers"),
    ("free and gratis", "free"),
    ("free gratis", "free"),
    ("completely full", "full"),
    ("basic fundamentals", "fundamentals"),
    ("free gift", "gift"),
    ("new innovation", "innovation"),
    ("interact with each other", "interact"),
    ("large-size", "large"),
    ("meld together", "meld"),
    ("merge together", "merge"),
    ("mingle together", "mingle"),
    ("mix together", "mix"),
    ("mutual feelings for eachother", "mutual feelings"),
    ("mutual respect for each other", "mutual respect"),
    ("native-born citizen", "native citizen"),
    ("absolute necessity", "necessity"),
    ("blatantly obvious", "obvious"),
    ("pause for a moment", "pause"),
    ("advance planning", "planning"),
    ("future plans", "plans"),
    ("pooled together", "pooled"),
    ("potable drinking water", "potable water"),
    ("new recruit", "recruit"),
    ("reelected for another term", "reelected"),
    ("refer back", "refer"),
    ("regress back", "regress"),
    ("repay them back", "repay them"),
    ("repay back", "repay"),
    ("repeat again", "repeat"),
    ("repeat back", "repeat"),
    ("repeat the same", "repeat"),
    ("repeated the same", "repeated"),
    ("temporary reprieve", "reprieve"),
    ("brief respite", "respite"),
    ("retiral", "retirement"),
    ("retiracy", "retirement"),
    ("retreat back", "retreat"),
    ("return back", "return"),
    ("closely scrutinize", "scrutinize"),
    ("software program", "software"),
    ("surrounded on all sides", "surrounded"),
    ("the whole entire nation", "the nation"),
    ("throughout the entire", "throughout the"),
    ("timpani drum", "timpani"),
    ("pair of twins", "twins"),
    ("unfilled vacancy", "vacancy"),
    ("various different", "various"),
    ("former veteran", "veteran"),
    ("visible to the eye", "visible"),
    ("professional vocation", "vocation"),
    ("while at the same time", "while"),
];
const REGEX_FORMS: [(&str, &str); 2] = [
    (r"(?:general )?consensus of opinion", "consensus"),
    (r"associate together(?: in groups)?", "associate"),
];
static SIMPLE_ACTIVATIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            RegexBuilder::new(&format!(r"\b{}\b", regex::escape(term)))
                .case_insensitive(true)
                .unicode(false)
                .build()
                .unwrap()
        })
        .collect()
});
static SIMPLE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            RegexBuilder::new(&regex::escape(term))
                .case_insensitive(true)
                .unicode(false)
                .build()
                .unwrap()
        })
        .collect()
});
static REGEX_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    REGEX_FORMS
        .iter()
        .map(|(pattern, _)| {
            RegexBuilder::new(&format!(r"\b{pattern}\b"))
                .case_insensitive(true)
                .unicode(false)
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
    let mut found = Vec::new();
    for (index, pattern) in SIMPLE_PATTERNS.iter().enumerate() {
        if SIMPLE_ACTIVATIONS[index].find(&padded).is_none() {
            continue;
        }
        for matched in pattern.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let original = matched.as_str();
            let replacement = FORMS[index].1;
            let end = start + original.chars().count();
            let (line, column) = crate::line_col_of(start, &lines);
            found.push((
                start,
                found.len(),
                json!({
                    "check_path": "redundancy.misc.garner",
                    "message": format!("Redundancy. Use '{replacement}' instead of '{original}'."),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }
    for (index, pattern) in REGEX_PATTERNS.iter().enumerate() {
        for matched in pattern.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let original = matched.as_str().trim();
            let replacement = REGEX_FORMS[index].1;
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &lines);
            found.push((
                start,
                found.len(),
                json!({
                    "check_path": "redundancy.misc.garner",
                    "message": format!("Redundancy. Use '{replacement}' instead of '{original}'."),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }
    found.sort_by_key(|(start, order, _)| (*start, *order));
    found.into_iter().map(|(_, _, value)| value).collect()
}
