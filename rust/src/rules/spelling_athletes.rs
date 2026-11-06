use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Copied in declaration order from proselint 0.16.0 at dbed789caae662d06c7c8a5a13dd31f1acd36f5c.
const FORMS: [(&str, &str); 17] = [
    ("dwayne wade", "Dwyane Wade"),
    ("mikka kiprusoff", "Miikka Kiprusoff"),
    ("mark buerhle", "Mark Buehrle"),
    ("skyler diggins", "Skylar Diggins"),
    ("agnieska radwanska", "Agnieszka Radwanska"),
    (r"j\.j\. reddick", r"J\.J\. Redick"),
    ("manny packquaio", "Manny Pacquiao"),
    ("antwan jamison", "Antawn Jamison"),
    ("cal ripkin", "Cal Ripken"),
    ("johnny peralta", "Jhonny Peralta"),
    ("monte ellis", "Monta Ellis"),
    ("alex rodriquez", "Alex Rodriguez"),
    ("mark texeira", "Mark Teixeira"),
    ("brett farve", "Brett Favre"),
    ("tori hunter", "Torii Hunter"),
    ("stephon curry", "Stephen Curry"),
    ("mike kryzewski", "Mike Krzyzewski"),
];

static ACTIVATION: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(form, _)| {
            RegexBuilder::new(&format!(r"(?i)(?-u:\b){form}(?-u:\b)"))
                .build()
                .expect("valid pinned spelling.athletes form")
        })
        .collect()
});

static RAW: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(form, _)| {
            RegexBuilder::new(&format!(r"(?i){form}"))
                .build()
                .expect("valid pinned spelling.athletes form")
        })
        .collect()
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quote_bounds = paired_quote_bounds(&chars);
    let lines = splitlines_bounds(&chars);
    let mut active = ACTIVATION
        .iter()
        .enumerate()
        .filter_map(|(index, pattern)| pattern.find(&padded).map(|m| (m.start(), index)))
        .collect::<Vec<_>>();
    active.sort_unstable();

    let mut diagnostics = Vec::new();
    for (_, index) in active {
        let (original, replacement) = FORMS[index];
        for matched in RAW[index].find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let surface = matched.as_str();
            let end = start + surface.chars().count();
            let preferred = (surface.to_lowercase() == original).then_some(replacement);
            let (line, column) = line_col_of(start, &lines);
            diagnostics.push(json!({
                "check_path": "spelling.athletes",
                "message": format!("Misspelling of athlete's name. '{}' is the preferred form.", preferred.unwrap_or("None")),
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
