use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Garner's Modern American Usage, https://bit.ly/1T4alrY.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const FORMS: [(&str, &str); 46] = [
    ("hawk-like", "accipitrine"),
    ("goose-like", "anserine"),
    ("eagle-like", "aquiline"),
    ("bird-like", "avine"),
    ("crab-like", "cancrine"),
    ("goat-like", "hircine"),
    ("deer-like", "damine"),
    ("crow-like", "corvine"),
    ("raven-like", "corvine"),
    ("crocodile-like", "crocodiline"),
    ("rattlesnake-like", "crotaline"),
    ("falcon-like", "falconine"),
    ("wild animal-like", "ferine"),
    ("hippopotamus-like", "hippopotamine"),
    ("swallow-like", "hirundine"),
    ("porcupine-like", "hystricine"),
    ("lizard-like", "lacertine"),
    ("gull-like", "laridine"),
    ("hare-like", "leporine"),
    ("earthworm-like", "lumbricine"),
    ("wolf-like", "lupine"),
    ("mouse-like", "murine"),
    ("sheep-like", "ovine"),
    ("leopard-like", "pardine"),
    ("panther-like", "pardine"),
    ("sparrow-like", "passerine"),
    ("peacock-like", "pavonine"),
    ("woodpecker-like", "picine"),
    ("fish-like", "piscine"),
    ("frog-like", "ranine"),
    ("centipede-like", "scolopendrine"),
    ("shrew-like", "soricine"),
    ("ostrich-like", "struthionine"),
    ("swine-like", "suilline"),
    ("bull-like", "taurine"),
    ("ox-like", "taurine"),
    ("tiger-like", "tigrine"),
    ("wasp-like", "vespine"),
    ("viper-like", "viperine"),
    ("calf-like", "vituline"),
    ("veal-like", "vituline"),
    ("mongoose-like", "viverrine"),
    ("fox-like", "vulpine"),
    ("vulture-like", "vulturine"),
    ("zebra-like", "zebrine"),
    ("sable-like", "zibeline"),
];

static PATTERNS: LazyLock<Vec<(Regex, Regex)>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            let escaped = regex::escape(term);
            let activation = RegexBuilder::new(&format!(r"\b{escaped}\b"))
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid pinned animal adjective");
            let raw = RegexBuilder::new(&escaped)
                .case_insensitive(true)
                .unicode(false)
                .build()
                .expect("valid pinned animal adjective");
            (activation, raw)
        })
        .collect()
});

/// Return diagnostics for the pinned `terms.animal_adjectives` check.
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
            let original = matched.as_str();
            let end = start + original.chars().count();
            let (line, column) = crate::line_col_of(start, &line_bounds);
            diagnostics.push((
                start,
                index,
                json!({
                    "check_path": "terms.animal_adjectives",
                    "message": format!("There's a word for this: '{replacement}'."),
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
