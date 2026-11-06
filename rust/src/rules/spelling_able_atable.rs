use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Copied in declaration order from proselint 0.16.0 at dbed789caae662d06c7c8a5a13dd31f1acd36f5c.
// Pinned source SHA-256: a13cb4045f35993421f423968c65efe4f666d3e6b4fbbedd509ae84841b9aae4.
const FORMS: [(&str, &str); 68] = [
    ("abbreviatable", "abbreviable"),
    ("abdicatable", "abdicable"),
    ("abrogatable", "abrogable"),
    ("accommodatable", "accommodable"),
    ("accumulatable", "accumulable"),
    ("activatable", "activable"),
    ("administratable", "administrable"),
    ("adulteratable", "adulterable"),
    ("affiliatable", "affiliable"),
    ("aggregatable", "aggregable"),
    ("agitatable", "agitable"),
    ("alienatable", "alienable"),
    ("alleviatable", "alleviable"),
    ("allocatable", "allocable"),
    ("amelioratable", "ameliorable"),
    ("annihilatable", "annihilable"),
    ("appreciatable", "appreciable"),
    ("appropriatable", "appropriable"),
    ("arbitratable", "arbitrable"),
    ("articulatable", "articulable"),
    ("calculatable", "calculable"),
    ("communicatable", "communicable"),
    ("compensatable", "compensable"),
    ("confiscatable", "confiscable"),
    ("corroboratable", "corroborable"),
    ("cultivatable", "cultivable"),
    ("delegatable", "delegable"),
    ("delineatable", "delineable"),
    ("demonstratable", "demonstrable"),
    ("detonatable", "detonable"),
    ("differentiatable", "differentiable"),
    ("eradicatable", "eradicable"),
    ("evacuatable", "evacuable"),
    ("evaluatable", "evaluable"),
    ("expropriatable", "expropriable"),
    ("generatable", "generable"),
    ("indicatable", "indicable"),
    ("inebriatable", "inebriable"),
    ("inextirpatable", "inextirpable"),
    ("inextricatable", "inextricable"),
    ("infatuatable", "infatuable"),
    ("infuriatable", "infuriable"),
    ("invalidatable", "invalidable"),
    ("investigatable", "investigable"),
    ("isolatable", "isolable"),
    ("litigatable", "litigable"),
    ("manipulatable", "manipulable"),
    ("medicatable", "medicable"),
    ("navigatable", "navigable"),
    ("obligatable", "obligable"),
    ("obviatable", "obviable"),
    ("operatable", "operable"),
    ("originatable", "originable"),
    ("participatable", "participable"),
    ("penetratable", "penetrable"),
    ("perpetratable", "perpetrable"),
    ("perpetuatable", "perpetuable"),
    ("predicatable", "predicable"),
    ("propagatable", "propagable"),
    ("regulatable", "regulable"),
    ("replicatable", "replicable"),
    ("repudiatable", "repudiable"),
    ("segregatable", "segregable"),
    ("separatable", "separable"),
    ("subjugatable", "subjugable"),
    ("vindicatable", "vindicable"),
    ("violatable", "violable"),
    ("vitiatable", "vitiable"),
];

static ACTIVATION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(original, _)| {
            RegexBuilder::new(&format!(r"(?i)(?-u:\b){original}(?-u:\b)"))
                .build()
                .expect("valid pinned able/atable form")
        })
        .collect()
});

static RAW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(original, _)| {
            RegexBuilder::new(&format!(r"(?i){original}"))
                .build()
                .expect("valid pinned able/atable form")
        })
        .collect()
});

/// Return diagnostics for the pinned `spelling.able_atable` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let lines = splitlines_bounds(&padded);
    let mut active = Vec::new();

    // FastSet activates patterns by their first match in the text, then emits all matches for each.
    for (index, pattern) in ACTIVATION_PATTERNS.iter().enumerate() {
        if let Some(first) = pattern.find(&padded) {
            active.push((first.start(), index));
        }
    }
    active.sort_by_key(|(start, _)| *start);

    let mut diagnostics = Vec::new();
    for (_, index) in active {
        let (original, replacement) = FORMS[index];
        for matched in RAW_PATTERNS[index].find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let surface = matched.as_str();
            let end = start + surface.chars().count();
            let preferred = (surface.to_lowercase() == original).then_some(replacement);
            let (line, column) = line_col_of(start, &lines);
            diagnostics.push(json!({
                "check_path": "spelling.able_atable",
                "message": format!("-able vs. -atable. '{}' is the preferred spelling.", preferred.unwrap_or("None")),
                "span": [start, end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
    }

    diagnostics
}

#[cfg(test)]
pub fn form_text_for_test() -> String {
    FORMS
        .iter()
        .map(|(form, _)| *form)
        .collect::<Vec<_>>()
        .join(" ")
}

fn splitlines_bounds(text: &str) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut bounds = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let end = match chars[index] {
            '\r' if chars.get(index + 1) == Some(&'\n') => index + 2,
            '\r' | '\n' | '\u{000b}' | '\u{000c}' | '\u{001c}' | '\u{001d}' | '\u{001e}'
            | '\u{0085}' | '\u{2028}' | '\u{2029}' => index + 1,
            _ => {
                index += 1;
                continue;
            }
        };
        bounds.push(end - 1);
        index = end;
    }
    bounds
}

fn line_col_of(position: usize, bounds: &[usize]) -> (usize, usize) {
    let line = bounds.partition_point(|bound| *bound < position);
    (line, position - bounds[line - 1])
}

fn paired_quote_bounds(text: &str) -> Vec<usize> {
    const QUOTES: [char; 6] = ['"', '\'', '“', '”', '‘', '’'];
    const SEPARATORS: &str = " ,.!?:;-\r\n";
    let chars: Vec<char> = text.chars().collect();
    let mut opening: Option<(char, usize)> = None;
    let mut bounds = Vec::new();
    for (index, quote) in chars.iter().copied().enumerate() {
        if !QUOTES.contains(&quote) {
            continue;
        }
        let prev_word = index > 0 && is_unicode_word(chars[index - 1]);
        let next_word = chars.get(index + 1).is_some_and(|ch| is_unicode_word(*ch));
        // Mirrors Python's `(?:\B[quotes]|[quotes]\B)` quote candidate filter.
        if prev_word && next_word {
            continue;
        }
        match opening {
            None if index > 0 && SEPARATORS.contains(chars[index - 1]) => {
                opening = Some((quote, index));
            }
            None => {}
            Some((open, start))
                if quotes_match(open, quote)
                    && chars
                        .get(index + 1)
                        .is_some_and(|ch| SEPARATORS.contains(*ch)) =>
            {
                bounds.extend([start, index]);
                opening = None;
            }
            Some(_) => {}
        }
    }
    bounds
}

fn is_unicode_word(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn quotes_match(open: char, close: char) -> bool {
    (matches!(open, '\'' | '"') && open == close)
        || matches!(
            (open, close),
            ('“', '”') | ('”', '“') | ('‘', '’') | ('’', '‘')
        )
}
