use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r#""[\w\s\d]+""#)
        .case_insensitive(true)
        .unicode(false)
        .build()
        .expect("valid pinned curly quotes pattern")
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let line_bounds = crate::splitlines_bounds(&padded);
    PATTERN
        .find_iter(&padded)
        .map(|matched| {
            let start = padded[..matched.start()].chars().count();
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            json!({
                "check_path": "typography.symbols.curly_quotes",
                "message": "Use curly quotes “”, not straight quotes \"\".",
                "span": [start, end],
                "pos": [line, column],
                "replacements": Value::Null,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::diagnostics;
    use serde_json::{Value, json};

    fn finding(span: [usize; 2], pos: [usize; 2]) -> Value {
        json!({
            "check_path": "typography.symbols.curly_quotes",
            "message": "Use curly quotes “”, not straight quotes \"\".",
            "span": span,
            "pos": pos,
            "replacements": null,
        })
    }

    #[test]
    fn reports_ascii_words_digits_spaces_and_matches_inside_quotes() {
        assert_eq!(
            diagnostics("A \"hello 42\" phrase"),
            vec![finding([3, 13], [1, 3])],
        );
    }

    #[test]
    fn regex_whitespace_can_cross_lines() {
        assert_eq!(
            diagnostics("before \"two\nlines\" after"),
            vec![finding([8, 19], [1, 8])],
        );
    }

    #[test]
    fn punctuation_and_empty_quotes_do_not_match() {
        assert_eq!(
            diagnostics("\"hello, world\" \"\""),
            vec![finding([14, 17], [1, 14])],
        );
    }

    #[test]
    fn non_ascii_internal_word_character_breaks_the_fast_re2_class() {
        assert!(diagnostics("\"café\"").is_empty());
    }

    #[test]
    fn positions_count_unicode_codepoints() {
        assert_eq!(
            diagnostics("é 😊 \"hello\""),
            vec![finding([5, 12], [1, 5])],
        );
    }
}
