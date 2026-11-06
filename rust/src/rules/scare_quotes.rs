use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?i)(?-u:\b)the 'take-home message'(?-u:\B)")
        .build()
        .expect("valid pinned scare quotes pattern")
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quoted = paired_quote_bounds(&chars);
    let mut output = Vec::new();
    for matched in PATTERN.find_iter(&padded) {
        let start = padded[..matched.start()].chars().count();
        if quoted.partition_point(|bound| *bound < start) % 2 == 1 {
            continue;
        }
        let line_start = chars[..start]
            .iter()
            .rposition(|ch| *ch == '\n')
            .unwrap_or(0);
        let line = chars[..start].iter().filter(|ch| **ch == '\n').count();
        output.push(json!({
            "check_path": "misc.scare_quotes",
            "message": "Misuse of 'scare quotes'. Delete them.",
            "span": [start, start + matched.as_str().chars().count()],
            "pos": [line, start - line_start],
            "replacements": Value::Null,
        }));
    }
    output
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
        let prev_word = index > 0 && is_word(chars[index - 1]);
        let next_word = chars.get(index + 1).is_some_and(|ch| is_word(*ch));
        if prev_word && next_word {
            continue;
        }
        match opening {
            None if index > 0 && SEPARATORS.contains(chars[index - 1]) => {
                opening = Some((quote, index))
            }
            Some((open, start))
                if quotes_match(open, quote)
                    && chars
                        .get(index + 1)
                        .is_some_and(|ch| SEPARATORS.contains(*ch)) =>
            {
                bounds.extend([start, index]);
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

fn quotes_match(open: char, close: char) -> bool {
    (matches!(open, '\'' | '"') && open == close)
        || matches!(
            (open, close),
            ('“', '”') | ('”', '“') | ('‘', '’') | ('’', '‘')
        )
}
