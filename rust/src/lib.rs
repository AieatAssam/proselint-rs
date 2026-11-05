//! Native Rust implementation of Proselint.

use serde_json::{Map, Value, json};
use std::path::Path;

const ANNOTATIONS: [&str; 6] = ["FIXME", "FIX ME", "TODO", "todo", "ERASE THIS", "FIX THIS"];
const CLICHE_HELL_PATTERN: &str = "all hell broke loose";
const CLICHE_HELL_MESSAGE: &str = "Never use the phrase 'all hell broke loose'.";
const APOLOGIZING_PATTERN: &str = "More research is needed";
const APOLOGIZING_MESSAGE: &str = "Excessive apologizing.";
const WHENCE_PATTERN: &str = "from whence";
const WHENCE_MESSAGE: &str = "The 'from' in 'from whence' is not needed.";
const SUDDENLY_PATTERN: &str = "Suddenly,";
const SUDDENLY_MESSAGE: &str = "Suddenly is nondescript, slows the action, and warns your reader.";
const BUT_PATTERN: &str = "But";
const BUT_MESSAGE: &str = "No paragraph should start with a 'But'.";
const AM_PM_MESSAGE: &str = "It's standard to use periods for lowercase 'a.m.' or 'p.m.'.";
const AM_PM_SPACING_MESSAGE: &str = "It's standard to put a space before 'a.m.' or 'p.m.'.";
const DEBASED_ITEMS: [&str; 4] = [
    "a not unjustifiable assumption",
    "leaves much to be desired",
    "would serve no purpose",
    "a consideration which we should do well to bear in mind",
];
const DEBASED_MESSAGE: &str = "Debased language is a continuous temptation.";
const CURRENCY_MESSAGE_PREFIX: &str = "Incorrect use of symbols in ";
const INFERIOR_SUPERIOR: [(&str, &str); 2] = [
    ("inferior than", "inferior to"),
    ("superior than", "superior to"),
];
const INFERIOR_SUPERIOR_MESSAGE: &str =
    "'Inferior' and 'superior' are not true comparatives. Use '{}'.";
const KUDOS_PATTERN: &str = "many kudos";
const KUDOS_MESSAGE: &str = "Kudos is singular.";
const FALSE_PLURALS_MISC: [(&str, &str); 2] =
    [("talismen", "talismans"), ("phenomenons", "phenomena")];
const TYPOGRAPHY_ELLIPSIS_PATTERN: &str = "...";
const TYPOGRAPHY_ELLIPSIS_MESSAGE: &str = "'...' is an approximation, use the ellipsis symbol '…'.";
const TYPOGRAPHY_COPYRIGHT_PATTERN: &str = "(c)";
const TYPOGRAPHY_COPYRIGHT_MESSAGE: &str =
    "(c) is a goofy alphabetic approximation, use the symbol ©.";
const TYPOGRAPHY_SENTENCE_SPACING_PATTERN: &str = ".   ";
const TYPOGRAPHY_SENTENCE_SPACING_MESSAGE: &str =
    "More than two spaces after the period; use 1 or 2.";
const TYPOGRAPHY_TRADEMARKS: [(&str, &str); 2] = [
    (
        "(tm)",
        "(TM) is a goofy alphabetic approximation, use the symbol ™.",
    ),
    (
        "(r)",
        "(R) is a goofy alphabetic approximation, use the symbol ®.",
    ),
];
const TYPOGRAPHY_MULTIPLICATION_MESSAGE: &str =
    "Use the multiplication symbol ×, not the letter x.";
const MENTAL_TELEPATHY_PATTERN: &str = "mental telepathy";
const MENTAL_TELEPATHY_MESSAGE: &str =
    "This is redundant because all purported telepathy is mental.";
