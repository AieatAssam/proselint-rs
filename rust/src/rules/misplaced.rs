use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const CHECK_PATH: &str = "typography.punctuation.misplaced";
const MESSAGE: &str = "Misplaced punctuation. It's 'et al.'";
static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?-u:\bet\. al\b|\bet\. al\.\b)")
        .unicode(false)
        .build()
        .expect("valid pinned misplaced punctuation pattern")
});

/// Return diagnostics for the pinned `typography.punctuation.misplaced` rule.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quotes = paired_quote_bounds(&chars);
    PATTERN
        .find_iter(&padded)
        .filter_map(|matched| {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                return None;
            }
            let end = start + matched.as_str().chars().count();
            let line_start = chars[..start]
                .iter()
                .rposition(|ch| *ch == '\n')
                .map_or(0, |i| i + 1);
            let line = chars[..start].iter().filter(|ch| **ch == '\n').count();
            Some(json!({
                "check_path": CHECK_PATH,
                "message": MESSAGE,
                "span": [start, end],
                "pos": [line, start - line_start + 1],
                "replacements": Value::Null,
            }))
        })
        .collect()
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
    ch.is_alphanumeric() || ch == '_'
}

fn quotes_match(open: char, close: char) -> bool {
    (matches!(open, '\'' | '"') && open == close)
        || matches!(
            (open, close),
            ('“', '”') | ('”', '“') | ('‘', '’') | ('’', '‘')
        )
}
