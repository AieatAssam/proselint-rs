use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Garner's Modern American Usage, http://bit.ly/1T4alrY.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const FORMS: [(&str, &str); 32] = [
    ("affrontery", "effrontery"),
    ("analyzation", "analysis"),
    ("annoyment", "annoyance"),
    ("confirmant", "confirmand"),
    ("confirmants", "confirmands"),
    ("conversate", "converse"),
    ("crained", "cranded"),
    ("discomforture", "discomfort' or 'discomfiture"),
    ("dispersement", "disbursement' or 'dispersal"),
    ("doubtlessly", "doubtless' or 'undoubtedly"),
    ("forebearance", "forbearance"),
    ("improprietous", "improper"),
    ("inclimate", "inclement"),
    ("inimicable", "inimical"),
    ("irregardless", "regardless"),
    ("minimalize", "minimize"),
    ("minimalized", "minimized"),
    ("minimalizes", "minimizes"),
    ("minimalizing", "minimizing"),
    ("optimalize", "optimize"),
    ("paralyzation", "paralysis"),
    ("pettifogger", "pettifog"),
    ("proprietous", "proper"),
    (
        "relative inexpense",
        "relatively low price' or 'affordability",
    ),
    ("seldomly", "seldom"),
    ("squelch", "quell' or 'quash"),
    ("thusly", "thus"),
    ("uncategorically", "categorically"),
    ("undoubtably", "undoubtedly' or 'indubitably"),
    ("unequivocable", "unequivocal"),
    ("unmercilessly", "mercilessly"),
    ("unrelentlessly", "unrelentingly' or relentlessly"),
];

static PATTERNS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            let escaped = regex::escape(term);
            let activation = RegexBuilder::new(&format!(r"(?-u:\b){escaped}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid pinned nonword");
            let raw = RegexBuilder::new(&escaped)
                .case_insensitive(true)
                .build()
                .expect("valid pinned nonword");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `nonwords` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();
    for (index, ((_, replacement), (activation, raw))) in
        FORMS.iter().zip(PATTERNS.iter()).enumerate()
    {
        let Some(first) = activation.find(&padded) else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for matched in raw.find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let end = start + matched.as_str().chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "nonwords",
                "message": format!("Nonword - try '{replacement}'."),
                "span": [start, end],
                "pos": [line, column],
                "replacements": replacement,
            }));
        }
        groups.push((first.start(), index, diagnostics));
    }
    groups.sort_by_key(|(start, index, _)| (*start, *index));
    groups
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}
