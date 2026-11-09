use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Oxford Dictionaries, http://www.oxforddictionaries.com/words/what-do-you-call-a-group-of.
// Rule data retained under the package BSD-3-Clause basis; this is not a legal guarantee.
const TERMS: [(&str, &str); 49] = [
    ("alligators", "congregation"),
    ("antelopes", "herd"),
    ("baboons", "troop"),
    ("badgers", "cete"),
    ("bats", "colony"),
    ("bears", "sloth"),
    ("buffalo", "herd"),
    ("bullfinches", "bellowing"),
    ("caribou", "herd"),
    ("cats", "glaring"),
    ("caterpillars", "army"),
    ("cockroaches", "intrusion"),
    ("coyotes", "pack"),
    ("crows", "murder"),
    ("dogs", "pack"),
    ("eagles", "convocation"),
    ("emus", "mob"),
    ("flamingos", "stand"),
    ("frogs", "army"),
    ("goldfinches", "charm"),
    ("gorillas", "band"),
    ("guineafowl", "rasp"),
    ("hedgehogs", "array"),
    ("herons", "siege"),
    ("hogs", "parcel"),
    ("hyenas", "cackle"),
    ("ibex", "herd"),
    ("iguanas", "mess"),
    ("lions", "pride"),
    ("locusts", "plague"),
    ("mackerel", "shoal"),
    ("mares", "stud"),
    ("minnows", "shoal"),
    ("moose", "herd"),
    ("mosquitoes", "scourge"),
    ("nightingales", "watch"),
    ("oysters", "bed"),
    ("partridges", "covey"),
    ("pelicans", "pod"),
    ("raccoons", "gaze"),
    ("ravens", "unkindness"),
    ("rhinoceroses", "crash"),
    ("sea urchins", "sea"),
    ("starlings", "murmuration"),
    ("toads", "knot"),
    ("wombats", "wisdom"),
    ("woodcocks", "fall"),
    ("woodpeckers", "descent"),
    ("wrens", "herd"),
];
const GENERIC_TERMS: [&str; 2] = ["group", "bunch"];

static TRIGGER: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new("group|bunch")
        .case_insensitive(true)
        .build()
        .expect("valid venery trigger")
});

static PATTERNS: LazyLock<Vec<(String, String, Regex, Regex)>> = LazyLock::new(|| {
    TERMS
        .iter()
        .flat_map(|(animal, collective)| {
            GENERIC_TERMS.iter().map(move |generic| {
                let term = format!("{generic} of {animal}");
                let replacement = format!("{collective} of {animal}");
                let escaped = regex::escape(&term);
                let activation = RegexBuilder::new(&format!(r"(?-u:\b){escaped}(?-u:\b)"))
                    .case_insensitive(true)
                    .build()
                    .expect("valid pinned venery term");
                let raw = RegexBuilder::new(&escaped)
                    .case_insensitive(true)
                    .build()
                    .expect("valid pinned venery term");
                (term, replacement, activation, raw)
            })
        })
        .collect()
});

/// Return diagnostics for the pinned `terms.venery` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    if !TRIGGER.is_match(text) {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let line_bounds = crate::splitlines_bounds(&padded);
    let mut groups = Vec::new();
    for (index, (_, replacement, activation, raw)) in PATTERNS.iter().enumerate() {
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
                "check_path": "terms.venery",
                "message": format!("The venery term is '{replacement}'."),
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
