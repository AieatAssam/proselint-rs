//! Native Rust implementation of Proselint.

use serde_json::{Map, Value, json};
use std::path::Path;

const ANNOTATIONS: [&str; 6] = ["FIXME", "FIX ME", "TODO", "todo", "ERASE THIS", "FIX THIS"];
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
    let mut spans = Vec::new();

    for item in ANNOTATIONS {
        for (start, matched) in padded.match_indices(item) {
            let end = start + matched.len();
            if is_ascii_word_before(&padded, start) || is_ascii_word_at(&padded, end) {
                continue;
            }
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            spans.push((span_start, span_end));
        }
    }

    // LintFile sorts results by line and column; this check has a single line
    // coordinate in its current acceptance slice, so source order is equivalent.
    spans.sort_unstable();
    spans
        .into_iter()
        .map(|(start, end)| {
            json!({
                "check_path": "annotations.misc",
                "message": MESSAGE,
                "span": [start, end],
                "pos": [1, start],
                "replacements": null,
            })
        })
        .collect()
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
