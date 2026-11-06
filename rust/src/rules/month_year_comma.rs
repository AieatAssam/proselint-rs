// Derived from pinned Proselint 0.16.0 `checks/dates_times/dates.py`
// (BSD-3-Clause); the source rule attributes its usage guidance to Garner's
// Modern American Usage.
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::OnceLock;

const CHECK_PATH: &str = "dates_times.dates.month_year_comma";
const MESSAGE: &str = "When specifying a month and year, no comma is needed.";

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        // Fast uses RE2 in the pinned Python implementation; its `\d` is ASCII.
        RegexBuilder::new(
            r"(?:January|February|March|April|May|June|July|August|September|October|November|December), [0-9]{3,}",
        )
        .case_insensitive(true)
        .unicode(false)
        .build()
        .expect("month/year comma regex is valid")
    })
}

/// Return diagnostics for the pinned `dates_times.dates.month_year_comma` rule.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    // Proselint's regex checks run over a text padded with one newline at both ends.
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quote_bounds = paired_quote_bounds(&chars);
    let mut findings = Vec::new();

    for matched in pattern().find_iter(&padded) {
        let start = padded[..matched.start()].chars().count();
        if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
            continue;
        }
        let end = start + padded[matched.start()..matched.end()].chars().count();
        let (line, column) = line_col(start, &chars);
        findings.push(json!({
            "check_path": CHECK_PATH,
            "message": MESSAGE,
            "span": [start, end],
            "pos": [line, column],
            "replacements": Value::Null,
        }));
    }

    findings
}

fn line_col(position: usize, chars: &[char]) -> (usize, usize) {
    let mut bounds = Vec::new();
    let mut index = 0;
    while index < position {
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
    let line = bounds.partition_point(|bound| *bound < position);
    (
        line,
        position - bounds.get(line.wrapping_sub(1)).copied().unwrap_or(0),
    )
}

fn paired_quote_bounds(chars: &[char]) -> Vec<usize> {
    let mut opening: Option<(char, usize)> = None;
    let mut bounds = Vec::new();
    for (index, quote) in chars.iter().copied().enumerate() {
        if !matches!(quote, '\'' | '"' | '“' | '”' | '‘' | '’') {
            continue;
        }
        let prev_word = index > 0 && is_word(chars[index - 1]);
        let next_word = chars.get(index + 1).is_some_and(|ch| is_word(*ch));
        if prev_word && next_word {
            continue;
        }
        match opening {
            None if index > 0 && is_separator(chars[index - 1]) => {
                opening = Some((quote, index));
            }
            Some((open, open_index))
                if quotes_match(open, quote)
                    && chars.get(index + 1).is_some_and(|ch| is_separator(*ch)) =>
            {
                bounds.extend([open_index, index]);
                opening = None;
            }
            _ => {}
        }
    }
    bounds
}

fn is_word(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn is_separator(ch: char) -> bool {
    " ,.!?:;-\r\n".contains(ch)
}

fn quotes_match(open: char, close: char) -> bool {
    (matches!(open, '\'' | '"') && open == close)
        || matches!(
            (open, close),
            ('“', '”') | ('”', '“') | ('‘', '’') | ('’', '‘')
        )
}
