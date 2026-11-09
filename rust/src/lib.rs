//! Native Rust implementation of Proselint.

pub mod rules;
pub use rules::archaism::diagnostics as archaism_diagnostics;
pub use rules::cliches_write_good::diagnostics as cliches_write_good_diagnostics;
pub use rules::dash_and_from::diagnostics as dates_times_dates_dash_and_from_diagnostics;
pub use rules::exclamation::diagnostics as exclamation_diagnostics;
pub use rules::hyperbole::diagnostics as hyperbole_diagnostics;
pub use rules::lexical_illusions::diagnostics as lexical_illusions_diagnostics;
pub use rules::misc_illogic::diagnostics as misc_illogic_diagnostics;
pub use rules::misc_tense_present::diagnostics as misc_tense_present_diagnostics;
pub use rules::misc_waxed::diagnostics as misc_waxed_diagnostics;
pub use rules::misplaced::diagnostics as misplaced_diagnostics;
pub use rules::mixed_metaphors_bottleneck::diagnostics as mixed_metaphors_bottleneck_diagnostics;
pub use rules::mixed_metaphors_misc::diagnostics as mixed_metaphors_misc_diagnostics;
pub use rules::month_of_year::diagnostics as dates_times_dates_month_of_year_diagnostics;
pub use rules::month_year_comma::diagnostics as dates_times_dates_month_year_comma_diagnostics;
pub use rules::redundancy_ras_syndrome::diagnostics as redundancy_ras_syndrome_diagnostics;
pub use rules::scare_quotes::diagnostics as scare_quotes_diagnostics;
pub use rules::social_awareness_lgbtq_offensive::diagnostics as social_awareness_lgbtq_offensive_diagnostics;
pub use rules::social_awareness_lgbtq_terms::diagnostics as social_awareness_lgbtq_terms_diagnostics;
pub use rules::social_nword::diagnostics as social_nword_diagnostics;
pub use rules::spelling_able_atable::diagnostics as spelling_able_atable_diagnostics;
pub use rules::spelling_able_ible::diagnostics as spelling_able_ible_diagnostics;
pub use rules::spelling_athletes::diagnostics as spelling_athletes_diagnostics;
pub use rules::spelling_em_im_en_in::diagnostics as spelling_em_im_en_in_diagnostics;
pub use rules::spelling_er_or::diagnostics as spelling_er_or_diagnostics;
pub use rules::typography_curly_quotes::diagnostics as typography_curly_quotes_diagnostics;
pub use rules::typography_diacritical_marks::diagnostics as typography_diacritical_marks_diagnostics;
pub use rules::typography_spacing::diagnostics as typography_spacing_diagnostics;
pub use rules::weasel_words_very::diagnostics as weasel_words_very_diagnostics;

use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value, json};
use std::path::Path;
use std::sync::LazyLock;

const ANNOTATIONS: [&str; 6] = ["FIXME", "FIX ME", "TODO", "todo", "ERASE THIS", "FIX THIS"];
const CLICHE_HELL_PATTERN: &str = "all hell broke loose";
const CLICHE_HELL_MESSAGE: &str = "Never use the phrase 'all hell broke loose'.";
const APOLOGIZING_PATTERN: &str = "More research is needed";
const APOLOGIZING_MESSAGE: &str = "Excessive apologizing.";
const PHRASAL_ADJECTIVES_MESSAGE: &str = "No hyphen is necessary in phrasal adjectives with an adverb ending in -ly, unless the -ly adverb is part of a longer phrase.";
const NARCISSISM_PATTERN: &str = r"In recent years, an increasing number of [a-zA-Z]{3,}sts have";
const NARCISSISM_MESSAGE: &str = "Professional narcissism. Talk about the subject, not its study.";
const NOT_GUILTY_PATTERN: &str = r"not guilty beyond (a |any )?reasonable doubt";
const NOT_GUILTY_MESSAGE: &str = "'not guilty beyond a reasonable doubt' is an ambiguous phrasing.";
const WHENCE_PATTERN: &str = "from whence";
const WHENCE_MESSAGE: &str = "The 'from' in 'from whence' is not needed.";
const SUDDENLY_PATTERN: &str = "Suddenly,";
const SUDDENLY_MESSAGE: &str = "Suddenly is nondescript, slows the action, and warns your reader.";
const BUT_PATTERN: &str = "But";
const BUT_MESSAGE: &str = "No paragraph should start with a 'But'.";
const AM_PM_MESSAGE: &str = "It's standard to use periods for lowercase 'a.m.' or 'p.m.'.";
const AM_PM_SPACING_MESSAGE: &str = "It's standard to put a space before 'a.m.' or 'p.m.'.";
const MIDNIGHT_NOON_MESSAGE: &str =
    "'12 a.m.' and '12 p.m.' are ambiguous. Use 'midnight' or 'noon'.";
const MIDNIGHT_NOON_REDUNDANT_PATTERNS: [&str; 4] = [
    r"[0-9]{1,2} ?a\.?m\.? in the morning",
    r"[0-9]{1,2} ?p\.?m\.? in the evening",
    r"[0-9]{1,2} ?p\.?m\.? at night",
    r"[0-9]{1,2} ?p\.?m\.? in the afternoon",
];
const MIDNIGHT_NOON_REDUNDANT_MESSAGE: &str = "'a.m.' is always morning; 'p.m.' is always night.";
const DECADE_APOSTROPHES_MESSAGE: &str = "Apostrophes aren't needed for decades.";
const DECADE_APOSTROPHES_PATTERNS: [&str; 2] = [r"[0-9]0's", r"[0-9]{3}0's"];
static MIDNIGHT_NOON_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"(?i)12 ?[ap]\.?m\.?")
        .build()
        .expect("valid midnight/noon regex")
});
static MIDNIGHT_NOON_REDUNDANT_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    MIDNIGHT_NOON_REDUNDANT_PATTERNS
        .iter()
        .map(|pattern| {
            RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .expect("valid midnight/noon redundancy regex")
        })
        .collect()
});
static DECADE_APOSTROPHES_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    DECADE_APOSTROPHES_PATTERNS
        .iter()
        .map(|pattern| {
            RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .expect("valid decade apostrophes regex")
        })
        .collect()
});
static NARCISSISM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(NARCISSISM_PATTERN)
        .case_insensitive(true)
        .build()
        .expect("valid narcissism regex")
});
static NOT_GUILTY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(NOT_GUILTY_PATTERN)
        .case_insensitive(true)
        .build()
        .expect("valid not-guilty regex")
});
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
const P_EQUALS_ZERO_MESSAGE: &str =
    "Unless p really equals zero, you should use more decimal places.";
