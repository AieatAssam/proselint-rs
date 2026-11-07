use serde_json::{Value, json};

// The pinned check's Fancy pattern is `(?<![A-Za-z'-])(\w+(?:\s+\w+){0,3})(\s+\1)+(?![A-Za-z'-])`.
// Upstream credits write-good: https://github.com/btford/write-good, at Proselint revision dbed789caae662d06c7c8a5a13dd31f1acd36f5c.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded: Vec<char> = format!("\n{text}\n").chars().collect();
    let quote_bounds = crate::paired_quote_bounds(&padded.iter().collect::<String>());
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let mut search_from = 0;

    while search_from < padded.len() {
        let Some((start, end)) = find_match(&padded, search_from) else {
            break;
        };
        let matched: String = padded[start..end].iter().collect();
        let surface = matched.trim();
        if !is_exception(surface) {
            let quote_index = quote_bounds.partition_point(|bound| *bound < start);
            if quote_index % 2 == 0 {
                let (line, column) = line_col_of(start, &line_bounds);
                diagnostics.push(json!({
                    "check_path": "lexical_illusions",
                    "message": format!("There's a lexical illusion in '{surface}' - a phrase is repeated."),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": null,
                }));
            }
        }
        search_from = end;
    }

    diagnostics
}

fn find_match(text: &[char], from: usize) -> Option<(usize, usize)> {
    for start in from..text.len() {
        if start > 0 && is_strict_boundary_char(text[start - 1]) {
            continue;
        }
        let mut word_ends = Vec::with_capacity(4);
        let Some(first_end) = word_end(text, start) else {
            continue;
        };
        word_ends.push(first_end);

        // The bounded phrase quantifier is greedy. Try its longest capture first,
        // preserving the backtracking order of the upstream expression.
        let mut cursor = first_end;
        while word_ends.len() < 4 {
            let ws_end = whitespace_end(text, cursor);
            if ws_end == cursor {
                break;
            }
            let Some(next_end) = word_end(text, ws_end) else {
                break;
            };
            word_ends.push(next_end);
            cursor = next_end;
        }

        for phrase_words in (1..=word_ends.len()).rev() {
            let phrase_end = word_ends[phrase_words - 1];
            let phrase = &text[start..phrase_end];
            if let Some(end) = repeated_phrase_end(text, phrase_end, phrase)
                && !text.get(end).is_some_and(|ch| is_strict_boundary_char(*ch))
            {
                return Some((start, end));
            }
        }
    }
    None
}

fn word_end(text: &[char], start: usize) -> Option<usize> {
    if !text
        .get(start)
        .is_some_and(|ch| crate::rules::weasel_words_very::is_python_word(*ch))
    {
        return None;
    }
    let mut end = start + 1;
    while text
        .get(end)
        .is_some_and(|ch| crate::rules::weasel_words_very::is_python_word(*ch))
    {
        end += 1;
    }
    Some(end)
}

fn whitespace_end(text: &[char], start: usize) -> usize {
    let mut end = start;
    while text.get(end).is_some_and(|ch| is_python_whitespace(*ch)) {
        end += 1;
    }
    end
}

fn repeated_phrase_end(text: &[char], phrase_end: usize, phrase: &[char]) -> Option<usize> {
    let mut cursor = phrase_end;
    let mut best = None;
    loop {
        let whitespace = whitespace_end(text, cursor);
        if whitespace == cursor {
            break;
        }
        let Some(after_phrase) = case_insensitive_phrase_end(text, whitespace, phrase) else {
            break;
        };
        cursor = after_phrase;
        if !text
            .get(cursor)
            .is_some_and(|ch| is_strict_boundary_char(*ch))
        {
            best = Some(cursor);
        }
    }
    best
}

fn case_insensitive_phrase_end(text: &[char], start: usize, phrase: &[char]) -> Option<usize> {
    let candidate = text.get(start..start + phrase.len())?;
    candidate
        .iter()
        .zip(phrase)
        .all(|(left, right)| simple_lower(*left) == simple_lower(*right))
        .then_some(start + phrase.len())
}

fn simple_lower(ch: char) -> char {
    match ch {
        // CPython's Unicode IGNORECASE uses these simple mappings; Rust's full
        // lowercase iterator expands U+0130, while Python's regex mapping does not.
        '\u{0130}' => 'i',
        _ => {
            let mut lower = ch.to_lowercase();
            let first = lower.next().unwrap_or(ch);
            if lower.next().is_none() { first } else { ch }
        }
    }
}

fn is_python_whitespace(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\u{001c}'..='\u{001f}')
}

fn is_strict_boundary_char(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(
            ch,
            '\'' | '-' | '\u{0130}' | '\u{0131}' | '\u{017f}' | '\u{212a}'
        )
}

fn is_exception(surface: &str) -> bool {
    ["had had", "that that"].iter().any(|exception| {
        let trimmed: Vec<char> = trim_python_whitespace(surface.chars()).collect();
        trimmed.len() == exception.chars().count()
            && trimmed
                .iter()
                .copied()
                .zip(exception.chars())
                .all(|(left, right)| simple_lower(left) == simple_lower(right))
    })
}

fn trim_python_whitespace(chars: impl Iterator<Item = char>) -> impl Iterator<Item = char> {
    let chars: Vec<char> = chars.collect();
    let start = chars
        .iter()
        .position(|ch| !is_python_whitespace(*ch))
        .unwrap_or(chars.len());
    let end = chars
        .iter()
        .rposition(|ch| !is_python_whitespace(*ch))
        .map_or(start, |index| index + 1);
    chars.into_iter().skip(start).take(end - start)
}

fn splitlines_bounds(text: &[char]) -> Vec<usize> {
    let mut bounds = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let end = match text[index] {
            '\r' if text.get(index + 1) == Some(&'\n') => index + 2,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_ascii_boundary_includes_unicode_ignorecase_letters() {
        assert!(is_strict_boundary_char('\u{017f}'));
        assert!(is_strict_boundary_char('\u{212a}'));
        assert!(!is_strict_boundary_char('é'));
    }

    #[test]
    fn unicode_words_and_python_whitespace_are_recognized() {
        assert!(crate::rules::weasel_words_very::is_python_word('é'));
        assert!(!crate::rules::weasel_words_very::is_python_word('\u{0301}'));
        assert!(is_python_whitespace('\u{001c}'));
    }
}
