use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Rule terms copied from proselint 0.16.0 at dbed789caae662d06c7c8a5a13dd31f1acd36f5c.
// Source: proselint/checks/archaism.py, derived from Garner's Modern American Usage.
// Upstream project licence: BSD-3-Clause; retain Garner attribution with this data.
const FORMS: [&str; 47] = [
    "alack",
    "anent",
    "begat",
    "belike",
    "betimes",
    "boughten",
    "brocage",
    "brokage",
    "camarade",
    "chiefer",
    "chiefest",
    "Christiana",
    "completely obsolescent",
    "cozen",
    "divers",
    "deflexion",
    "durst",
    "fain",
    "forsooth",
    "foreclose from",
    "haply",
    "howbeit",
    "illumine",
    "in sooth",
    "maugre",
    "meseems",
    "methinks",
    "nigh",
    "peradventure",
    "perchance",
    "saith",
    "shew",
    "sistren",
    "spake",
    "to wit",
    "verily",
    "whilom",
    "withal",
    "wot",
    "enclosed please find",
    "please find enclosed",
    "enclosed herewith",
    "enclosed herein",
    "inforce",
    "ex postfacto",
    "forewent",
    "for ever",
];

static ACTIVATION: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|form| {
            RegexBuilder::new(&format!(r"(?i)(?-u:\b){form}(?-u:\b)"))
                .build()
                .expect("valid pinned archaism form")
        })
        .collect()
});

static RAW: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|form| {
            RegexBuilder::new(&format!("(?i){form}"))
                .build()
                .expect("valid pinned archaism form")
        })
        .collect()
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quote_bounds = paired_quote_bounds(&chars);
    let line_bounds = splitlines_bounds(&chars);
    let mut active = ACTIVATION
        .iter()
        .enumerate()
        .filter_map(|(index, pattern)| pattern.find(&padded).map(|m| (m.start(), index)))
        .collect::<Vec<_>>();
    active.sort_unstable();

    let mut output = Vec::new();
    for (_, index) in active {
        for matched in RAW[index].find_iter(&padded) {
            let padded_start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < padded_start) % 2 == 1 {
                continue;
            }
            // CheckResult spans refer to the newline-padded content itself. The
            // CLI's LintFile line/column conversion uses those same coordinates.
            let start = padded_start;
            let end = start + matched.as_str().chars().count();
            let (line, column) = line_col_of(padded_start, &line_bounds);
            output.push(json!({
                "check_path": "archaism",
                "message": format!("'{}' is archaic.", matched.as_str()),
                "span": [start, end],
                "pos": [line, column],
                "replacements": Value::Null,
            }));
        }
    }
    output
}

#[cfg(test)]
pub fn form_text_for_test() -> String {
    FORMS.join(" | ")
}

fn splitlines_bounds(chars: &[char]) -> Vec<usize> {
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

fn paired_quote_bounds(chars: &[char]) -> Vec<usize> {
    const QUOTES: [char; 6] = ['"', '\'', '“', '”', '‘', '’'];
    const SEPARATORS: &str = " ,.!?:;-\r\n";
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
