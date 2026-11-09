use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Mary Norris, http://nyr.kr/1rGienj.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const FORMS: [(&str, &str); 19] = [
    ("manchesterian", "Mancunian"),
    ("manchesterians", "Mancunians"),
    ("valladolidian", "Vallisoletano"),
    ("wolverhamptonian", "Wulfrunian"),
    ("wolverhamptonite", "Wulfrunian"),
    ("newcastleite", "Novocastrian"),
    ("newcastlite", "Novocastrian"),
    ("trois-rivièrester", "Trifluvian"),
    ("leedsian", "Leodenisian"),
    ("minneapolisian", "Minneapolitan"),
    ("hartlepoolian", "Hartlepudlian"),
    ("liverpoolian", "Liverpudlian"),
    ("halifaxer", "Haligonian"),
    ("warsawer", "Varsovian"),
    ("warsawian", "Varsovian"),
    ("providencian", "Providentian"),
    ("providencer", "Providentian"),
    ("trentian", "Tridentine"),
    ("trentonian", "Tridentine"),
];

static PATTERNS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            let escaped = regex::escape(term);
            let activation = RegexBuilder::new(&format!(r"(?-u:\b){escaped}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned denizen label");
            let raw = RegexBuilder::new(&escaped)
                .case_insensitive(true)
                .build()
                .expect("valid pinned denizen label");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `terms.denizen_labels.norris` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();
    for (index, ((_, replacement), (activation, raw))) in
        FORMS.iter().zip(PATTERNS.iter()).enumerate()
    {
        if activation.find(&padded).is_none() {
            continue;
        }
        for matched in raw.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            diagnostics.push((
                start,
                index,
                json!({
                    "check_path": "terms.denizen_labels.norris",
                    "message": format!("Would you like '{replacement}'?"),
                    "span": [start, end],
                    "pos": [line, column],
                    "replacements": replacement,
                }),
            ));
        }
    }
    diagnostics.sort_by_key(|(start, index, _)| (*start, *index));
    diagnostics
        .into_iter()
        .map(|(_, _, diagnostic)| diagnostic)
        .collect()
}
