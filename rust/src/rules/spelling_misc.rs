use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Source: Garner's Modern American Usage; pinned spelling/misc.py SHA-256 05f0777984407bdc6c98549bab59629293af01ff00ff24ccf1d2cd90a4ea85ab.
const FORMS: [(&str, &str); 128] = [
    ("alot", "a lot"),
    ("accomodatable", "accommodable"),
    ("analingus", "anilingus"),
    ("amn't i", "aren't i"),
    ("an't i", "aren't i"),
    ("bob wire", "barbed wire"),
    ("chargable", "chargeable"),
    ("chiffonnier", "chiffonier"),
    ("chinchy", "chintzy"),
    ("chipolte", "chipotle"),
    ("chlorophyl", "chlorophyll"),
    ("chronical", "chronicle"),
    ("chronicals", "chronicles"),
    ("coldslaw", "coleslaw"),
    ("choosey", "choosy"),
    ("kummerbund", "cummerbund"),
    ("financable", "financeable"),
    ("flourescent", "fluorescent"),
    ("flouridation", "fluoridation"),
    ("flouride", "fluoride"),
    ("forclose", "foreclose"),
    ("foreswear", "forswear"),
    ("free reign", "free rein"),
    ("jist", "gist"),
    ("glamor", "glamour"),
    ("grandad", "granddad"),
    ("granpa", "grandpa"),
    ("highfaluting", "highfalutin"),
    ("highfalutin'", "highfalutin"),
    ("hifalutin", "highfalutin"),
    ("hypocratic", "Hippocratic"),
    ("hireable", "hirable"),
    ("wholistic", "holistic"),
    ("idealogy", "ideology"),
    ("ideosyncracy", "idiosyncrasy"),
    ("improvize", "improvise"),
    ("incurment", "incurrence"),
    ("inevitableness", "inevitability"),
    ("inovate", "innovate"),
    ("innoculation", "inoculation"),
    ("inocculation", "inoculation"),
    ("intergral", "integral"),
    ("innundate", "inundate"),
    ("innundated", "inundated"),
    ("innundates", "inundates"),
    ("innundating", "inundating"),
    ("irridescent", "iridescent"),
    ("irrevelant", "irrelevant"),
    ("jiujutsu", "jujitsu"),
    ("kaleidascope", "kaleidoscope"),
    ("knicknack", "knickknack"),
    ("lassoes", "lassos"),
    ("leasee", "lessee"),
    ("leasor", "lessor"),
    ("liason", "liaison"),
    ("laison", "liaison"),
    ("lightening rod", "lightning rod"),
    ("struck by lightening", "struck by lightning"),
    ("liquify", "liquefy"),
    ("loathesome", "loathsome"),
    ("loadstar", "lodestar"),
    ("to make due", "to make do"),
    ("madamoiselle", "mademoiselle"),
    ("marshall arts", "martial arts"),
    ("court marshall", "court-martial"),
    ("court-marshall", "court-martial"),
    ("maelstorm", "maelstrom"),
    ("maffia", "mafia"),
    ("mafiosos", "mafiosi"),
    ("mangos", "mangoes"),
    ("marihuana", "marijuana"),
    ("marshmellow", "marshmallow"),
    ("marshall law", "martial law"),
    ("massacering", "massacring"),
    ("massacreing", "massacring"),
    ("measels", "measles"),
    ("momento", "memento"),
    ("miniscule", "minuscule"),
    ("milage", "mileage"),
    ("mileau", "milieu"),
    ("millenium", "millennium"),
    ("millenia", "millennia"),
    ("milleniums", "millenniums"),
    ("millepede", "millipede"),
    ("millionnaire", "millionaire"),
    ("milktoast", "milquetoast"),
    ("mimiced", "mimicked"),
    ("mimicing", "mimicking"),
    ("mispelling", "misspelling"),
    ("mischievious", "mischievous"),
    ("mocasin", "moccasin"),
    ("moccassin", "moccasin"),
    ("mocassin", "moccasin"),
    ("monolog", "monologue"),
    ("monologist", "monologuist"),
    ("naptha", "naphtha"),
    ("naval orange", "navel orange"),
    ("neckware", "neckwear"),
    ("newstand", "newsstand"),
    ("non sequiter", "non sequitur"),
    ("non sequitar", "non sequitur"),
    ("non sequitor", "non sequitur"),
    ("nouveau rich", "nouveau riche"),
    ("occured", "occurred"),
    ("planter fascitis", "plantar fasciitis"),
    ("planter fasciitis", "plantar fasciitis"),
    ("plantar fascitis", "plantar fasciitis"),
    ("pledgable", "pledgeable"),
    ("pledgeble", "pledgeable"),
    ("portentuous", "portentous"),
    ("portentious", "portentous"),
    ("preying mantis", "praying mantis"),
    ("prestablished", "preestablished"),
    ("prexisting", "preexisting"),
    ("presumptious", "presumptuous"),
    ("rarify", "rarefy"),
    ("wreckless", "reckless"),
    ("renumeration", "remuneration"),
    ("restauranteur", "restaurateur"),
    ("revery", "reverie"),
    ("spicey", "spicy"),
    ("stupify", "stupefy"),
    ("subtley", "subtly"),
    ("tenderbox", "tinderbox"),
    ("tympani", "timpani"),
    ("a timpano", "a timpani"),
    ("umteenth", "umpteenth"),
    ("verbage", "verbiage"),
];
static ACTIVATIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            RegexBuilder::new(&format!(r"(?-u:\b){term}(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .unwrap()
        })
        .collect()
});
static RAW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(term, _)| {
            RegexBuilder::new(term)
                .case_insensitive(true)
                .build()
                .unwrap()
        })
        .collect()
});
static TILL: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?i)(?-u:\B)'till?(?-u:\b)")
        .build()
        .unwrap()
});

pub fn diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quotes = crate::paired_quote_bounds(&padded);
    let lines = crate::splitlines_bounds(&padded);
    let mut found = Vec::new();
    for (index, activation) in ACTIVATIONS.iter().enumerate() {
        if activation.find(&padded).is_none() {
            continue;
        }
        for matched in RAW_PATTERNS[index].find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let original = matched.as_str().trim();
            let replacement = FORMS
                .iter()
                .find(|(term, _)| *term == original.to_lowercase())
                .map(|(_, replacement)| *replacement);
            push_finding(
                &mut found,
                start,
                start + matched.as_str().chars().count(),
                crate::line_col_of(start, &lines),
                replacement,
            );
        }
    }

    for matched in TILL.find_iter(&padded) {
        let start = padded[..matched.start()].chars().count();
        if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
            continue;
        }
        push_finding(
            &mut found,
            start,
            start + matched.as_str().chars().count(),
            crate::line_col_of(start, &lines),
            Some("till"),
        );
    }

    found.sort_by_key(|(start, order, _)| (*start, *order));
    found.into_iter().map(|(_, _, value)| value).collect()
}

fn push_finding(
    found: &mut Vec<(usize, usize, Value)>,
    start: usize,
    end: usize,
    (line, column): (usize, usize),
    replacement: Option<&str>,
) {
    found.push((
        start,
        found.len(),
        json!({
            "check_path": "spelling.misc",
            "message": format!(
                "Misspelling. '{}' is the preferred spelling.",
                replacement.unwrap_or("None")
            ),
            "span": [start, end],
            "pos": [line, column],
            "replacements": replacement,
        }),
    ));
}