const ILLOGIC_COIN_PATTERN: &str = "to coin a phrase from";
const ILLOGIC_COIN_MESSAGE: &str = "You can't coin an existing phrase. Did you mean 'borrow'?";
const ILLOGIC_COLLUSION_PATTERN: &str = "without your collusion";
const ILLOGIC_COLLUSION_MESSAGE: &str = "It's impossible to defraud yourself. Try 'aquiescence'.";
const BACK_FORMATIONS_PATTERN: &str = "improprietous";
const BACK_FORMATIONS_REPLACEMENT: &str = "improper";
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
const OXYMORON_PATTERNS: [&str; 17] = [
    "amateur expert",
    "increasingly less",
    "advancing backwards?",
    "alludes explicitly to",
    "explicitly alludes to",
    "totally obsolescent",
    "completely obsolescent",
    "generally always",
    "usually always",
    "build down",
    "conspicuous absence",
    "exact estimate",
    "found missing",
    "intense apathy",
    "mandatory choice",
    "nonworking mother",
    "organized mess",
];
const OXYMORON_MESSAGE: &str = "'{}' is an oxymoron.";
static OXYMORON_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    OXYMORON_PATTERNS
        .iter()
        .map(|pattern| {
            RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .expect("valid oxymoron pattern")
        })
        .collect()
});
static OXYMORON_ACTIVATION_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    OXYMORON_PATTERNS
        .iter()
        .map(|pattern| {
            RegexBuilder::new(&format!(r"(?-u:\b)(?:{pattern})(?-u:\b)"))
                .case_insensitive(true)
                .build()
                .expect("valid oxymoron activation pattern")
        })
        .collect()
});
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
const CAPITALIZATION_TERMS: [(&str, &str); 4] = [
    ("mother nature", "Mother Nature"),
    ("Space Age", "space age"),
    ("stone age", "Stone Age"),
    ("the American west", "the American West"),
];
const CAPITALIZATION_TERMS_MESSAGE: &str = "Incorrect capitalization. '{}' is the preferred form.";
const CAPITALIZATION_DAYS: [(&str, &str); 7] = [
    ("monday", "Monday"),
    ("tuesday", "Tuesday"),
    ("wednesday", "Wednesday"),
    ("thursday", "Thursday"),
    ("friday", "Friday"),
    ("saturday", "Saturday"),
    ("sunday", "Sunday"),
];
const CAPITALIZATION_DAYS_MESSAGE: &str =
    "Days of the week should be capitalized. '{}' is the preferred form.";
const CAPITALIZATION_MONTHS: [(&str, &str); 10] = [
    ("january", "January"),
    ("february", "February"),
    ("april", "April"),
    ("june", "June"),
    ("july", "July"),
    ("august", "August"),
    ("september", "September"),
    ("october", "October"),
    ("november", "November"),
    ("december", "December"),
];
const CAPITALIZATION_MONTHS_MESSAGE: &str =
    "Months should be capitalized. '{}' is the preferred form.";
const LIE_DETECTOR_FORMS: [(&str, &str); 2] = [
    ("lie detector test", "polygraph test"),
    ("lie detector machine", "polygraph machine"),
];
const LIE_DETECTOR_MESSAGE: &str =
    "Polygraph machines measure arousal, not lying per se. Try '{}'.";
const WALLACE_FORMS: [(&str, &str); 2] = [
    ("rectangular in shape", "rectangular"),
    ("audible to the ear", "audible"),
];
const WALLACE_MESSAGE: &str = "Redundancy. Use '{}' instead of '{}'.";
const NORDQUIST_FORMS: [&str; 4] = [
    "absolutely essential",
    "absolutely necessary",
    "a.m. in the morning",
    "p.m. at night",
];
const INSTITUTION_NAME_PATTERN: &str = "Virginia Polytechnic and State University";
const INSTITUTION_NAME_REPLACEMENT: &str = "Virginia Polytechnic Institute and State University";
const PROFESSIONS: [(&str, &str); 2] = [("shoe repair guy", "cobbler"), ("geometrist", "geometer")];
const EPONYMOUS_ADJECTIVES: [(&str, &str); 2] = [
    ("mephistophelian", "Mephistophelean"),
    ("shakespearian", "Shakespearean"),
];
const COMPOSITION_SIMPLE_FORMS: [(&str, &str); 20] = [
    ("not honest", "dishonest"),
    ("not important", "unimportant"),
    ("did not remember", "forgot"),
    ("did not pay attention to", "ignored"),
    ("did not have much confidence in", "distrusted"),
    ("the question as to whether", "whether"),
    ("there is no doubt but that", "no doubt"),
    ("used for fuel purposes", "used for fuel"),
    ("he is a man who", "he"),
    ("in a hasty manner", "hastily"),
    ("this is a subject that", "this subject"),
    ("a strange one", "strange."),
    ("the reason why is that", "because"),
    ("owing to the fact that", "because / since"),
    ("in spite of the fact that", "although / though"),
    (
        "call your attention to the fact that",
        "remind you / notify you",
    ),
    ("was unaware of the fact", "did not know / was unaware"),
    ("did not succeed", "failed"),
    ("not succeed", "fail"),
    ("the fact that i had arrived", "my arrival"),
];
const COMPOSITION_REGEX_FORMS: [(&str, &str); 3] = [
    (
        r"(?-u:\b)did not pay (any )?attention to(?-u:\b)",
        "ignored",
    ),
    (
        r"(?-u:\b)did not have (much )?(confidence|faith) in(?-u:\b)",
        "distrusted",
    ),
    (r"(?-u:\b)(had )?not succeeded(?-u:\b)", "failed"),
];
static COMPOSITION_REGEXES: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    std::array::from_fn(|index| {
        RegexBuilder::new(COMPOSITION_REGEX_FORMS[index].0)
            .case_insensitive(true)
            .unicode(true)
            .build()
            .expect("valid composition preferred-form pattern")
    })
});
const ANCE_ENCE_FORMS: [(&str, &str); 3] = [
    ("appearence", "appearance"),
    ("occurrance", "occurrence"),
    ("resistence", "resistance"),
];
const ALLY_LY_FORMS: [(&str, &str); 8] = [
    ("academicly", "academically"),
    ("accidently", "accidentally"),
    ("automaticly", "automatically"),
    ("basicly", "basically"),
    ("dramaticly", "dramatically"),
    ("emotionly", "emotionally"),
    ("incidently", "incidentally"),
    ("optimisticly", "optimistically"),
];
const ELY_LY_FORMS: [(&str, &str); 3] = [
    ("completly", "completely"),
    ("immediatly", "immediately"),
    ("unfortunatly", "unfortunately"),
];
const IN_UN_FORMS: [(&str, &str); 4] = [
    ("unadvisable", "inadvisable"),
    ("unalienable", "inalienable"),
    ("unexpressive", "inexpressive"),
    ("unfeasible", "infeasible"),
];
const MONDEGREENS_FORMS: [(&str, &str); 8] = [
    (
        "a girl with colitis goes by",
        "a girl with kaleidoscope eyes",
    ),
    ("a part-red gingerbread tree", "a partridge in a pear tree"),
    ("attorney and not a republic", "attorney and notary public"),
    ("beckon call", "beck and call"),
    ("for all intensive purposes", "for all intents and purposes"),
    ("lady mondegreen", "laid him on the green"),
    ("olive, the other reindeer", "all of the other reindeer"),
    ("to the manor born", "to the manner born"),
];
const MALAPROPISMS: [&str; 3] = [
    "the infinitesimal universe",
    "a serial experience",
    "attack my voracity",
];
const SKUNKED_TERMS: [&str; 8] = [
    "bona fides",
    "deceptively",
    "decimate",
    "effete",
    "fulsome",
    "hopefully",
    "impassionate",
    "Thankfully,",
];
const VE_OF_FORMS: [(&str, &str); 3] = [
    ("could of", "could've"),
    ("should of", "should've"),
    ("would of", "would've"),
];
const MESSAGE: &str = "Annotation left in text.";
pub(crate) const QUOTES: [char; 6] = ['"', '\'', '“', '”', '‘', '’'];
pub(crate) const SEPARATORS: &str = " ,.!?:;-\r\n";

