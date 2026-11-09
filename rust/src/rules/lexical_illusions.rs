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

/// Match stdin decoded with Python's surrogateescape convention. Invalid bytes
/// occupy one code point and therefore break words and repeated phrases.
pub fn diagnostics_codepoints(text: &[u32]) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut padded = Vec::with_capacity(text.len() + 2);
    padded.push('\n' as u32);
    padded.extend_from_slice(text);
    padded.push('\n' as u32);
    let quote_bounds = quote_bounds_codepoints(&padded);
    let line_bounds = splitlines_bounds_codepoints(&padded);
    let mut diagnostics = Vec::new();
    let mut search_from = 0;
    while search_from < padded.len() {
        let Some((start, end)) = find_match_codepoints(&padded, search_from) else {
            break;
        };
        let surface: String = padded[start..end]
            .iter()
            .filter_map(|&cp| char::from_u32(cp))
            .collect();
        let surface = surface.trim();
        if !is_exception(surface) {
            let quote_index = quote_bounds.partition_point(|bound| *bound < start);
            if quote_index % 2 == 0 {
                let (line, column) = line_col_of(start, &line_bounds);
                diagnostics.push(json!({"check_path":"lexical_illusions","message":format!("There's a lexical illusion in '{surface}' - a phrase is repeated."),"span":[start,end],"pos":[line,column],"replacements":null}));
            }
        }
        search_from = end;
    }
    diagnostics
}

fn find_match_codepoints(text: &[u32], from: usize) -> Option<(usize, usize)> {
    for start in from..text.len() {
        if start > 0 && is_strict_boundary_cp(text[start - 1]) {
            continue;
        }
        let mut ends = Vec::with_capacity(4);
        let Some(first) = word_end_cp(text, start) else {
            continue;
        };
        ends.push(first);
        let mut cursor = first;
        while ends.len() < 4 {
            let ws = whitespace_end_cp(text, cursor);
            if ws == cursor {
                break;
            }
            let Some(end) = word_end_cp(text, ws) else {
                break;
            };
            ends.push(end);
            cursor = end;
        }
        for count in (1..=ends.len()).rev() {
            let phrase_end = ends[count - 1];
            if let Some(end) = repeated_phrase_end_cp(text, phrase_end, &text[start..phrase_end])
                && !text.get(end).is_some_and(|&cp| is_strict_boundary_cp(cp))
            {
                return Some((start, end));
            }
        }
    }
    None
}
fn word_end_cp(text: &[u32], start: usize) -> Option<usize> {
    let is_word =
        |cp: u32| char::from_u32(cp).is_some_and(crate::rules::weasel_words_very::is_python_word);
    if !text.get(start).is_some_and(|&cp| is_word(cp)) {
        return None;
    }
    let mut end = start + 1;
    while text.get(end).is_some_and(|&cp| is_word(cp)) {
        end += 1;
    }
    Some(end)
}
fn whitespace_end_cp(text: &[u32], start: usize) -> usize {
    let mut end = start;
    while text
        .get(end)
        .is_some_and(|&cp| char::from_u32(cp).is_some_and(is_python_whitespace))
    {
        end += 1;
    }
    end
}
fn repeated_phrase_end_cp(text: &[u32], phrase_end: usize, phrase: &[u32]) -> Option<usize> {
    let mut cursor = phrase_end;
    let mut best = None;
    loop {
        let ws = whitespace_end_cp(text, cursor);
        if ws == cursor {
            break;
        }
        let Some(end) = case_insensitive_phrase_end_cp(text, ws, phrase) else {
            break;
        };
        cursor = end;
        if !text
            .get(cursor)
            .is_some_and(|&cp| is_strict_boundary_cp(cp))
        {
            best = Some(cursor);
        }
    }
    best
}
fn case_insensitive_phrase_end_cp(text: &[u32], start: usize, phrase: &[u32]) -> Option<usize> {
    let candidate = text.get(start..start + phrase.len())?;
    candidate
        .iter()
        .zip(phrase)
        .all(|(&a, &b)| {
            a == b
                || char::from_u32(a)
                    .zip(char::from_u32(b))
                    .is_some_and(|(a, b)| simple_lower(a) == simple_lower(b))
        })
        .then_some(start + phrase.len())
}
fn is_strict_boundary_cp(cp: u32) -> bool {
    char::from_u32(cp).is_some_and(is_strict_boundary_char)
}
fn quote_bounds_codepoints(text: &[u32]) -> Vec<usize> {
    let mut opening: Option<(char, usize)> = None;
    let mut bounds = Vec::new();
    for (i, &cp) in text.iter().enumerate() {
        let Some(q) = char::from_u32(cp) else {
            continue;
        };
        if !crate::QUOTES.contains(&q) {
            continue;
        }
        let word = |at: usize| {
            text.get(at)
                .and_then(|&v| char::from_u32(v))
                .is_some_and(|c| c == '_' || c.is_alphanumeric())
        };
        if i > 0 && word(i - 1) && word(i + 1) {
            continue;
        }
        match opening {
            None => {
                if i > 0
                    && text
                        .get(i - 1)
                        .and_then(|&v| char::from_u32(v))
                        .is_some_and(|c| crate::SEPARATORS.contains(c))
                {
                    opening = Some((q, i));
                }
            }
            Some((open, oi)) => {
                if crate::quotes_match(open, q)
                    && text
                        .get(i + 1)
                        .and_then(|&v| char::from_u32(v))
                        .is_some_and(|c| crate::SEPARATORS.contains(c))
                {
                    bounds.extend([oi, i]);
                    opening = None;
                }
            }
        }
    }
    bounds
}
fn splitlines_bounds_codepoints(text: &[u32]) -> Vec<usize> {
    let mut bounds = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let end = match text[i] {
            13 if text.get(i + 1) == Some(&10) => i + 2,
            13 | 10 | 11 | 12 | 28 | 29 | 30 | 0x85 | 0x2028 | 0x2029 => i + 1,
            _ => {
                i += 1;
                continue;
            }
        };
        bounds.push(end - 1);
        i = end;
    }
    bounds
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
