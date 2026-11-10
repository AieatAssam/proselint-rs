use regex::RegexBuilder;
use serde_json::{Value, json};

// Ported from Proselint 0.16.0 proselint/checks/uncomparables.py (SHA-256 e6842c6cf15f2ee47782fa9d48d04ad3a9e5b5f29b6d3c89db6d02cedfd61582). The user-approved package BSD-3-Clause basis is not a row-specific license determination.
const COMPARATORS: &[&str] = &[
    "extremely",
    "increasingly",
    "kind of",
    "largely",
    "less",
    "mildly",
    "more",
    "quite",
    "very",
];
const FANCY_COMPARATORS: &[&str] = &["least", "most"];
const UNCOMPARABLES: &[&str] = &[
    "absolute",
    "adequate",
    "chief",
    "complete",
    "correct",
    "devoid",
    "entire",
    "false",
    "fatal",
    "favorite",
    "final",
    "ideal",
    "impossible",
    "inevitable",
    "infinite",
    "irrevocable",
    "main",
    "manifest",
    "only",
    "paramount",
    "perfect",
    "perpetual",
    "possible",
    "preferable",
    "principal",
    "singular",
    "stationary",
    "sufficient",
    "true",
    "unanimous",
    "unavoidable",
    "unbroken",
    "uniform",
    "unique",
    "universal",
    "void",
    "whole",
];
const EXCEPTIONS: &[(&str, &str)] = &[("more", "perfect"), ("more", "possible")];
const MESSAGE: &str = "Comparison of an uncomparable: '{}' is not comparable.";

pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quotes = crate::paired_quote_bounds(&padded);
    let lines = crate::splitlines_bounds(&padded);
    let mut fast_groups = Vec::new();
    let mut pattern_index = 0;
    for comparator in COMPARATORS {
        for adjective in UNCOMPARABLES {
            if EXCEPTIONS.contains(&(comparator, adjective)) {
                continue;
            }
            let expression = format!(
                "{}\\s+{}",
                regex::escape(comparator),
                regex::escape(adjective)
            );
            let regex = RegexBuilder::new(&expression)
                .case_insensitive(true)
                .build()
                .unwrap();
            let activation = RegexBuilder::new(&format!(
                r"(?-u:\b){}\s+{}(?-u:\b)",
                regex::escape(comparator),
                regex::escape(adjective)
            ))
            .case_insensitive(true)
            .build()
            .unwrap()
            .find(&padded);
            if let Some(activation) = activation {
                let mut group = Vec::new();
                for matched in regex.find_iter(&padded) {
                    push(
                        &mut group,
                        &padded,
                        &lines,
                        &quotes,
                        matched.start(),
                        matched.end(),
                        matched.as_str(),
                    );
                }
                fast_groups.push((activation.start(), pattern_index, group));
            }
            pattern_index += 1;
        }
    }
    fast_groups.sort_by_key(|(start, index, _)| (*start, *index));
    let mut found = fast_groups
        .into_iter()
        .flat_map(|(_, _, group)| group)
        .collect::<Vec<_>>();

    // Fancy matcher findings are emitted left-to-right by the upstream regex set.
    let mut fancy_findings = Vec::new();
    let mut fancy_index = 0;
    for comparator in FANCY_COMPARATORS {
        for adjective in UNCOMPARABLES {
            let expression = format!(
                r"(?-u:\b){}\s+{}(?-u:\b)",
                regex::escape(comparator),
                regex::escape(adjective)
            );
            let regex = RegexBuilder::new(&expression)
                .case_insensitive(true)
                .build()
                .unwrap();
            for matched in regex.find_iter(&padded) {
                let prefix = &padded[..matched.start()];
                let mut chars = prefix.chars().rev();
                let suppress = chars.next().is_some_and(char::is_whitespace)
                    && chars.next().is_some_and(|ch| ch.eq_ignore_ascii_case(&'t'))
                    && chars.next().is_some_and(|ch| ch.eq_ignore_ascii_case(&'a'));
                if !suppress {
                    let mut one = Vec::new();
                    push(
                        &mut one,
                        &padded,
                        &lines,
                        &quotes,
                        matched.start(),
                        matched.end(),
                        matched.as_str(),
                    );
                    fancy_findings.push((matched.start(), fancy_index, one));
                }
            }
            fancy_index += 1;
        }
    }
    fancy_findings.sort_by_key(|(start, index, _)| (*start, *index));
    found.extend(fancy_findings.into_iter().flat_map(|(_, _, group)| group));
    found
}

fn push(
    found: &mut Vec<Value>,
    padded: &str,
    lines: &[usize],
    quotes: &[usize],
    byte_start: usize,
    byte_end: usize,
    surface: &str,
) {
    let start = padded[..byte_start].chars().count();
    if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
        return;
    }
    let end = padded[..byte_end].chars().count();
    let (line, column) = crate::line_col_of(start, lines);
    found.push(json!({
        "check_path": "uncomparables",
        "message": MESSAGE.replace("{}", surface),
        "span": [start, end],
        "pos": [line, column],
        "replacements": null,
    }));
}