const ILLOGIC_COIN_PATTERN: &str = "to coin a phrase from";
const ILLOGIC_COIN_MESSAGE: &str = "You can't coin an existing phrase. Did you mean 'borrow'?";
const ILLOGIC_COLLUSION_PATTERN: &str = "without your collusion";
const ILLOGIC_COLLUSION_MESSAGE: &str = "It's impossible to defraud yourself. Try 'aquiescence'.";
const BACK_FORMATIONS_PATTERN: &str = "improprietous";
const BACK_FORMATIONS_REPLACEMENT: &str = "improper";
const BACK_FORMATIONS_MESSAGE: &str = "Back-formation. 'improper' is the preferred form.";
const GREYLIST_TERMS: [(&str, &str); 2] = [
    ("obviously", "This is obviously an inadvisable word to use."),
    (
        "utilize",
        "Do you know anyone who needs to utilize the word utilize?",
    ),
];
const BUREAUCRATESE_PREFIX: &str = "meet";
const BUREAUCRATESE_SUFFIX: &str = " with your approval";
const BUREAUCRATESE_MESSAGE: &str = "'{}' is bureaucratese.";
const HEDGING_PATTERNS: [&str; 3] = ["I would argue that", ", so to speak", "to a certain degree"];
const HEDGING_MESSAGE: &str = "Hedging. Just say it.";
const JARGON_PATTERNS: [&str; 6] = [
    "in the affirmative",
    "in the negative",
    "agendize",
    "per your order",
    "per your request",
    "disincentivize",
];
const JARGON_MESSAGE: &str = "'{}' is jargon. Can you replace it with something more standard?";
const AIRLINESE_PREFIXES: [&str; 2] = ["enplan", "deplan"];
const AIRLINESE_SUFFIXES: [&str; 4] = ["e", "ed", "ing", "ement"];
const AIRLINESE_PHRASE: &str = "taking off momentarily";
const AIRLINESE_MESSAGE: &str = "'{}' is airlinese.";
const MANY_A_FORMS: [(&str, &str); 3] = [
    ("are many a", "is many a"),
    ("have been many a", "has been many a"),
    ("were many a", "was many a"),
];
const MANY_A_MESSAGE: &str = "'many a' requires a singular verb.";
const METADISCOURSE_PATTERNS: [&str; 5] = [
    "The preceeding discussion",
    "The rest of this article",
    "This chapter discusses",
    "The preceding paragraph demonstrated",
    "The previous section analyzed",
];
const METADISCOURSE_MESSAGE: &str = "Excessive metadiscourse.";
const PRETENSION_TERMS: [&str; 4] = [
    "reconceptualize",
    "demassification",
    "attitudinally",
    "judgmentally",
];
const PRETENSION_MESSAGE: &str = "Jargon words like '{}' are the hallmarks of a pretentious ass.";
const LATIN_FORMS: [(&str, &str); 4] = [
    ("ceteris paribus", "other things being equal"),
    ("inter alia", "among other things"),
    ("simpliciter", "in and of itself"),
    ("mutatis mutandis", "having made the necessary changes"),
];
const LATIN_MESSAGE: &str = "Use English. '{}' is the preferred form.";
const CAPITALIZATION_SEASONS: [(&str, &str); 2] = [("Winter", "winter"), ("Summer", "summer")];
const CAPITALIZATION_SEASONS_MESSAGE: &str =
    "Seasons shouldn't be capitalized. '{}' is the preferred form.";
const INSTITUTION_NAME_PATTERN: &str = "Virginia Polytechnic and State University";
const INSTITUTION_NAME_REPLACEMENT: &str = "Virginia Polytechnic Institute and State University";
const PROFESSIONS: [(&str, &str); 2] = [("shoe repair guy", "cobbler"), ("geometrist", "geometer")];
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