/// Format an absolute local path as a `file:` URI, escaping bytes as Python's
/// `Path.as_uri()` does for the ASCII paths supported by the CLI.
pub fn file_uri(path: &Path) -> Result<String, std::io::Error> {
    let absolute = path.canonicalize()?;
    let bytes = absolute.to_string_lossy();
    let mut uri = String::from("file://");
    for byte in bytes.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(byte) {
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
    let mut diagnostics = annotation_diagnostics_in_matcher_order(text);
    diagnostics.sort_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return diagnostics for Fast `misc.phrasal_adjectives.ly` using the pinned
/// RE2 ASCII whitespace class explicitly.
pub fn phrasal_adjectives_ly_diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)[ \t\n\r\x0c][^ \t\n\r\x0c-]+ly-").expect("constant regex")
    });
    let quotes = paired_quote_bounds(&padded);
    let lines = splitlines_bounds(&padded);
    PATTERN.find_iter(&padded).filter_map(|m| {
        let matcher_start = padded[..m.start()].chars().count();
        let start = matcher_start + 1;
        let end = matcher_start + m.as_str().chars().count();
        (quotes.partition_point(|bound| *bound < start) % 2 == 0).then(|| {
            let (line, column) = line_col_of(start, &lines);
            (start, json!({"check_path":"misc.phrasal_adjectives.ly", "message":PHRASAL_ADJECTIVES_MESSAGE,
                "span":[start,end], "pos":[line,column], "replacements":null}))
        })
    }).map(|(_, diagnostic)| diagnostic).collect()
}

/// Return diagnostics for the pinned raw, case-insensitive narcissism pattern.
pub fn narcissism_diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    NARCISSISM_REGEX
        .find_iter(&padded)
        .filter_map(|matched| {
            let span_start = padded[..matched.start()].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                return None;
            }
            let span_end = span_start + matched.as_str().chars().count();
            let (line, column) = line_col_of(span_start, &line_bounds);
            Some(json!({
                "check_path": "misc.narcissism",
                "message": NARCISSISM_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }))
        })
        .collect()
}

/// Return diagnostics for the pinned raw, case-insensitive not-guilty pattern.
pub fn not_guilty_diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    NOT_GUILTY_REGEX
        .find_iter(&padded)
        .filter_map(|matched| {
            let padded_start = padded[..matched.start()].chars().count();
            let span_start = padded_start;
            let quote_index = quote_bounds.partition_point(|bound| *bound < padded_start);
            if quote_index % 2 == 1 {
                return None;
            }
            let span_end = span_start + matched.as_str().chars().count();
            let (line, column) = line_col_of(padded_start, &line_bounds);
            Some(json!({
                "check_path": "misc.not_guilty",
                "message": NOT_GUILTY_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }))
        })
        .collect()
}

/// Return annotation diagnostics in upstream matcher registration order,
/// before the final source-coordinate sort used by `annotation_diagnostics`.
pub fn annotation_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut spans = Vec::new();

    let mut annotation_patterns = ANNOTATIONS
        .iter()
        .map(|item| {
            let first_end = padded
                .match_indices(item)
                .filter_map(|(start, m)| {
                    let end = start + m.len();
                    (!is_ascii_word_before(&padded, start) && !is_ascii_word_at(&padded, end))
                        .then_some(end)
                })
                .min();
            (*item, first_end)
        })
        .filter(|(_, first_end)| first_end.is_some())
        .collect::<Vec<_>>();
    annotation_patterns.sort_by_key(|(_, first_end)| *first_end);
    for (item, _) in annotation_patterns {
        for (start, matched) in padded.match_indices(item) {
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
    for (start, _) in padded.char_indices() {
        let Some(end) = unicode_case_match_end(&padded, start, CLICHE_HELL_PATTERN) else {
            continue;
        };

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
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

/// Return diagnostics for the pinned `malapropisms` existence check.
pub fn malapropisms_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, pattern) in MALAPROPISMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };

        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "malapropisms",
                "message": format!("'{}' is a malapropism.", matched),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the pinned `skunked_terms` existence check.
pub fn skunked_terms_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, pattern) in SKUNKED_TERMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                is_ascii_word_before(&padded, *start) != is_ascii_word_at(&padded, *start)
                    && is_ascii_word_before(&padded, *end) != is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };

        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "skunked_terms",
                "message": format!("'{}' is a a skunked term - impossible to use without issue. Find some other way to say it.", matched.trim()),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    // FastSet selects patterns by padded boundaries, then emits their raw matches.
    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

#[cfg(test)]
mod skunked_terms_tests {
    use super::skunked_terms_diagnostics;
    use serde_json::json;

    #[test]
    fn reports_the_seven_standard_terms() {
        let diagnostics = skunked_terms_diagnostics(
            "bona fides deceptively decimate effete fulsome hopefully impassionate",
        );
        assert_eq!(diagnostics.len(), 7);
        assert_eq!(diagnostics[0]["span"], json!([1, 11]));
        assert_eq!(
            diagnostics[0]["message"],
            "'bona fides' is a a skunked term - impossible to use without issue. Find some other way to say it."
        );
        assert_eq!(
            diagnostics[6]["message"],
            "'impassionate' is a a skunked term - impossible to use without issue. Find some other way to say it."
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic["replacements"].is_null())
        );
    }

    #[test]
    fn comma_pattern_needs_a_following_word_for_activation() {
        let diagnostics = skunked_terms_diagnostics("Thankfully,however");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["span"], json!([1, 12]));
        assert_eq!(
            diagnostics[0]["message"],
            "'Thankfully,' is a a skunked term - impossible to use without issue. Find some other way to say it."
        );
        assert!(skunked_terms_diagnostics("Thankfully,").is_empty());
        assert!(skunked_terms_diagnostics("Thankfully, however").is_empty());
    }

    #[test]
    fn quoted_activation_keeps_the_later_raw_match_before_a_space() {
        let diagnostics =
            skunked_terms_diagnostics("\"Thankfully,however\" decimate Thankfully, I agree.");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0]["span"], json!([31, 42]));
        assert_eq!(diagnostics[1]["span"], json!([22, 30]));
    }
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
    for (start, _) in padded.char_indices() {
        let Some(end) = unicode_case_match_end(&padded, start, APOLOGIZING_PATTERN) else {
            continue;
        };

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
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

/// Return diagnostics for both pinned direct decade apostrophe regex checks,
/// preserving registration order for max_errors truncation.
pub fn decade_apostrophes_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for regex in DECADE_APOSTROPHES_REGEXES.iter() {
        for matched in regex.find_iter(&padded) {
            let span_start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < span_start) % 2 == 1 {
                continue;
            }
            let span_end = span_start + matched.as_str().chars().count();
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "dates_times.dates.decade_apostrophes",
                "message": DECADE_APOSTROPHES_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
    }

    diagnostics
}

