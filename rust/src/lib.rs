//! Native Rust implementation of Proselint.

use serde_json::{Map, Value, json};
use std::path::Path;

const ANNOTATIONS: [&str; 6] = ["FIXME", "FIX ME", "TODO", "todo", "ERASE THIS", "FIX THIS"];
const CLICHE_HELL_PATTERN: &str = "all hell broke loose";
const CLICHE_HELL_MESSAGE: &str = "Never use the phrase 'all hell broke loose'.";
const APOLOGIZING_PATTERN: &str = "More research is needed";
const APOLOGIZING_MESSAGE: &str = "Excessive apologizing.";
const MESSAGE: &str = "Annotation left in text.";
const QUOTES: [char; 6] = ['"', '\'', '“', '”', '‘', '’'];
const SEPARATORS: &str = " ,.!?:;-\r\n";

/// Format an absolute local path as a `file:` URI, escaping bytes as Python's
/// `Path.as_uri()` does for the ASCII paths supported by the CLI.
pub fn file_uri(path: &Path) -> Result<String, std::io::Error> {
    let absolute = path.canonicalize()?;
    let bytes = absolute.to_string_lossy();
    let mut uri = String::from("file://");
    for byte in bytes.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(byte) {
            uri.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            write!(&mut uri, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    Ok(uri)
}

/// Construct the JSON result envelope for a source with no diagnostics.
pub fn clean_json_result(source: &str) -> Value {
    let mut source_results = Map::new();
    source_results.insert(source.to_owned(), json!({ "diagnostics": [] }));
    json!({ "result": source_results })
}

/// Construct a JSON result envelope containing the supplied diagnostics.
pub fn json_result(source: &str, diagnostics: Vec<Value>) -> Value {
    let mut source_results = Map::new();
    source_results.insert(source.to_owned(), json!({ "diagnostics": diagnostics }));
    json!({ "result": source_results })
}

/// Return annotation diagnostics using the pinned check's case-sensitive,
/// ASCII word-boundary and quote-suppression behavior.
pub fn annotation_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut spans = Vec::new();

    for item in ANNOTATIONS {
        for (start, matched) in padded.match_indices(item) {
            let end = start + matched.len();
            if is_ascii_word_before(&padded, start) || is_ascii_word_at(&padded, end) {
                continue;
            }
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let position = line_col_of(span_start, &line_bounds);
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            spans.push((span_start, span_end, position));
        }
    }

    // LintFile sorts results by line and column; this check has a single line
    // coordinate in its current acceptance slice, so source order is equivalent.
    spans.sort_unstable();
    spans
        .into_iter()
        .map(|(start, end, (line, column))| {
            json!({
                "check_path": "annotations.misc",
                "message": MESSAGE,
                "span": [start, end],
                "pos": [line, column],
                "replacements": null,
            })
        })
        .collect()
}

/// Return diagnostics for the pinned `cliches.hell` literal rule.
///
/// This ExistenceSimple pattern is case-insensitive and unanchored, so it
/// matches the literal phrase inside larger tokens. The default quote policy
/// suppresses matches inside paired quotes.
pub fn cliches_hell_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = CLICHE_HELL_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(CLICHE_HELL_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + CLICHE_HELL_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "cliches.hell",
            "message": CLICHE_HELL_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.apologizing` literal rule.
///
/// The Fast matcher is case-insensitive and unanchored. This check uses raw
/// matching without word-boundary padding and suppresses matches in quotes.
pub fn apologizing_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = APOLOGIZING_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(APOLOGIZING_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + APOLOGIZING_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.apologizing",
            "message": APOLOGIZING_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Match Python's `splitlines(keepends=True)` line boundaries and the
/// corresponding `LintFile.line_col_of` zero-based padded-text positions.
fn splitlines_bounds(text: &str) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut bounds = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let end = match chars[index] {
            '\r' if chars.get(index + 1) == Some(&'\n') => {
                index += 1;
                index + 1
            }
            '\r' | '\n' | '\u{000B}' | '\u{000C}' | '\u{001C}' | '\u{001D}' | '\u{001E}'
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

/// Python `Path.read_text()` enables universal newline decoding. Stdin follows
/// a separate path and retains CRLF code points in its spans.
pub fn normalize_file_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn is_ascii_word_before(text: &str, byte_index: usize) -> bool {
    byte_index > 0 && is_ascii_word(text.as_bytes()[byte_index - 1])
}

fn is_ascii_word_at(text: &str, byte_index: usize) -> bool {
    text.as_bytes()
        .get(byte_index)
        .copied()
        .is_some_and(is_ascii_word)
}

fn is_ascii_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn paired_quote_bounds(text: &str) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut opening: Option<(char, usize)> = None;
    let mut bounds = Vec::new();

    for (index, quote) in chars.iter().copied().enumerate() {
        if !QUOTES.contains(&quote) {
            continue;
        }
        let prev_word = index > 0 && is_unicode_word(chars[index - 1]);
        let next_word = chars.get(index + 1).is_some_and(|ch| is_unicode_word(*ch));
        // Python's quote detector uses Unicode `\B` around quote characters.
        if prev_word && next_word {
            continue;
        }

        match opening {
            None => {
                if index > 0 && SEPARATORS.contains(chars[index - 1]) {
                    opening = Some((quote, index));
                }
            }
            Some((open_quote, open_index)) => {
                if quotes_match(open_quote, quote)
                    && chars
                        .get(index + 1)
                        .is_some_and(|ch| SEPARATORS.contains(*ch))
                {
                    bounds.extend([open_index, index]);
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

fn quotes_match(open: char, close: char) -> bool {
    (matches!(open, '\'' | '"') && open == close)
        || (matches!(
            (open, close),
            ('“', '”') | ('”', '“') | ('‘', '’') | ('’', '‘')
        ))
}
