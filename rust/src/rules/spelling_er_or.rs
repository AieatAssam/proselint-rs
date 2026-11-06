// Data and message are copied verbatim from pinned checks/spelling/er_or.py
// (Proselint 0.16.0, dbed789caae662d06c7c8a5a13dd31f1acd36f5c).
const FORMS: [(&str, &str); 24] = [
    ("abducter", "abductor"),
    ("abbeter", "abettor"),
    ("acquiror", "acquirer"),
    ("adaptor", "adapter"),
    ("collecter", "collector"),
    ("conjuror", "conjurer"),
    ("corruptor", "corrupter"),
    ("digestor", "digester"),
    ("dispensor", "dispenser"),
    ("distributer", "distributor"),
    ("endorsor", "endorser"),
    ("erasor", "eraser"),
    ("idolator", "idolater"),
    ("imposter", "impostor"),
    ("infiltrater", "infiltrator"),
    ("invester", "investor"),
    ("manipulater", "manipulator"),
    ("mortgager", "mortgagor"),
    ("persecuter", "persecutor"),
    ("promotor", "promoter"),
    ("purveyer", "purveyor"),
    ("requestor", "requester"),
    ("revisor", "reviser"),
    ("surveyer", "surveyor"),
];

/// Return diagnostics from pinned `spelling.er_or`.
pub fn diagnostics(text: &str) -> Vec<serde_json::Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quotes = paired_quote_bounds(&chars);
    let lines = line_bounds(&chars);
    let mut active = Vec::new();
    for (misspelling, preferred) in FORMS {
        let target: Vec<char> = misspelling.chars().collect();
        if let Some(first) = (0..chars.len()).find(|start| {
            match_end(&chars, *start, &target).is_some_and(|end| {
                !is_ascii_word_before(&chars, *start) && !is_ascii_word_at(&chars, end)
            })
        }) {
            active.push((first, misspelling, preferred));
        }
    }
    active.sort_by_key(|(first, _, _)| *first);

    let mut diagnostics = Vec::new();
    for (_, misspelling, preferred) in active {
        let target: Vec<char> = misspelling.chars().collect();
        for start in 0..chars.len() {
            let Some(end) = match_end(&chars, start, &target) else {
                continue;
            };
            let span_start = start;
            let span_end = end;
            let quote_index = quotes.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col(span_start, &lines);
            let surface_lower: String = chars[start..end]
                .iter()
                .flat_map(|ch| ch.to_lowercase())
                .collect();
            let replacement = (surface_lower == misspelling).then_some(preferred);
            diagnostics.push(serde_json::json!({
                "check_path": "spelling.er_or",
                "message": format!("-er vs. -or. '{}' is the preferred spelling.", replacement.unwrap_or("None")),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": replacement,
            }));
        }
    }

    diagnostics
}

fn match_end(chars: &[char], start: usize, target: &[char]) -> Option<usize> {
    let mut folded = Vec::new();
    for (offset, ch) in chars.iter().enumerate().skip(start) {
        folded.extend(case_fold(*ch));
        if folded.len() >= target.len() {
            return (folded == target).then_some(offset + 1);
        }
        if !target.starts_with(&folded) {
            return None;
        }
    }
    None
}

fn case_fold(ch: char) -> Vec<char> {
    match ch {
        '\u{017f}' => vec!['s'],
        '\u{212a}' => vec!['k'],
        _ => ch.to_lowercase().collect(),
    }
}

fn is_ascii_word_before(chars: &[char], index: usize) -> bool {
    index > 0 && is_ascii_word(chars[index - 1])
}

fn is_ascii_word_at(chars: &[char], index: usize) -> bool {
    chars.get(index).is_some_and(|ch| is_ascii_word(*ch))
}

fn is_ascii_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn line_bounds(chars: &[char]) -> Vec<usize> {
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

fn line_col(position: usize, bounds: &[usize]) -> (usize, usize) {
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
        if !matches!(quote, '"' | '\'' | '“' | '”' | '‘' | '’') {
            continue;
        }
        let prev_word = index > 0 && is_unicode_word(chars[index - 1]);
        let next_word = chars.get(index + 1).is_some_and(|ch| is_unicode_word(*ch));
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

fn is_separator(ch: char) -> bool {
    " ,.!?:;-\r\n".contains(ch)
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
