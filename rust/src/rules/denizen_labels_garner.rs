use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Garner's Modern American Usage, http://bit.ly/1T4alrY.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const FORMS: [(&str, Option<&str>); 41] = [
    ("afrikaaner", Some("Afrikaner")),
    ("afrikander", Some("Afrikaner")),
    ("alabaman", Some("Alabamian")),
    ("albuquerquian", Some("Albuquerquean")),
    ("anchoragite", Some("Anchorageite")),
    ("los Angelean", None),
    ("arizonian", Some("Arizonan")),
    ("arkansawyer", Some("Arkansan")),
    ("belarusan", Some("Belarusian")),
    ("cayman islander", Some("Caymanian")),
    ("coloradoan", Some("Coloradan")),
    ("fairbanksian", Some("Fairbanksan")),
    ("fort worther", Some("Fort Worthian")),
    ("grenadian", Some("Grenadan")),
    ("hong kongite", Some("Hong Konger")),
    ("hong kongian", Some("Hong Konger")),
    ("indianan", Some("Hoosier")),
    ("indianian", Some("Hoosier")),
    ("illinoisian", Some("Illinoisan")),
    ("iowegian", Some("Iowan")),
    ("louisianan", Some("Louisianian")),
    ("michiganite", Some("Michigander")),
    ("michiganian", Some("Michigander")),
    ("missouran", Some("Missourian")),
    ("monacan", Some("Monegasque")),
    ("neopolitan", Some("Neapolitan")),
    ("new hampshireite", Some("New Hampshirite")),
    ("new hampshireman", Some("New Hampshirite")),
    ("new jerseyite", Some("New Jerseyan")),
    ("new orleansian", Some("New Orleanian")),
    ("connecticuter", Some("Nutmegger")),
    ("oklahoma citian", Some("Oklahoma Cityan")),
    ("oklahomian", Some("Oklahoman")),
    ("seattlite", Some("Seattleite")),
    ("surinamer", Some("Surinamese")),
    ("tallahassean", Some("Tallahasseean")),
    ("tennesseean", Some("Tennessean")),
    ("tusconian", Some("Tusconan")),
    ("tusconite", Some("Tusconan")),
    ("utahan", Some("Utahn")),
    ("saudi arabian", Some("Saudi")),
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

/// Return diagnostics for the pinned `terms.denizen_labels.garner` check.
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
            let shown = replacement.unwrap_or("None");
            diagnostics.push((
                start,
                index,
                json!({
                    "check_path": "terms.denizen_labels.garner",
                    "message": format!("'{shown}' is the preferred denizen label."),
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
