use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

const PATTERN: &str = r"\w*(?:!|\?){2,}";
const MESSAGE: &str = "'{}' is hyperbolic.";
static RAW: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(PATTERN)
        .unicode(false)
        .build()
        .expect("valid hyperbole pattern")
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quotes = paired_quote_bounds(&chars);
    RAW.find_iter(&padded)
        .filter_map(|matched| {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                return None;
            }
            let end = start + matched.as_str().chars().count();
            let mut bounds = Vec::new();
            for (index, ch) in chars.iter().enumerate() {
                if matches!(ch, '\r' | '\n') {
                    bounds.push(index);
                }
            }
            let line = bounds.partition_point(|bound| *bound < start);
            let column = start - bounds[line - 1];
            Some(json!({
                "check_path": "typography.punctuation.hyperbole",
                "message": MESSAGE.replace("{}", matched.as_str()),
                "pos": [line, column],
                "replacements": null,
                "span": [start, end],
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
                opening = Some((quote, index));
            }
            None => {}
            Some((open, start))
                if (open == quote || matches!((open, quote), ('“', '”') | ('‘', '’')))
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

fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}