/// Return diagnostics for both pinned checks registered at the shared
/// midnight/noon path, preserving registration and FastSet matcher order.
pub fn midnight_noon_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for matched in MIDNIGHT_NOON_REGEX.find_iter(&padded) {
        let span_start = padded[..matched.start()].chars().count();
        if quote_bounds.partition_point(|bound| *bound < span_start) % 2 == 1 {
            continue;
        }
        let span_end = span_start + padded[matched.start()..matched.end()].chars().count();
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "dates_times.am_pm.midnight_noon",
            "message": MIDNIGHT_NOON_MESSAGE,
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": null,
        }));
    }

    let mut active_patterns = MIDNIGHT_NOON_REDUNDANT_REGEXES
        .iter()
        .enumerate()
        .filter_map(|(index, regex)| {
            let matches = regex.find_iter(&padded).collect::<Vec<_>>();
            let activation_start = matches
                .iter()
                .find(|matched| {
                    !is_ascii_word_before(&padded, matched.start())
                        && !is_ascii_word_at(&padded, matched.end())
                })
                .map(|matched| matched.start());
            activation_start.map(|start| (start, index, matches))
        })
        .collect::<Vec<_>>();
    active_patterns.sort_unstable_by_key(|(start, index, _)| (*start, *index));

    for (_, _, matches) in active_patterns {
        for matched in matches {
            let span_start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < span_start) % 2 == 1 {
                continue;
            }
            let span_end = span_start + padded[matched.start()..matched.end()].chars().count();
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "dates_times.am_pm.midnight_noon",
                "message": MIDNIGHT_NOON_REDUNDANT_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
    }

    diagnostics
}

/// Return diagnostics for the pinned `misc.debased` check. WORDS_IN_TEXT
/// first selects active phrases; FastSet then searches each selected raw
/// phrase throughout the padded text, so embedded copies of an active phrase
/// can also be reported. Matching is case-insensitive and quoted occurrences
/// are suppressed.
pub fn debased_diagnostics(text: &str) -> Vec<Value> {
    let mut diagnostics = debased_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return `misc.debased` findings in FastSet activation order for CLI limits.
pub fn debased_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let lowered = padded.to_ascii_lowercase();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, item) in DEBASED_ITEMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&lowered, item);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&lowered, *start) && !is_ascii_word_at(&lowered, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };

        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let span_start = padded[..start].chars().count();
            let span_end = span_start + lowered[start..end].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "misc.debased",
                "message": DEBASED_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
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

        let suffix = ["dollars", "usd", "us dollars"]
            .into_iter()
            .find_map(|suffix| unicode_case_match_end(&lowered, suffix_start, suffix));
        let Some(end) = suffix else {
            continue;
        };
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

    let mut patterns = INFERIOR_SUPERIOR
        .iter()
        .map(|(p, r)| (*p, *r))
        .collect::<Vec<_>>();
    patterns.sort_by_key(|(p, _)| {
        unicode_case_match_ends(&lowered, p)
            .iter()
            .filter(|(s, e)| !is_ascii_word_before(&lowered, *s) && !is_ascii_word_at(&lowered, *e))
            .map(|(_, e)| *e)
            .min()
    });
    for (pattern, replacement) in patterns {
        let active = unicode_case_match_ends(&lowered, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&lowered, *start) && !is_ascii_word_at(&lowered, *end)
            });
        if !active {
            continue;
        }
        for (start, end) in unicode_case_match_ends(&lowered, pattern) {
            let span_start = padded[..start].chars().count();
            let span_end = span_start + lowered[start..end].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.inferior_superior",
                    "message": INFERIOR_SUPERIOR_MESSAGE.replace("{}", preferred_lookup(&lowered[start..end], replacement).unwrap_or("None")),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": preferred_lookup(&lowered[start..end], replacement),
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
    for (start, _) in padded.char_indices() {
        let Some(end) = unicode_case_match_end(&padded, start, KUDOS_PATTERN) else {
            continue;
        };

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
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
    let mut diagnostics = false_plurals_misc_diagnostics_in_matcher_order(text);
    diagnostics.sort_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

pub fn false_plurals_misc_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let lowered = padded.to_ascii_lowercase();
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    let mut patterns = FALSE_PLURALS_MISC
        .iter()
        .map(|(p, r)| (*p, *r))
        .collect::<Vec<_>>();
    patterns.sort_by_key(|(pattern, _)| {
        unicode_case_match_ends(&lowered, pattern)
            .iter()
            .filter(|(s, e)| !is_ascii_word_before(&lowered, *s) && !is_ascii_word_at(&lowered, *e))
            .map(|(_, e)| *e)
            .min()
    });
    for (pattern, replacement) in patterns {
        let active = unicode_case_match_ends(&lowered, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&lowered, *start) && !is_ascii_word_at(&lowered, *end)
            });
        if !active {
            continue;
        }
        for (start, end) in unicode_case_match_ends(&lowered, pattern) {
            let span_start = padded[..start].chars().count();
            let span_end = span_start + lowered[start..end].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "misc.false_plurals.misc",
                    "message": format!("The correct plural is '{}'.", if lowered[start..end].contains('ſ') { "None" } else { replacement }),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": if lowered[start..end].contains('ſ') { Value::Null } else { json!(replacement) },
                }),
            ));
        }
    }

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

