use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const FORMS: [(&str, &str); 156] = [
    ("actionible", "actionable"),
    ("addible", "addable"),
    ("admittible", "admittable"),
    ("advisible", "advisable"),
    ("affectible", "affectable"),
    ("allegeible", "allegeable"),
    ("analyzible", "analyzable"),
    ("annexible", "annexable"),
    ("arrestible", "arrestable"),
    ("ascendible", "ascendable"),
    ("assertible", "assertable"),
    ("assessible", "assessable"),
    ("averageible", "averageable"),
    ("bailible", "bailable"),
    ("blamible", "blamable"),
    ("changeible", "changeable"),
    ("chargeible", "chargeable"),
    ("circumscribible", "circumscribable"),
    ("commensurible", "commensurable"),
    ("committible", "committable"),
    ("condensible", "condensable"),
    ("connectible", "connectable"),
    ("contestible", "contestable"),
    ("contractible", "contractable"),
    ("conversible", "conversable"),
    ("convictible", "convictable"),
    ("correctible", "correctable"),
    ("definible", "definable"),
    ("detectible", "detectable"),
    ("diagnosible", "diagnosable"),
    ("discussible", "discussable"),
    ("endorsible", "endorsable"),
    ("enforceible", "enforceable"),
    ("evadible", "evadable"),
    ("excisible", "excisable"),
    ("excludible", "excludable"),
    ("expandible", "expandable"),
    ("extendible", "extendable"),
    ("extractible", "extractable"),
    ("ignitible", "ignitable"),
    ("immovible", "immovable"),
    ("improvible", "improvable"),
    ("includible", "includable"),
    ("inferible", "inferable"),
    ("inventible", "inventable"),
    ("investible", "investable"),
    ("lapsible", "lapsable"),
    ("lovible", "lovable"),
    ("mixible", "mixable"),
    ("movible", "movable"),
    ("noticeible", "noticeable"),
    ("offendible", "offendable"),
    ("patentible", "patentable"),
    ("persuadible", "persuadable"),
    ("preventible", "preventable"),
    ("processible", "processable"),
    ("protectible", "protectable"),
    ("ratible", "ratable"),
    ("redressible", "redressable"),
    ("referible", "referable"),
    ("retractible", "retractable"),
    ("revisible", "revisable"),
    ("rinsible", "rinsable"),
    ("salible", "salable"),
    ("suspendible", "suspendable"),
    ("tractible", "tractable"),
    ("transferible", "transferable"),
    ("transmittible", "transmittable"),
    ("willible", "willable"),
    ("accessable", "accessible"),
    ("adducable", "adducible"),
    ("admissable", "admissible"),
    ("audable", "audible"),
    ("avertable", "avertible"),
    ("collapsable", "collapsible"),
    ("collectable", "collectible"),
    ("combustable", "combustible"),
    ("compactable", "compactible"),
    ("compatable", "compatible"),
    ("comprehensable", "comprehensible"),
    ("compressable", "compressible"),
    ("concussable", "concussible"),
    ("conductable", "conductible"),
    ("contemptable", "contemptible"),
    ("controvertable", "controvertible"),
    ("convertable", "convertible"),
    ("corrodable", "corrodible"),
    ("corruptable", "corruptible"),
    ("credable", "credible"),
    ("deducable", "deducible"),
    ("deductable", "deductible"),
    ("defeasable", "defeasible"),
    ("defensable", "defensible"),
    ("descendable", "descendible"),
    ("destructable", "destructible"),
    ("diffusable", "diffusible"),
    ("digestable", "digestible"),
    ("discernable", "discernible"),
    ("dismissable", "dismissible"),
    ("divisable", "divisible"),
    ("edable", "edible"),
    ("educable", "educible"),
    ("eligable", "eligible"),
    ("erodable", "erodible"),
    ("exhaustable", "exhaustible"),
    ("expressable", "expressible"),
    ("fallable", "fallible"),
    ("feasable", "feasible"),
    ("flexable", "flexible"),
    ("forcable", "forcible"),
    ("fusable", "fusible"),
    ("gullable", "gullible"),
    ("horrable", "horrible"),
    ("impressable", "impressible"),
    ("inadmissable", "inadmissible"),
    ("incorrigable", "incorrigible"),
    ("indelable", "indelible"),
    ("inexpressable", "inexpressible"),
    ("intelligable", "intelligible"),
    ("interfusable", "interfusible"),
    ("invincable", "invincible"),
    ("irascable", "irascible"),
    ("irresistable", "irresistible"),
    ("legable", "legible"),
    ("negligable", "negligible"),
    ("omissable", "omissible"),
    ("oppressable", "oppressible"),
    ("ostensable", "ostensible"),
    ("perceptable", "perceptible"),
    ("perfectable", "perfectible"),
    ("permissable", "permissible"),
    ("plausable", "plausible"),
    ("possable", "possible"),
    ("producable", "producible"),
    ("reducable", "reducible"),
    ("remissable", "remissible"),
    ("reprehensable", "reprehensible"),
    ("repressable", "repressible"),
    ("resistable", "resistible"),
    ("responsable", "responsible"),
    ("reversable", "reversible"),
    ("revertable", "revertible"),
    ("risable", "risible"),
    ("seducable", "seducible"),
    ("sensable", "sensible"),
    ("submersable", "submersible"),
    ("submergable", "submergible"),
    ("suggestable", "suggestible"),
    ("suppressable", "suppressible"),
    ("susceptable", "susceptible"),
    ("terrable", "terrible"),
    ("transfusable", "transfusible"),
    ("transmissable", "transmissible"),
    ("uncollectable", "uncollectible"),
    ("vendable", "vendible"),
    ("visable", "visible"),
];

#[cfg(test)]
pub fn form_text_for_test() -> String {
    FORMS
        .iter()
        .map(|(form, _)| *form)
        .collect::<Vec<_>>()
        .join(" ")
}

static BOUNDARY_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(original, _)| {
            RegexBuilder::new(&format!(r"(?i)(?-u:\b){original}(?-u:\b)"))
                .build()
                .expect("valid pinned able/ible activation form")
        })
        .collect()
});

static RAW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(original, _)| {
            RegexBuilder::new(&format!(r"(?i){original}"))
                .build()
                .expect("valid pinned able/ible output form")
        })
        .collect()
});

/// Return diagnostics for the pinned `spelling.able_ible` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quotes = paired_quote_bounds(&padded);
    let lines = splitlines_bounds(&padded);
    let mut active_patterns: Vec<(usize, usize)> = BOUNDARY_PATTERNS
        .iter()
        .enumerate()
        .filter_map(|(index, pattern)| {
            pattern
                .find(&padded)
                .map(|matched| (matched.start(), index))
        })
        .collect();
    active_patterns.sort_by_key(|(start, index)| (*start, *index));

    let mut diagnostics = Vec::new();
    for (_, index) in active_patterns {
        let (original, replacement) = FORMS[index];
        for matched in RAW_PATTERNS[index].find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            let surface = matched.as_str();
            let end = start + surface.chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let preferred = (surface.to_lowercase() == original).then_some(replacement);
            let (line, column) = line_col_of(start, &lines);
            diagnostics.push(json!({
                "check_path": "spelling.able_ible",
                "message": format!("-able vs. -ible. '{}' is the preferred spelling.", preferred.unwrap_or("None")),
                "span": [start, end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
    }

    diagnostics
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
