use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// DFW's Tense Present; the source pattern's Unicode escapes are retained
// verbatim from the pinned check.
const PATTERNS: [&str; 10] = [
    r"up to \d{1,3}% ?[-\x{2014}\x{2013}]{0,3} ?(?:or|and) more\W?",
    "between you and I",
    "on accident",
    "somewhat of a",
    "all it's own",
    "reason is because",
    "audible to the ear",
    "in regards to",
    "would of",
    r"i ?(?:feel|am feeling|am|'m|'m feeling) nauseous",
];

static REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .map(|pattern| {
            let pattern = pattern
                .replace(r"\x{2014}", "—")
                .replace(r"\x{2013}", "–")
                .replace(r"\d", "[0-9]")
                .replace(r"\W", "[^A-Za-z0-9_]");
            RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
                .expect("valid pinned misc.tense_present RE2 pattern")
        })
        .collect()
});

fn ascii_word_before(text: &str, byte_index: usize) -> bool {
    byte_index > 0 && ascii_word(text.as_bytes()[byte_index - 1])
}

fn ascii_word_at(text: &str, byte_index: usize) -> bool {
    text.as_bytes()
        .get(byte_index)
        .copied()
        .is_some_and(ascii_word)
}

fn ascii_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Fast `Existence` first tests WORDS_IN_TEXT padded patterns, then reports
/// every raw occurrence for each activated pattern. RE2's boundaries are
/// ASCII, so activation is computed around raw matches rather than using the
/// Rust regex crate's Unicode `\b` implementation.
fn activates_with_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    if ascii_word_before(text, start) || !ascii_word_at(text, start) {
        return false;
    }

    let mut boundary_end = end;
    // Pattern 1 optionally consumes one trailing RE2 non-word character.
    // During padded activation, the outer right \b can backtrack before that
    // character; recognize that same endpoint while retaining it in raw hits.
    if !ascii_word_before(text, boundary_end)
        && let Some((previous_start, previous_char)) = text[..boundary_end].char_indices().last()
        && !(previous_char.is_ascii() && ascii_word(previous_char as u8))
        && ascii_word_before(text, previous_start)
    {
        boundary_end = previous_start;
    }

    ascii_word_before(text, boundary_end) && !ascii_word_at(text, boundary_end)
}

/// Diagnostics for the pinned default-enabled `misc.tense_present` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();

    for (index, regex) in REGEXES.iter().enumerate() {
        let activation_matches = regex.find_iter(&padded).collect::<Vec<_>>();
        let first_active = activation_matches
            .iter()
            .find(|matched| activates_with_word_boundaries(&padded, matched.start(), matched.end()))
            .map(|matched| matched.start());
        if let Some(first_active) = first_active {
            groups.push((
                first_active,
                index,
                regex.find_iter(text).collect::<Vec<_>>(),
            ));
        }
    }
    groups.sort_unstable_by_key(|(first_active, index, _)| (*first_active, *index));

    groups
        .into_iter()
        .flat_map(|(_, _, matches)| {
            matches.into_iter().filter_map(|matched| {
                let span_start = padded[..matched.start() + 1].chars().count();
                let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
                if quote_index % 2 == 1 {
                    return None;
                }

                let surface = matched.as_str();
                let span_end = span_start + surface.chars().count();
                let (line, column) = crate::line_col_of(span_start, &line_bounds);
                Some(json!({
                    "check_path": "misc.tense_present",
                    "message": format!("'{surface}'."),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }))
            })
        })
        .collect()
}