/// Return diagnostics for the pinned Fast `psychology.p_equals_zero` regex.
pub fn p_equals_zero_diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    static PATTERN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)p = 0.0{2,4}").expect("constant regex"));
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    PATTERN
        .find_iter(&padded)
        .filter_map(|matched| {
            let span_start = padded[..matched.start()].chars().count();
            let span_end = span_start + matched.as_str().chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                return None;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            Some(json!({
                "check_path": "psychology.p_equals_zero",
                "message": P_EQUALS_ZERO_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }))
        })
        .collect()
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
    for (start, end) in unicode_case_match_ends(&padded, ILLOGIC_COIN_PATTERN) {
        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
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
    for (start, end) in unicode_case_match_ends(&padded, ILLOGIC_COLLUSION_PATTERN) {
        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
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
    let active = unicode_case_match_ends(&padded, BACK_FORMATIONS_PATTERN)
        .iter()
        .any(|(start, end)| {
            !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
        });
    if !active {
        return diagnostics;
    }

    for (start, end) in unicode_case_match_ends(&padded, BACK_FORMATIONS_PATTERN) {
        let replacement = preferred_lookup(&padded[start..end], BACK_FORMATIONS_REPLACEMENT);

        let span_start = padded[..start].chars().count();
        let span_end = span_start + padded[start..end].chars().count();
        let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
        if quote_index % 2 == 1 {
            continue;
        }
        let (line, column) = line_col_of(span_start, &line_bounds);
        diagnostics.push(json!({
            "check_path": "misc.back_formations",
            "message": format!("Back-formation. '{}' is the preferred form.", replacement.unwrap_or("None")),
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": replacement,
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
    let mut diagnostics = greylist_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return greylist findings in the upstream dictionary traversal order.
pub fn greylist_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for (term, explanation) in GREYLIST_TERMS {
        for (start, end) in unicode_case_match_ends(&padded, term) {
            let span_start = padded[..start].chars().count();
            let span_end = span_start + padded[start..end].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "misc.greylist",
                "message": format!("Use of '{term}'. {explanation}"),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
    }

    diagnostics
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
    for (start, _) in padded.char_indices() {
        let Some(prefix_end) = unicode_case_match_end(&padded, start, BUREAUCRATESE_PREFIX) else {
            continue;
        };
        let middle = if unicode_case_match_end(&padded, prefix_end, "s").is_some() {
            unicode_case_match_end(&padded, prefix_end, "s").unwrap()
        } else {
            prefix_end
        };
        let Some(end) = unicode_case_match_end(&padded, middle, BUREAUCRATESE_SUFFIX) else {
            continue;
        };
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
        let activated = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                is_ascii_word_before(&padded, *start) != is_ascii_word_at(&padded, *start)
                    && is_ascii_word_before(&padded, *end) != is_ascii_word_at(&padded, *end)
            });
        if !activated {
            continue;
        }

        for (start, end) in unicode_case_match_ends(&padded, pattern) {
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
                    "check_path": "hedging",
                    "message": HEDGING_MESSAGE,
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

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
        let activated = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !activated {
            continue;
        }

        for (start, end) in unicode_case_match_ends(&padded, pattern) {
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
                    "check_path": "industrial_language.jargon",
                    "message": JARGON_MESSAGE.replace("{}", matched),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic)
        .collect()
}

/// Return diagnostics for the pinned `oxymorons` Fast `Existence` check.
///
/// Patterns first activate on WORDS_IN_TEXT boundaries, then raw regex
/// occurrences are reported. The upstream `advancing backwards?` syntax is
/// intentionally retained: the question mark quantifies the final `s`.
pub fn oxymorons_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    let mut active_patterns = OXYMORON_ACTIVATION_REGEXES
        .iter()
        .enumerate()
        .filter_map(|(index, regex)| regex.find(&padded).map(|matched| (matched.start(), index)))
        .collect::<Vec<_>>();
    active_patterns.sort_unstable();

    for (_, index) in active_patterns {
        let pattern = OXYMORON_PATTERNS[index];
        let raw_regex = &OXYMORON_REGEXES[index];
        for matched in raw_regex.find_iter(&padded) {
            let surface = matched.as_str();
            let span_start = padded[..matched.start()].chars().count();
            let span_end = span_start + surface.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push((
                span_start,
                pattern,
                json!({
                    "check_path": "oxymorons",
                    "message": OXYMORON_MESSAGE.replace("{}", surface),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": null,
                }),
            ));
        }
    }

    diagnostics
        .into_iter()
        .map(|(_, _, diagnostic)| diagnostic)
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
        let active = AIRLINESE_SUFFIXES.iter().any(|suffix| {
            unicode_case_match_ends(&padded, &format!("{prefix}{suffix}"))
                .iter()
                .any(|(start, end)| {
                    !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
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

    let phrase_active = unicode_case_match_ends(&padded, AIRLINESE_PHRASE)
        .iter()
        .any(|(start, end)| {
            !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
        });
    if phrase_active {
        for (start, end) in unicode_case_match_ends(&padded, AIRLINESE_PHRASE) {
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
    let mut diagnostics = many_a_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return `misc.many_a` findings in FastSet activation order for CLI limits.
pub fn many_a_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in MANY_A_FORMS.iter().enumerate() {
        let pattern_len = pattern.len();
        let pattern_matches = padded
            .char_indices()
            .filter_map(|(start, _)| {
                let candidate = padded.get(start..start.saturating_add(pattern_len))?;
                candidate
                    .eq_ignore_ascii_case(pattern)
                    .then_some((start, start + pattern_len))
            })
            .collect::<Vec<_>>();
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };

        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let candidate = &padded[start..end];
            let span_start = padded[..start].chars().count();
            let span_end = span_start + candidate.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "misc.many_a",
                "message": MANY_A_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": replacement,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the pinned `misc.metadiscourse` existence check.
///
/// Upstream uses a case-insensitive Fast set with WORDS_IN_TEXT padding,
/// including the intentional spelling `preceeding` in its first phrase.
pub fn metadiscourse_diagnostics(text: &str) -> Vec<Value> {
    let mut diagnostics = metadiscourse_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return metadiscourse findings in FastSet activation order for CLI limits.
pub fn metadiscourse_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, pattern) in METADISCOURSE_PATTERNS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };

        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];

            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "misc.metadiscourse",
                "message": METADISCOURSE_MESSAGE,
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": null,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
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
        let activated = unicode_case_match_ends(&padded, term)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !activated {
            continue;
        }

        for (start, end) in unicode_case_match_ends(&padded, term) {
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
        let activated = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !activated {
            continue;
        }

        for (start, end) in unicode_case_match_ends(&padded, pattern) {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);

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
                    "message": LATIN_MESSAGE.replace("{}", preferred.unwrap_or("None")),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": preferred,
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

fn capitalization_diagnostics_in_matcher_order(
    text: &str,
    check_path: &str,
    message: &str,
    forms: &[(&str, &str)],
) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in forms.iter().enumerate() {
        let pattern_len = pattern.len();
        let activation_position = padded
            .match_indices(pattern)
            .filter(|(start, _)| {
                !is_ascii_word_before(&padded, *start)
                    && !is_ascii_word_at(&padded, *start + pattern_len)
            })
            .map(|(start, _)| start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };

        let mut diagnostics = Vec::new();
        for (start, _) in padded.char_indices() {
            let end = start.saturating_add(pattern_len);
            let Some(matched) = padded.get(start..end) else {
                continue;
            };
            if matched != *pattern {
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
                "check_path": check_path,
                "message": message.replace("{}", replacement),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": replacement,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the case-sensitive pinned seasons capitalization
/// preferred forms. Lowercase and other-case variants do not match.
pub fn capitalization_seasons_diagnostics(text: &str) -> Vec<Value> {
    let mut diagnostics = capitalization_seasons_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return season capitalization findings in FastSet activation order for CLI limits.
pub fn capitalization_seasons_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    capitalization_diagnostics_in_matcher_order(
        text,
        "misc.capitalization.seasons",
        CAPITALIZATION_SEASONS_MESSAGE,
        &CAPITALIZATION_SEASONS,
    )
}

/// Return diagnostics for the case-sensitive pinned capitalization terms.
pub fn capitalization_terms_diagnostics(text: &str) -> Vec<Value> {
    let mut diagnostics = capitalization_terms_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return term capitalization findings in FastSet activation order for CLI limits.
pub fn capitalization_terms_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    capitalization_diagnostics_in_matcher_order(
        text,
        "misc.capitalization.terms",
        CAPITALIZATION_TERMS_MESSAGE,
        &CAPITALIZATION_TERMS,
    )
}

/// Return diagnostics for the case-sensitive pinned capitalization days rule.
/// PreferredFormsSimple activates each item at a word boundary, then reports
/// every raw case-sensitive occurrence of that item in the padded text.
pub fn capitalization_days_diagnostics(text: &str) -> Vec<Value> {
    let mut diagnostics = capitalization_days_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return day capitalization findings in FastSet activation order for CLI limits.
pub fn capitalization_days_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    capitalization_diagnostics_in_matcher_order(
        text,
        "misc.capitalization.days",
        CAPITALIZATION_DAYS_MESSAGE,
        &CAPITALIZATION_DAYS,
    )
}

/// Return diagnostics for the pinned case-sensitive month capitalization rule.
pub fn capitalization_months_diagnostics(text: &str) -> Vec<Value> {
    let mut diagnostics = capitalization_months_diagnostics_in_matcher_order(text);
    diagnostics
        .sort_unstable_by_key(|diagnostic| diagnostic["span"][0].as_u64().unwrap_or_default());
    diagnostics
}

/// Return month capitalization findings in FastSet activation order for CLI limits.
pub fn capitalization_months_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    capitalization_diagnostics_in_matcher_order(
        text,
        "misc.capitalization.months",
        CAPITALIZATION_MONTHS_MESSAGE,
        &CAPITALIZATION_MONTHS,
    )
}

/// Return diagnostics for the pinned `psychology.lie_detector` preferred forms.
/// Word boundaries use RE2's default ASCII word class.
pub fn lie_detector_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for (pattern, replacement) in LIE_DETECTOR_FORMS {
        let activated = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !activated {
            continue;
        }

        for (start, end) in unicode_case_match_ends(&padded, pattern) {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);

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
                    "check_path": "psychology.lie_detector",
                    "message": LIE_DETECTOR_MESSAGE.replace("{}", preferred.unwrap_or("None")),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": preferred,
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

/// Return diagnostics for the pinned `redundancy.misc.wallace` preferred forms.
pub fn wallace_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    for (pattern, replacement) in WALLACE_FORMS {
        let activated = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !activated {
            continue;
        }

        for (start, end) in unicode_case_match_ends(&padded, pattern) {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);

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
                    "check_path": "redundancy.misc.wallace",
                    "message": WALLACE_MESSAGE
                        .replacen("{}", preferred.unwrap_or("None"), 1)
                        .replacen("{}", matched, 1),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": preferred,
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

/// Return diagnostics for the pinned `redundancy.misc.nordquist` rule.
pub fn nordquist_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    let padded = format!("\n{text}\n");
    let quotes = paired_quote_bounds(&padded);
    let lines = splitlines_bounds(&padded);
    let mut activated = Vec::new();
    for (index, pattern) in NORDQUIST_FORMS.into_iter().enumerate() {
        let matches = unicode_case_match_ends(&padded, pattern);
        let Some(first) = matches
            .iter()
            .find(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
        else {
            continue;
        };
        activated.push((first, index, matches));
    }
    activated.sort_unstable_by_key(|(first, index, _)| (*first, *index));
    let mut diagnostics = Vec::new();
    for (_, _, matches) in activated {
        for (start, end) in matches {
            let matched = &padded[start..end];
            let replacement = match matched.to_lowercase().as_str() {
                "absolutely essential" => Some("essential"),
                "absolutely necessary" => Some("necessary"),
                _ => None,
            };
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            if quotes.partition_point(|bound| *bound < span_start) % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &lines);
            diagnostics.push((span_start, json!({
                "check_path":"redundancy.misc.nordquist",
                "message":WALLACE_MESSAGE.replacen("{}", replacement.unwrap_or("None"), 1).replacen("{}", matched, 1),
                "span":[span_start,span_end], "pos":[line,column], "replacements":replacement
            })));
        }
    }
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
    let active = unicode_case_match_ends(&padded, INSTITUTION_NAME_PATTERN)
        .iter()
        .any(|(start, end)| {
            !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
        });
    if !active {
        return diagnostics;
    }

    for (start, end) in unicode_case_match_ends(&padded, INSTITUTION_NAME_PATTERN) {
        let matched = &padded[start..end];
        let preferred = preferred_lookup(matched, INSTITUTION_NAME_REPLACEMENT);

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
                preferred.unwrap_or("None"),
                matched
            ),
            "span": [span_start, span_end],
            "pos": [line, column],
            "replacements": preferred,
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
        let active = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !active {
            continue;
        }
        for (start, end) in unicode_case_match_ends(&padded, pattern) {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);

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
                    "message": format!("'{}' is the name of that job.", preferred.unwrap_or("None")),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": preferred,
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

/// Return diagnostics for the pinned `terms.eponymous_adjectives` rule.
pub fn eponymous_adjectives_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (pattern, replacement) in EPONYMOUS_ADJECTIVES {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        if !pattern_matches.iter().any(|(start, end)| {
            !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
        }) {
            continue;
        }
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "terms.eponymous_adjectives",
                    "message": format!("'{}' is the preferred eponymous adjective.", preferred.unwrap_or("None")),
                    "span": [span_start, span_end],
                    "pos": [line, column],
                    "replacements": preferred,
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

/// Return diagnostics for the pinned `spelling.ance_ence` preferred forms.
/// Each term activates at an ASCII word boundary; once activated, all raw
/// case-insensitive occurrences of that term are reported.
pub fn spelling_ance_ence_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut matches = Vec::new();

    for (pattern, replacement) in ANCE_ENCE_FORMS {
        let active = unicode_case_match_ends(&padded, pattern)
            .iter()
            .any(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            });
        if !active {
            continue;
        }
        for (start, end) in unicode_case_match_ends(&padded, pattern) {
            let matched = &padded[start..end];
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            matches.push((
                span_start,
                json!({
                    "check_path": "spelling.ance_ence",
                    "message": format!("-ance vs. -ence. '{}' is the correct spelling.", replacement),
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

/// Return diagnostics for the pinned `spelling.ally_ly` preferred forms.
/// Each spelling activates at an ASCII word boundary; after activation, all
/// case-insensitive raw occurrences of that spelling are reported.
pub fn spelling_ally_ly_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in ALLY_LY_FORMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "spelling.ally_ly",
                "message": format!("-ally vs. -ly. '{}' is the correct spelling.", preferred.unwrap_or("None")),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    // FastSet orders selected patterns by their earliest eligible boundary
    // activation, then emits every raw occurrence for each selected pattern.
    // The caller applies max_errors before its final document-position sort.
    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the pinned `spelling.ely_ly` preferred forms.
/// Each spelling activates at an ASCII word boundary; after activation, all
/// case-insensitive raw occurrences of that spelling are reported.
pub fn spelling_ely_ly_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in ELY_LY_FORMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "spelling.ely_ly",
                "message": format!("-ely vs. -ly. '{}' is the correct spelling.", preferred.unwrap_or("None")),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    // Preserve FastSet activation order until the caller applies max_errors.
    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the pinned `spelling.in_un` preferred forms.
pub fn spelling_in_un_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in IN_UN_FORMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "spelling.in_un",
                "message": format!("in- vs. un-. '{}' is the preferred spelling.", preferred.unwrap_or("None")),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    // FastSet activation order is observable before max_errors truncation.
    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the pinned `mondegreens` preferred forms.
pub fn mondegreens_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in MONDEGREENS_FORMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "mondegreens",
                "message": format!("'{}' is the preferred form.", preferred.unwrap_or("None")),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    // FastSet orders activated patterns before the caller applies max_errors.
    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return diagnostics for the pinned `spelling.ve_of` preferred forms.
/// Activation requires word characters at both ends of the raw match.
pub fn spelling_ve_of_diagnostics(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut active_patterns = Vec::new();

    for (pattern_index, (pattern, replacement)) in VE_OF_FORMS.iter().enumerate() {
        let pattern_matches = unicode_case_match_ends(&padded, pattern);
        let activation_position = pattern_matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start)
                    && is_ascii_word_at(&padded, *start)
                    && is_ascii_word_before(&padded, *end)
                    && !is_ascii_word_at(&padded, *end)
            })
            .map(|(start, _)| *start)
            .min();
        let Some(activation_position) = activation_position else {
            continue;
        };
        let mut diagnostics = Vec::new();
        for (start, end) in pattern_matches {
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched, replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "spelling.ve_of",
                "message": format!("-ve vs. -of. '{}' is the preferred spelling.", preferred.unwrap_or("None")),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
        active_patterns.push((activation_position, pattern_index, diagnostics));
    }

    // Preserve FastSet activation order until the caller applies max_errors.
    active_patterns.sort_by_key(|(activation_position, pattern_index, _)| {
        (*activation_position, *pattern_index)
    });
    active_patterns
        .into_iter()
        .flat_map(|(_, _, diagnostics)| diagnostics)
        .collect()
}

/// Return composition findings in upstream registration and matcher order.
pub fn composition_diagnostics_in_matcher_order(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }

    let padded = format!("\n{text}\n");
    let quote_bounds = paired_quote_bounds(&padded);
    let line_bounds = splitlines_bounds(&padded);
    let mut diagnostics = Vec::new();

    let mut simple_matches = Vec::new();
    for (pattern_index, (pattern, replacement)) in COMPOSITION_SIMPLE_FORMS.iter().enumerate() {
        let matches = unicode_case_match_ends(&padded, pattern);
        if let Some((start, end)) = matches
            .iter()
            .filter(|(start, end)| {
                !is_ascii_word_before(&padded, *start) && !is_ascii_word_at(&padded, *end)
            })
            .min_by_key(|(start, end)| (*end, std::cmp::Reverse(*start)))
        {
            simple_matches.push((*end, *start, pattern_index, *replacement, matches));
        }
    }
    simple_matches.sort_by_key(|(end, start, pattern_index, ..)| {
        (*end, std::cmp::Reverse(*start), *pattern_index)
    });

    for (_, _, _, replacement, matches) in simple_matches {
        let mut previous_end = 0;
        for (start, end) in matches {
            if start < previous_end {
                continue;
            }
            previous_end = end;
            let matched = &padded[start..end];
            let preferred = preferred_lookup(matched.trim(), replacement);
            let span_start = padded[..start].chars().count();
            let span_end = span_start + matched.chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "misc.composition",
                "message": format!("Try '{}' instead of '{}'.", preferred.unwrap_or("None"), matched.trim()),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
    }

    for ((_, replacement), regex) in COMPOSITION_REGEX_FORMS
        .iter()
        .zip(COMPOSITION_REGEXES.iter())
    {
        for matched in regex.find_iter(&padded) {
            let span_start = padded[..matched.start()].chars().count();
            let span_end = span_start + padded[matched.start()..matched.end()].chars().count();
            let quote_index = quote_bounds.partition_point(|bound| *bound < span_start);
            if quote_index % 2 == 1 {
                continue;
            }
            let (line, column) = line_col_of(span_start, &line_bounds);
            diagnostics.push(json!({
                "check_path": "misc.composition",
                "message": format!("Try '{replacement}' instead of '{}'.", matched.as_str().trim()),
                "span": [span_start, span_end],
                "pos": [line, column],
                "replacements": replacement,
            }));
        }
    }

    diagnostics
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

fn unicode_case_match_end(text: &str, start: usize, pattern: &str) -> Option<usize> {
    let target: Vec<char> = unicode_case_fold(pattern).chars().collect();
    let mut folded = Vec::with_capacity(target.len());
    let mut end = start;
    for ch in text.get(start..)?.chars() {
        folded.extend(unicode_case_fold(&ch.to_string()).chars());
        end += ch.len_utf8();
        if folded.len() >= target.len() {
            return (folded == target).then_some(end);
        }
        if !target.starts_with(&folded) {
            return None;
        }
    }
    None
}

pub(crate) fn unicode_case_match_ends(text: &str, pattern: &str) -> Vec<(usize, usize)> {
    text.char_indices()
        .filter_map(|(start, _)| {
            unicode_case_match_end(text, start, pattern).map(|end| (start, end))
        })
        .collect()
}

fn unicode_case_fold(text: &str) -> String {
    text.chars()
        .flat_map(|ch| match ch {
            '\u{017f}' => vec!['s'], // LATIN SMALL LETTER LONG S
            '\u{212a}' => vec!['k'], // KELVIN SIGN
            _ => ch.to_lowercase().collect(),
        })
        .collect()
}

pub(crate) fn preferred_lookup(surface: &str, replacement: &'static str) -> Option<&'static str> {
    (!surface.contains('ſ')).then_some(replacement)
}

pub(crate) fn paired_quote_bounds(text: &str) -> Vec<usize> {
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

pub(crate) fn quotes_match(open: char, close: char) -> bool {
    (matches!(open, '\'' | '"') && open == close)
        || (matches!(
            (open, close),
            ('“', '”') | ('”', '“') | ('‘', '’') | ('’', '‘')
        ))
}

#[cfg(test)]
mod oxymorons_tests {
    use super::oxymorons_diagnostics;
    use serde_json::json;

    #[test]
    fn reports_all_registered_phrases_with_surface_messages() {
        let text = "amateur expert | increasingly less | advancing backward | alludes explicitly to | explicitly alludes to | totally obsolescent | completely obsolescent | generally always | usually always | build down | conspicuous absence | exact estimate | found missing | intense apathy | mandatory choice | nonworking mother | organized mess";
        let diagnostics = oxymorons_diagnostics(text);
        assert_eq!(diagnostics.len(), 17);
        assert_eq!(
            diagnostics[0]["message"],
            "'amateur expert' is an oxymoron."
        );
        assert_eq!(
            diagnostics[2]["message"],
            "'advancing backward' is an oxymoron."
        );
        assert_eq!(
            diagnostics[16]["message"],
            "'organized mess' is an oxymoron."
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| d["replacements"] == serde_json::Value::Null)
        );
    }

    #[test]
    fn question_mark_in_source_pattern_is_an_optional_s() {
        let diagnostics = oxymorons_diagnostics("advancing backward; advancing backwards?");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0]["message"],
            "'advancing backward' is an oxymoron."
        );
        assert_eq!(
            diagnostics[1]["message"],
            "'advancing backwards' is an oxymoron."
        );
        assert_eq!(diagnostics[1]["span"], json!([21, 40]));
    }

    #[test]
    fn raw_prefix_is_only_emitted_after_boundary_activation() {
        assert!(oxymorons_diagnostics("advancing backwardness").is_empty());
        let diagnostics = oxymorons_diagnostics("advancing backwardness; advancing backward");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0]["span"], json!([1, 19]));
        assert_eq!(diagnostics[1]["span"], json!([25, 43]));
    }

    #[test]
    fn quotes_are_suppressed_and_unicode_positions_are_codepoints() {
        assert!(oxymorons_diagnostics("\"amateur expert\"").is_empty());
        let diagnostics = oxymorons_diagnostics("é amateur expert");
        assert_eq!(diagnostics[0]["span"], json!([3, 17]));
        assert_eq!(diagnostics[0]["pos"], json!([1, 3]));
    }

    #[test]
    fn unicode_letter_before_phrase_is_ascii_word_boundary() {
        let diagnostics = oxymorons_diagnostics("éamateur expert");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0]["message"],
            "'amateur expert' is an oxymoron."
        );
    }
}

#[cfg(test)]
mod spelling_in_un_tests {
    use super::spelling_in_un_diagnostics;
    use serde_json::json;

    #[test]
    fn maps_all_four_forms_with_codepoint_positions() {
        let diagnostics =
            spelling_in_un_diagnostics("unadvisable unalienable unexpressive unfeasible");
        assert_eq!(
            diagnostics,
            vec![
                json!({"check_path":"spelling.in_un","message":"in- vs. un-. 'inadvisable' is the preferred spelling.","span":[1,12],"pos":[1,1],"replacements":"inadvisable"}),
                json!({"check_path":"spelling.in_un","message":"in- vs. un-. 'inalienable' is the preferred spelling.","span":[13,24],"pos":[1,13],"replacements":"inalienable"}),
                json!({"check_path":"spelling.in_un","message":"in- vs. un-. 'inexpressive' is the preferred spelling.","span":[25,37],"pos":[1,25],"replacements":"inexpressive"}),
                json!({"check_path":"spelling.in_un","message":"in- vs. un-. 'infeasible' is the preferred spelling.","span":[38,48],"pos":[1,38],"replacements":"infeasible"}),
            ]
        );
    }

    #[test]
    fn embedded_occurrence_is_reported_after_boundary_activation() {
        let diagnostics = spelling_in_un_diagnostics("xunadvisable unadvisable");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0]["span"], json!([2, 13]));
        assert_eq!(diagnostics[1]["span"], json!([14, 25]));
    }

    #[test]
    fn long_s_matches_but_exact_lookup_returns_none() {
        let diagnostics = spelling_in_un_diagnostics("unexpreſſive unfeaſible");
        assert_eq!(diagnostics.len(), 2);
        for diagnostic in diagnostics {
            assert_eq!(
                diagnostic["message"],
                "in- vs. un-. 'None' is the preferred spelling."
            );
            assert_eq!(diagnostic["replacements"], serde_json::Value::Null);
        }
    }

    #[test]
    fn quoted_activation_is_suppressed_before_caller_truncation() {
        let diagnostics = spelling_in_un_diagnostics("\"unadvisable\" unfeasible xunadvisable");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0]["span"], json!([27, 38]));
        assert_eq!(diagnostics[1]["span"], json!([15, 25]));
    }
}

#[cfg(test)]
mod mondegreens_tests {
    use super::mondegreens_diagnostics;
    use serde_json::json;

    #[test]
    fn maps_all_eight_phrases_with_exact_messages_and_spans() {
        let diagnostics = mondegreens_diagnostics(
            "a girl with colitis goes by | a part-red gingerbread tree | attorney and not a republic | beckon call | for all intensive purposes | lady mondegreen | olive, the other reindeer | to the manor born",
        );
        assert_eq!(diagnostics.len(), 8);
        assert_eq!(
            diagnostics[0]["message"],
            "'a girl with kaleidoscope eyes' is the preferred form."
        );
        assert_eq!(diagnostics[0]["span"], json!([1, 28]));
        assert_eq!(
            diagnostics[1]["message"],
            "'a partridge in a pear tree' is the preferred form."
        );
        assert_eq!(
            diagnostics[2]["message"],
            "'attorney and notary public' is the preferred form."
        );
        assert_eq!(
            diagnostics[3]["message"],
            "'beck and call' is the preferred form."
        );
        assert_eq!(
            diagnostics[4]["message"],
            "'for all intents and purposes' is the preferred form."
        );
        assert_eq!(
            diagnostics[5]["message"],
            "'laid him on the green' is the preferred form."
        );
        assert_eq!(
            diagnostics[6]["message"],
            "'all of the other reindeer' is the preferred form."
        );
        assert_eq!(
            diagnostics[7]["message"],
            "'to the manner born' is the preferred form."
        );
    }

    #[test]
    fn matching_is_case_insensitive_and_preserves_repetitions() {
        let diagnostics = mondegreens_diagnostics("BECKON CALL, beckon call");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0]["message"],
            "'beck and call' is the preferred form."
        );
        assert_eq!(diagnostics[0]["span"], json!([1, 12]));
        assert_eq!(diagnostics[1]["span"], json!([14, 25]));
    }

    #[test]
    fn quotes_and_outer_word_boundaries_match_the_live_contract() {
        assert!(mondegreens_diagnostics("xbeckon call beckon caller").is_empty());
        let diagnostics = mondegreens_diagnostics("\"beckon call\" and beckon call");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["span"], json!([19, 30]));
        let comma = mondegreens_diagnostics("(olive, the other reindeer!)");
        assert_eq!(comma.len(), 1);
        assert_eq!(
            comma[0]["message"],
            "'all of the other reindeer' is the preferred form."
        );
        assert_eq!(comma[0]["span"], json!([2, 27]));
    }

    #[test]
    fn pattern_activation_order_is_preserved_before_max_errors() {
        let diagnostics = mondegreens_diagnostics("to the manor born; a girl with colitis goes by");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0]["message"],
            "'to the manner born' is the preferred form."
        );
        assert_eq!(diagnostics[0]["span"], json!([1, 18]));
        assert_eq!(diagnostics[1]["span"], json!([20, 47]));
    }
}
