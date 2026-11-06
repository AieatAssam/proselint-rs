use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?-u:\w!)")
        .build()
        .expect("valid pinned exclamation pattern")
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let padded_len = padded.chars().count();
    let required = ((30 * padded_len.max(1_000)).div_ceil(1_000_000).max(2)) as usize;
    let Some(selected) = PATTERN.find_iter(&padded).nth(required - 1) else {
        return Vec::new();
    };
    let start = padded[..selected.start()].chars().count();
    let end = start + selected.as_str().chars().count();
    let chars: Vec<char> = padded.chars().collect();
    if inside_quotes(&chars, start) {
        return Vec::new();
    }
    let line = chars[..start].iter().filter(|ch| **ch == '\n').count();
    let line_start = chars[..start]
        .iter()
        .rposition(|ch| *ch == '\n')
        .unwrap_or(0);
    vec![json!({
        "check_path": "typography.punctuation.exclamation",
        "message": "More than 30 ppm of exclamations. Keep them under control. Surpassed 30 ppm.",
        "span": [start, end],
        "pos": [line, start - line_start],
        "replacements": Value::Null,
    })]
}

fn inside_quotes(chars: &[char], position: usize) -> bool {
    let mut opening: Option<(char, usize)> = None;
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
            None if index > 0 && is_separator(chars[index - 1]) => opening = Some((quote, index)),
            Some((open, start))
                if quotes_match(open, quote)
                    && chars.get(index + 1).is_some_and(|ch| is_separator(*ch)) =>
            {
                if position > start && position < index {
                    return true;
                }
                opening = None;
            }
            _ => {}
        }
    }
    false
}

fn is_word(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
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
