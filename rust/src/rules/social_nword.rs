use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?i)the n-?word")
        .build()
        .expect("valid pinned n-word pattern")
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let chars: Vec<char> = padded.chars().collect();
    let quotes = crate::paired_quote_bounds(&padded);
    PATTERN
        .find_iter(&padded)
        .filter_map(|matched| {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                return None;
            }
            let line_start = chars[..start]
                .iter()
                .rposition(|ch| *ch == '\n')
                .unwrap_or(0);
            let line = chars[..start].iter().filter(|ch| **ch == '\n').count();
            Some(json!({
                "check_path": "social_awareness.nword",
                "message": "Take responsibility for the words you want to say.",
                "span": [start, start + matched.as_str().chars().count()],
                "pos": [line, start - line_start],
                "replacements": Value::Null,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::diagnostics;
    use serde_json::{Value, json};

    fn finding(span: [usize; 2], pos: [usize; 2]) -> Value {
        json!({
            "check_path": "social_awareness.nword",
            "message": "Take responsibility for the words you want to say.",
            "span": span,
            "pos": pos,
            "replacements": null,
        })
    }

    #[test]
    fn reports_unanchored_case_insensitive_matches_in_order() {
        assert_eq!(
            diagnostics("THE N-WORD, then the nword"),
            vec![finding([1, 11], [1, 1]), finding([18, 27], [1, 18])],
        );
    }

    #[test]
    fn reports_raw_match_inside_a_larger_word() {
        assert_eq!(
            diagnostics("They tried to breathe nword before speaking."),
            vec![finding([19, 28], [1, 19])],
        );
    }

    #[test]
    fn positions_count_unicode_codepoints() {
        assert_eq!(
            diagnostics("é 😊 then the nword."),
            vec![finding([10, 19], [1, 10])],
        );
    }

    #[test]
    fn suppresses_matches_inside_paired_quotes() {
        assert!(diagnostics("She wrote \"the n-word\" in the draft.").is_empty());
    }

    #[test]
    fn a_space_instead_of_the_hyphen_is_a_near_miss() {
        assert!(diagnostics("the n word").is_empty());
    }
}