/// Return diagnostics for the pinned `misc.whence` literal rule.
///
/// The Fast matcher is case-insensitive and unanchored, with no word-boundary
/// padding. The default quote policy suppresses matches inside paired quotes.
pub fn whence_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = WHENCE_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(WHENCE_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + WHENCE_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.whence",
            "message": WHENCE_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.suddenly` literal rule. It is
/// case-sensitive and unanchored; occurrences inside paired quotes are
/// suppressed by the default check flags.
pub fn suddenly_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, _) in padded.match_indices(SUDDENLY_PATTERN) {
        let span_start = padded[..start].chars().count();
        let span_end = span_start + SUDDENLY_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "misc.suddenly",
                "message": SUDDENLY_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.but` paragraph-start rule.
/// A case-sensitive `But` must follow document start or a blank-line pair and
/// end at an ASCII word boundary. The upstream match includes paragraph
/// separators, then adjusts the reported span to begin at `But`.
pub fn but_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let bytes = padded.as_bytes();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, _) in padded.match_indices(BUT_PATTERN) {
        let paragraph_start =
            start == 1 || (start >= 2 && bytes.get(start - 2..start) == Some(b"\n\n"));
        let end = start + BUT_PATTERN.len();
        if !paragraph_start || is_ascii_word_at(&padded, end) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + BUT_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "misc.but",
                "message": BUT_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned lowercase AM/PM typography check.
/// This matches `\b\d{1,2} ?[ap]m\b`: ASCII digits, lowercase a/p and m,
/// optional single ASCII space, and ASCII word boundaries. Quoted matches are
/// suppressed by the default check flags.
pub fn am_pm_lowercase_periods_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let bytes = padded.as_bytes();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, _) in padded.char_indices() {
        if !bytes[start].is_ascii_digit() || is_ascii_word_before(&padded, start) {
            continue;
        }

        let mut end = start + 1;
        while end < bytes.len() && end - start < 2 && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end < bytes.len() && bytes[end].is_ascii_digit() {
            continue;
        }
        if bytes.get(end) == Some(&b' ') {
            end += 1;
        }
        if !matches!(bytes.get(end), Some(b'a' | b'p')) {
            continue;
        }
        end += 1;
        if bytes.get(end) != Some(&b'm') {
            continue;
        }
        end += 1;
        if is_ascii_word_at(&padded, end) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "dates_times.am_pm.lowercase_periods",
                "message": AM_PM_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned AM/PM spacing rule. It matches the
/// case-insensitive literal pattern `\d{1,2}[ap]\.?m\.?` without word
/// boundaries; ASCII digit runs can therefore match from a later digit when
/// an earlier two-digit candidate fails. Quoted matches are suppressed.
pub fn am_pm_spacing_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let bytes = padded.as_bytes();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_digit() {
            cursor += 1;
            continue;
        }

        let start = cursor;
        let digit_end = (start..bytes.len())
            .take_while(|index| bytes[*index].is_ascii_digit())
            .take(2)
            .last()
            .map_or(start, |index| index + 1);
        let digit_count = digit_end - start;
        let mut matched_end = None;

        for count in (1..=digit_count).rev() {
            let mut end = start + count;
            if !matches!(bytes.get(end), Some(b'a' | b'A' | b'p' | b'P')) {
                continue;
            }
            end += 1;
            if bytes.get(end) == Some(&b'.') {
                end += 1;
            }
            if !matches!(bytes.get(end), Some(b'm' | b'M')) {
                continue;
            }
            end += 1;
            if bytes.get(end) == Some(&b'.') {
                end += 1;
            }
            matched_end = Some(end);
            break;
        }

        let Some(end) = matched_end else {
            cursor = start + 1;
            continue;
        };

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 0 {
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "dates_times.am_pm.spacing",
                    "message": AM_PM_SPACING_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
        cursor = end;
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.debased` check. WORDS_IN_TEXT
/// first selects active phrases; FastSet then searches each selected raw
/// phrase throughout the padded text, so embedded copies of an active phrase
/// can also be reported. Matching is case-insensitive and quoted occurrences
/// are suppressed.
pub fn debased_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let lowered = padded.to_ascii_lowercase();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for item in DEBASED_ITEMS {
        let active = lowered.match_indices(item).any(|(start, _)| {
            let end = start + item.len();
            !is_ascii_word_before(&lowered, start) && !is_ascii_word_at(&lowered, end)
        });
        if !active {
            continue;
        }

        for (start, _) in lowered.match_indices(item) {
            let span_start = padded[..start].chars().count();
            let span_end = span_start + item.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.debased",
                    "message": DEBASED_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.currency` rule. The RE2 pattern is
/// case-insensitive, uses ASCII digit and word classes, and begins with `\B`:
/// a dollar sign is accepted after punctuation or at text start, but not after
/// an ASCII word character. Quoted matches are suppressed.
pub fn currency_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let lowered = padded.to_ascii_lowercase();
    let bytes = lowered.as_bytes();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, ch) in padded.char_indices() {
        if ch != '$' || is_ascii_word_before(&padded, start) {
            continue;
        }

        let mut suffix_start = start + 1;
        while bytes.get(suffix_start).is_some_and(u8::is_ascii_digit) {
            suffix_start += 1;
        }
        if bytes.get(suffix_start) == Some(&b' ') {
            suffix_start += 1;
        }

        let suffix = [b"dollars".as_slice(), b"usd", b"us dollars"]
            .into_iter()
            .find(|suffix| bytes.get(suffix_start..suffix_start + suffix.len()) == Some(*suffix));
        let Some(suffix) = suffix else {
            continue;
        };
        let end = suffix_start + suffix.len();
        if is_ascii_word_at(&lowered, end) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        let matched = &padded[start..end];
        matches.push((
            span_start,
            json!({
                "check_path": "misc.currency",
                "message": format!("{CURRENCY_MESSAGE_PREFIX}{matched}."),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.inferior_superior` preferred-form
/// check. It matches the two case-insensitive phrases with ASCII word
/// boundaries, reports each original span, and supplies the mapped preferred
/// phrase as both the replacement and the message interpolation.
pub fn inferior_superior_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let lowered = padded.to_ascii_lowercase();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (pattern, replacement) in INFERIOR_SUPERIOR {
        for (start, _) in lowered.match_indices(pattern) {
            let end = start + pattern.len();
            if is_ascii_word_before(&lowered, start) || is_ascii_word_at(&lowered, end) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + pattern.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.inferior_superior",
                    "message": INFERIOR_SUPERIOR_MESSAGE.replace("{}", replacement),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.false_plurals.kudos` literal rule.
///
/// The Fast matcher is case-insensitive and unanchored, with no word-boundary
/// padding. The default quote policy suppresses matches inside paired quotes.
pub fn false_plurals_kudos_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = KUDOS_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(KUDOS_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + KUDOS_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.false_plurals.kudos",
            "message": KUDOS_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.false_plurals.misc` preferred
/// forms check. Matches use case-insensitive word padding, preserve the
/// upstream table order for ties, and suppress occurrences inside quotes.
pub fn false_plurals_misc_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let lowered = padded.to_ascii_lowercase();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (pattern, replacement) in FALSE_PLURALS_MISC {
        let pattern_len = pattern.len();
        for (start, _) in lowered.match_indices(pattern) {
            let end = start + pattern_len;
            if is_ascii_word_before(&lowered, start) || is_ascii_word_at(&lowered, end) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + pattern.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.false_plurals.misc",
                    "message": format!("The correct plural is '{replacement}'."),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `typography.symbols.ellipsis` rule.
/// The ASCII three-dot pattern is an unanchored literal; matches inside paired
/// quotes are suppressed by the default check flags.
pub fn typography_ellipsis_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, _) in padded.match_indices(TYPOGRAPHY_ELLIPSIS_PATTERN) {
        let span_start = padded[..start].chars().count();
        let span_end = span_start + TYPOGRAPHY_ELLIPSIS_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "typography.symbols.ellipsis",
                "message": TYPOGRAPHY_ELLIPSIS_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `typography.symbols.copyright` rule.
/// This case-insensitive literal check is unanchored and suppresses matches
/// inside paired quotes according to the default check flags.
pub fn typography_copyright_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, _) in padded.char_indices() {
        let Some(candidate) =
            padded.get(start..start.saturating_add(TYPOGRAPHY_COPYRIGHT_PATTERN.len()))
        else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(TYPOGRAPHY_COPYRIGHT_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + TYPOGRAPHY_COPYRIGHT_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "typography.symbols.copyright",
                "message": TYPOGRAPHY_COPYRIGHT_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `typography.symbols.sentence_spacing`
/// rule. The literal is exactly a period followed by three spaces, so longer
/// runs match only that prefix. Matches inside paired quotes are suppressed.
pub fn typography_sentence_spacing_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (start, _) in padded.match_indices(TYPOGRAPHY_SENTENCE_SPACING_PATTERN) {
        let span_start = padded[..start].chars().count();
        let span_end = span_start + TYPOGRAPHY_SENTENCE_SPACING_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "typography.symbols.sentence_spacing",
                "message": TYPOGRAPHY_SENTENCE_SPACING_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the two pinned `typography.symbols.trademark`
/// registrations. Each literal matches case-insensitively without anchoring;
/// merged findings are sorted by source position and quoted matches are
/// suppressed by the default check flags.
pub fn typography_trademark_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (pattern, message) in TYPOGRAPHY_TRADEMARKS {
        for (start, _) in padded.char_indices() {
            let Some(candidate) = padded.get(start..start.saturating_add(pattern.len())) else {
                continue;
            };
            if !candidate.eq_ignore_ascii_case(pattern) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + pattern.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "typography.symbols.trademark",
                    "message": message,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned multiplication-symbol check. It matches
/// ASCII digit runs separated by `x`/`X`, allowing zero or one ASCII space on
/// either side. Matches are unanchored and nonoverlapping; quote suppression
/// follows the default check flags.
pub fn typography_multiplication_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let bytes = padded.as_bytes();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_digit() {
            cursor += 1;
            continue;
        }

        let start = cursor;
        let mut separator = start;
        while separator < bytes.len() && bytes[separator].is_ascii_digit() {
            separator += 1;
        }
        if bytes.get(separator) == Some(&b' ') {
            separator += 1;
        }
        if !matches!(bytes.get(separator), Some(b'x' | b'X')) {
            cursor = start + 1;
            continue;
        }

        let mut end = separator + 1;
        if bytes.get(end) == Some(&b' ') {
            end += 1;
        }
        let right_start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == right_start {
            cursor = start + 1;
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 0 {
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "typography.symbols.multiplication",
                    "message": TYPOGRAPHY_MULTIPLICATION_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
        cursor = end;
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `psychology.mental_telepathy` literal rule.
///
/// The Fast matcher is case-insensitive and unanchored, with no word-boundary
/// padding. The default quote policy suppresses matches inside paired quotes.
pub fn mental_telepathy_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = MENTAL_TELEPATHY_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(MENTAL_TELEPATHY_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + MENTAL_TELEPATHY_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "psychology.mental_telepathy",
            "message": MENTAL_TELEPATHY_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.illogic.coin` literal rule.
///
/// The Fast matcher is case-insensitive and unanchored, with no word-boundary
/// padding. The default quote policy suppresses matches inside paired quotes.
pub fn illogic_coin_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = ILLOGIC_COIN_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(ILLOGIC_COIN_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + ILLOGIC_COIN_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.illogic.coin",
            "message": ILLOGIC_COIN_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.illogic.collusion` literal rule.
///
/// The Fast matcher is case-insensitive and unanchored, with no word-boundary
/// padding. The default quote policy suppresses matches inside paired quotes.
pub fn illogic_collusion_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = ILLOGIC_COLLUSION_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(ILLOGIC_COLLUSION_PATTERN) {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + ILLOGIC_COLLUSION_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.illogic.collusion",
            "message": ILLOGIC_COLLUSION_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.back_formations` preferred-form rule.
///
/// PreferredFormsSimple pads the literal with WORDS_IN_TEXT boundaries, uses
/// the Fast engine with case-insensitive matching, and suppresses quoted matches.
pub fn back_formations_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = BACK_FORMATIONS_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(candidate) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(BACK_FORMATIONS_PATTERN)
            || is_ascii_word_before(&padded, start)
            || is_ascii_word_at(&padded, start + pattern_len)
        {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + BACK_FORMATIONS_PATTERN.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.back_formations",
            "message": BACK_FORMATIONS_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": BACK_FORMATIONS_REPLACEMENT,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.greylist` term family.
///
/// The custom upstream check applies Fast case-insensitive ExistenceSimple
/// matching to each term without word-boundary padding, then formats a
/// term-specific message. Quoted matches are suppressed by the check default.
pub fn greylist_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (term, explanation) in GREYLIST_TERMS {
        let term_len = term.len();
        for (start, _) in padded.char_indices() {
            let Some(candidate) = padded.get(start..start.saturating_add(term_len)) else {
                continue;
            };
            if !candidate.eq_ignore_ascii_case(term) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + term.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.greylist",
                    "message": format!("Use of '{term}'. {explanation}"),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `industrial_language.bureaucratese` rule.
///
/// Upstream performs case-insensitive raw substring matching for
/// `meets? with your approval`; findings retain the matched spelling and are
/// suppressed inside paired quotes.
pub fn bureaucratese_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();
    let prefix_len = BUREAUCRATESE_PREFIX.len();

    for (start, _) in padded.char_indices() {
        let prefix_end = start.saturating_add(prefix_len);
        let Some(prefix) = padded.get(start..prefix_end) else {
            continue;
        };
        if !prefix.eq_ignore_ascii_case(BUREAUCRATESE_PREFIX) {
            continue;
        }

        let middle = if padded
            .get(prefix_end..prefix_end.saturating_add(1))
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case("s"))
        {
            prefix_end + 1
        } else {
            prefix_end
        };
        let Some(suffix) = padded.get(middle..middle.saturating_add(BUREAUCRATESE_SUFFIX.len()))
        else {
            continue;
        };
        if !suffix.eq_ignore_ascii_case(BUREAUCRATESE_SUFFIX) {
            continue;
        }
        let end = middle + BUREAUCRATESE_SUFFIX.len();
        let matched = padded[start..end].to_owned();
        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        matches.push((
            span_start,
            json!({
                "check_path": "industrial_language.bureaucratese",
                "message": BUREAUCRATESE_MESSAGE.replace("{}", &matched),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }),
        ));
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `hedging` existence check.
///
/// Upstream's Fast set first selects patterns with WORDS_IN_TEXT boundaries,
/// then runs each selected raw pattern through its case-insensitive iterator.
/// This means an activated phrase can also report raw occurrences that do not
/// themselves meet the boundary condition.
pub fn hedging_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for pattern in HEDGING_PATTERNS {
        let pattern_len = pattern.len();
        let activated = padded.char_indices().any(|(start, _)| {
            let end = start.saturating_add(pattern_len);
            let Some(candidate) = padded.get(start..end) else {
                return false;
            };
            candidate.eq_ignore_ascii_case(pattern)
                && (is_ascii_word_before(&padded, start) != is_ascii_word_at(&padded, start))
                && (is_ascii_word_before(&padded, end) != is_ascii_word_at(&padded, end))
        });
        if !activated {
            continue;
        }

        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(pattern) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "hedging",
                    "message": HEDGING_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `industrial_language.jargon` check.
///
/// Like the other Fast `Existence` checks, this first activates each pattern
/// only when its WORDS_IN_TEXT padded form matches, then reports raw
/// case-insensitive occurrences of activated patterns.
pub fn jargon_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for pattern in JARGON_PATTERNS {
        let pattern_len = pattern.len();
        let activated = padded.char_indices().any(|(start, _)| {
            let end = start.saturating_add(pattern_len);
            let Some(candidate) = padded.get(start..end) else {
                return false;
            };
            candidate.eq_ignore_ascii_case(pattern)
                && !is_ascii_word_before(&padded, start)
                && !is_ascii_word_at(&padded, end)
        });
        if !activated {
            continue;
        }

        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(pattern) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "industrial_language.jargon",
                    "message": JARGON_MESSAGE.replace("{}", matched),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `industrial_language.airlinese` rule.
///
/// The upstream regex alternation is leftmost-first: `e` precedes `ed` and
/// `ement`, so those inflections produce the shorter `enplane`/`deplane`
/// surface match. The Fast existence set activates a pattern on a padded
/// word-boundary match before raw regex matches are emitted.
pub fn airlinese_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for prefix in AIRLINESE_PREFIXES {
        let active = padded.char_indices().any(|(start, _)| {
            AIRLINESE_SUFFIXES.iter().any(|suffix| {
                let phrase_len = prefix.len() + suffix.len();
                let end = start.saturating_add(phrase_len);
                let Some(candidate) = padded.get(start..end) else {
                    return false;
                };
                candidate.eq_ignore_ascii_case(&format!("{prefix}{suffix}"))
                    && !is_ascii_word_before(&padded, start)
                    && !is_ascii_word_at(&padded, end)
            })
        });
        if !active {
            continue;
        }

        for (start, _) in padded.char_indices() {
            let base_end = start.saturating_add(prefix.len());
            let Some(base) = padded.get(start..base_end) else {
                continue;
            };
            if !base.eq_ignore_ascii_case(prefix) {
                continue;
            }

            let matched_len = if padded
                .get(base_end..base_end.saturating_add(1))
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case("e"))
            {
                prefix.len() + 1
            } else if padded
                .get(base_end..base_end.saturating_add(3))
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case("ing"))
            {
                prefix.len() + 3
            } else {
                continue;
            };
            let end = start + matched_len;
            let matched = &padded[start..end];
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "industrial_language.airlinese",
                    "message": AIRLINESE_MESSAGE.replace("{}", matched),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    let phrase_len = AIRLINESE_PHRASE.len();
    let phrase_active = padded.char_indices().any(|(start, _)| {
        let end = start.saturating_add(phrase_len);
        let Some(candidate) = padded.get(start..end) else {
            return false;
        };
        candidate.eq_ignore_ascii_case(AIRLINESE_PHRASE)
            && !is_ascii_word_before(&padded, start)
            && !is_ascii_word_at(&padded, end)
    });
    if phrase_active {
        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(phrase_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(AIRLINESE_PHRASE) {
                continue;
            }
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "industrial_language.airlinese",
                    "message": AIRLINESE_MESSAGE.replace("{}", matched),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.many_a` preferred forms.
///
/// Matches use case-insensitive WORDS_IN_TEXT boundaries, while replacements
/// come from the upstream form mapping and the message remains static.
pub fn many_a_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for (pattern, replacement) in MANY_A_FORMS {
        let pattern_len = pattern.len();
        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(candidate) = padded.get(start..end) else {
                continue;
            };
            if !candidate.eq_ignore_ascii_case(pattern)
                || is_ascii_word_before(&padded, start)
                || is_ascii_word_at(&padded, end)
            {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + candidate.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "misc.many_a",
                    "message": MANY_A_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.metadiscourse` existence check.
///
/// Upstream uses a case-insensitive Fast set with WORDS_IN_TEXT padding,
/// including the intentional spelling `preceeding` in its first phrase.
pub fn metadiscourse_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for pattern in METADISCOURSE_PATTERNS {
        let pattern_len = pattern.len();
        let activated = padded.char_indices().any(|(start, _)| {
            let end = start.saturating_add(pattern_len);
            let Some(candidate) = padded.get(start..end) else {
                return false;
            };
            candidate.eq_ignore_ascii_case(pattern)
                && !is_ascii_word_before(&padded, start)
                && !is_ascii_word_at(&padded, end)
        });
        if !activated {
            continue;
        }

        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(pattern) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "misc.metadiscourse",
                    "message": METADISCOURSE_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.pretension` existence check.
///
/// Each term uses case-insensitive Fast existence matching with
/// WORDS_IN_TEXT activation; the matched source spelling is interpolated in
/// the message. Quoted occurrences are suppressed.
pub fn pretension_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for term in PRETENSION_TERMS {
        let term_len = term.len();
        let activated = padded.char_indices().any(|(start, _)| {
            let end = start.saturating_add(term_len);
            let Some(candidate) = padded.get(start..end) else {
                return false;
            };
            candidate.eq_ignore_ascii_case(term)
                && !is_ascii_word_before(&padded, start)
                && !is_ascii_word_at(&padded, end)
        });
        if !activated {
            continue;
        }

        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(term_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(term) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "misc.pretension",
                    "message": PRETENSION_MESSAGE.replace("{}", matched),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.latin` preferred forms.
///
/// Matching is case-insensitive with WORDS_IN_TEXT activation. Replacement
/// text is the mapped English form; matches inside paired quotes are omitted.
pub fn latin_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for (pattern, replacement) in LATIN_FORMS {
        let pattern_len = pattern.len();
        let activated = padded.char_indices().any(|(start, _)| {
            let end = start.saturating_add(pattern_len);
            let Some(candidate) = padded.get(start..end) else {
                return false;
            };
            candidate.eq_ignore_ascii_case(pattern)
                && !is_ascii_word_before(&padded, start)
                && !is_ascii_word_at(&padded, end)
        });
        if !activated {
            continue;
        }

        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(pattern) {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "misc.latin",
                    "message": LATIN_MESSAGE.replace("{}", replacement),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the case-sensitive pinned seasons capitalization
/// preferred forms. Lowercase and other-case variants do not match.
pub fn capitalization_seasons_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for (pattern, replacement) in CAPITALIZATION_SEASONS {
        let pattern_len = pattern.len();
        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if matched != pattern
                || is_ascii_word_before(&padded, start)
                || is_ascii_word_at(&padded, end)
            {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                json!({
                    "check_path": "misc.capitalization.seasons",
                    "message": CAPITALIZATION_SEASONS_MESSAGE.replace("{}", replacement),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }

    diagnostics.sort_unstable_by_key(|(start, _)| *start);
    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `misc.institution_name` preferred-form rule.
///
/// The rule is case-insensitive, uses WORDS_IN_TEXT boundaries, and suppresses
/// occurrences inside paired quotes.
pub fn institution_name_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    let pattern_len = INSTITUTION_NAME_PATTERN.len();

    for (start, _) in padded.char_indices() {
        let Some(matched) = padded.get(start..start.saturating_add(pattern_len)) else {
            continue;
        };
        if !matched.eq_ignore_ascii_case(INSTITUTION_NAME_PATTERN)
            || is_ascii_word_before(&padded, start)
            || is_ascii_word_at(&padded, start + pattern_len)
        {
            continue;
        }

        let span_start = padded[..start].chars().count();
        let span_end = span_start + matched.chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.institution_name",
            "message": format!(
                "Incorrect name. Use '{}' instead of '{}'.",
                INSTITUTION_NAME_REPLACEMENT,
                matched
            ),
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": INSTITUTION_NAME_REPLACEMENT,
        }));
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.professions` preferred-form rule.
///
/// PreferredFormsSimple uses case-insensitive matching with WORDS_IN_TEXT
/// boundaries and suppresses matches inside paired quotes.
pub fn professions_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (pattern, replacement) in PROFESSIONS {
        let pattern_len = pattern.len();
        for (start, _) in padded.char_indices() {
            let Some(matched) = padded.get(start..start.saturating_add(pattern_len)) else {
                continue;
            };
            if !matched.eq_ignore_ascii_case(pattern)
                || is_ascii_word_before(&padded, start)
                || is_ascii_word_at(&padded, start + pattern_len)
            {
                continue;
            }

            let span_start = padded[..start].chars().count();
            let span_end = span_start + pattern.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.professions",
                    "message": format!("'{replacement}' is the name of that job."),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }

    matches.sort_unstable_by_key(|(start, _)| *start);
    matches
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
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
