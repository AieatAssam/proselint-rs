use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Equivalent to the pinned Fancy regex, with its negative lookahead applied
// separately because the Rust regex crate intentionally does not support lookaround.
static CANDIDATE: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(
        r"(?i)from (?P<year>\d+)(?P<separator>[^ \t\n\r\f\v\p{L}\p{N}_\.])(?P<end>\d+)",
    )
    .build()
    .expect("valid dash_and_from candidate regex")
});
static EXCLUDED_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[-/]\d{2}[-/]\d{2}|:\d{2})").expect("valid negative-lookahead suffix")
});

/// Return diagnostics for the pinned dates_times.dates.dash_and_from check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let lines = splitlines_bounds(&padded);
    let quote_bounds = paired_quote_bounds(&padded);
    let mut output = Vec::new();
    for captures in CANDIDATE.captures_iter(&padded) {
        let whole = captures.get(0).expect("whole match");
        let year = captures.name("year").expect("year capture");
        if EXCLUDED_SUFFIX.is_match(&padded[year.end()..]) {
            continue;
        }
        let start = padded[..whole.start()].chars().count();
        if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
            continue;
        }
        let end = start + whole.as_str().chars().count();
        let (line, column) = line_col_of(start, &lines);
        output.push(json!({
            "check_path": "dates_times.dates.dash_and_from",
            "message": "When specifying a date range, write 'from X to Y'.",
            "span": [start, end],
            "pos": [line, column],
            "replacements": null,
        }));
    }
    output
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
    let chars: Vec<char> = text.chars().collect();
    let mut opening: Option<(char, usize)> = None;
    let mut bounds = Vec::new();
    for (index, quote) in chars.iter().copied().enumerate() {
        if !matches!(quote, '\'' | '"' | '“' | '”' | '‘' | '’') {
            continue;
        }
        let prev_word = index > 0 && is_unicode_word(chars[index - 1]);
        let next_word = chars.get(index + 1).is_some_and(|ch| is_unicode_word(*ch));
        if prev_word && next_word {
            continue;
        }
        match opening {
            None => {
                if index > 0 && is_separator(chars[index - 1]) {
                    opening = Some((quote, index));
                }
            }
            Some((open, start)) => {
                if quotes_match(open, quote)
                    && chars.get(index + 1).is_some_and(|ch| is_separator(*ch))
                {
                    bounds.extend([start, index]);
                    opening = None;
                }
            }
        }
    }
    bounds
}

fn is_unicode_word(ch: char) -> bool {
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
