use regex::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// Garner's Modern American Usage; pinned source SHA-256 fefea51f8bf915b9ef8c4463d54e8fe83ea29d167d42410490a217167c526e2b.
const FORMS: [(&str, &str); 136] = [
    ("imprimature", "imprimatur"),
    ("hallowe'en", "Halloween"),
    ("haloween", "Halloween"),
    ("khruschev", "Khrushchev"),
    ("klu klux klan", "Ku Klux Klan"),
    ("kruschev", "Khrushchev"),
    ("pontius pilot", "Pontius Pilate"),
    ("hippopotami", "hippopotamuses"),
    ("longstanding", "long-standing"),
    ("manifesti", "manifestos"),
    ("matrixes", "matrices"),
    ("mongeese", "mongooses"),
    ("narcissuses", "narcissi"),
    ("non-sequitur", "non sequitur"),
    ("retinae", "retinas"),
    ("sans-serif", "sans serif"),
    ("sanserif", "sans serif"),
    ("soprani", "sopranos"),
    ("titmouses", "titmice"),
    ("tort feasor", "tortfeasor"),
    ("tort-feasor", "tortfeasor"),
    ("trans-ship", "transship"),
    ("trans-shipped", "transshipped"),
    ("trans-shipping", "transshipping"),
    ("tranship", "transship"),
    ("transhipped", "transshiped"),
    ("transhipping", "transshipping"),
    ("mental attitude", "attitude"),
    (
        "chief justice of the united states supreme court",
        "Chief Justice of the United States",
    ),
    (
        "chief justice of the supreme court of the united states",
        "Chief Justice of the United States",
    ),
    ("chitlings", "chitterlings"),
    ("chitlins", "chitterlings"),
    ("combustible engine", "combustion engine"),
    ("for the duration of", "during / throughout"),
    ("foreclose againt", "foreclose on"),
    ("mutual friend", "friend in common"),
    ("in regards to", "in regard to"),
    ("infectuous", "infectious"),
    ("inferrable", "inferable"),
    ("inferrible", "inferable"),
    ("in light of the fact that", "knowing that"),
    ("laniard", "lanyard"),
    ("largesse", "largess"),
    ("lasagne", "lasagna"),
    ("leary", "leery"),
    ("loan me her", "lend me her"),
    ("loan me his", "lend me his"),
    ("loan me their", "lend me their"),
    ("loan me your", "lend me your"),
    ("loaned me her", "lent me her"),
    ("loaned me his", "lent me his"),
    ("loaned me their", "lent me their"),
    ("loaned me your", "lent me your"),
    ("linguistician", "linguist"),
    ("matzoh-ball", "matzo-ball"),
    ("matza-ball", "matzo-ball"),
    ("matzah-ball", "matzo-ball"),
    ("matsah-ball", "matzo-ball"),
    ("mayorality", "mayoralty"),
    ("mealymouthed", "mealy-mouthed"),
    ("meanspirited", "mean-spirited"),
    ("midwived", "midwifed"),
    ("monicker", "moniker"),
    ("musical review", "musical revue"),
    ("moustache", "mustache"),
    ("nonplused", "nonplussed"),
    ("nonplusing", "nonplussing"),
    ("nonsequitur", "non sequitur"),
    ("nowhere near as", "not nearly as"),
    ("off of", "off"),
    ("chiropodist", "podiatrist"),
    ("chiropody", "podiatry"),
    ("shoe-in", "shoo-in"),
    ("suicide victim", "suicide"),
    ("the highway medium", "the highway median"),
    ("vapidness", "vaipidity"),
    ("weather vein", "weather vane"),
    ("weather vain", "weather vane"),
    ("with regards to", "with regard to"),
    ("a couple people", "a couple of people"),
    ("all of the time", "all the time"),
    ("as follow", "as follows"),
    ("bulk largely", "bulk large"),
    ("burying the lead", "burying the lede"),
    ("came to nought", "came to naught"),
    ("come off of it", "come off it"),
    ("corroborative evidence", "corroborating evidence"),
    ("dearly departed", "dear departed"),
    ("default to a loan", "default on a loan"),
    ("make an inference", "draw an inference"),
    ("in the meanwhile", "in the meantime"),
    ("lengthy distances", "long distances"),
    ("maddening crowd", "madding crowd"),
    ("magna charta", "Magna Carta"),
    ("mariage de convenance", "marriage of convenience"),
    ("middle west", "Midwest"),
    ("middle western", "Midwestern"),
    ("modes of operandi", "modi operandi"),
    ("mode of operandi", "modus operandi"),
    ("notion seconded", "motion seconded"),
    ("mucus membranes", "mucous membranes"),
    ("must past muster", "must pass muster"),
    ("neck-in-neck", "neck-and-neck"),
    ("no-holes-barred", "no-holds-barred"),
    ("oil magnet", "oil magnate"),
    ("punch up the lead", "punch up the lede"),
    ("railroad magnet", "railroad magnate"),
    ("seconded the notion", "seconded the motion"),
    ("statute of limits", "statute of limitationas"),
    ("take prescience over", "take precedence over"),
    ("both of the last two", "the last two"),
    ("both of the last", "the last two"),
    ("inorganic food", "unorganic food"),
    ("veil of tears", "vale of tears"),
    ("venus's flytrap", "Venus flytrap"),
    ("venus' flytrap", "Venus flytrap"),
    ("was accused with", "was accused of"),
    ("make an attempt to", "try to"),
    ("make attempts to", "try to"),
    ("make efforts to", "try to"),
    ("made an attempt to", "tried to"),
    ("made attempts to", "tried to"),
    ("made efforts to", "tried to"),
    ("modern-day", "modern"),
    ("be mislead", "be misled"),
    ("was mislead", "was misled"),
    ("were mislead", "were misled"),
    ("armed reconnaissance", "a search-and-destroy mission"),
    ("pregnancy termination", "abortion"),
    ("sexually ambidextrous", "bisexual"),
    ("extermination engineer", "exterminator"),
    ("permanent layoff", "firing"),
    ("rodent operative", "rat-catcher"),
    ("mistaked", "mistook"),
    ("ne", "né"),
    ("nee", "née"),
];
static PATTERN_SET: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSetBuilder::new(FORMS.iter().map(|(term, _)| regex::escape(term)))
        .case_insensitive(true)
        .build()
        .unwrap()
});
static MEANTIME: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"\bMeantime,\B")
        .case_insensitive(false)
        .build()
        .unwrap()
});
static MEANTIME_CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"\b[;,] meantime,\B")
        .case_insensitive(false)
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
    let mut candidates = Vec::new();
    if !padded.chars().any(has_non_ascii_alphabetic) {
        let bytes = padded.as_bytes();
        for (index, (term, _)) in FORMS.iter().enumerate() {
            let needle = term.as_bytes();
            if needle.len() > bytes.len() {
                continue;
            }
            for start in 0..=bytes.len() - needle.len() {
                let end = start + needle.len();
                if !bytes[start..end].eq_ignore_ascii_case(needle) {
                    continue;
                }
                let before = start.checked_sub(1).map(|i| bytes[i]);
                let after = bytes.get(end).copied();
                if before.is_some_and(is_strict_word) || after.is_some_and(is_strict_word) {
                    continue;
                }
                candidates.push((start, index, end, padded[start..end].to_owned()));
            }
        }
    } else {
        for index in PATTERN_SET.matches(&padded).iter() {
            let pattern = RegexBuilder::new(&regex::escape(FORMS[index].0))
                .case_insensitive(true)
                .build()
                .unwrap();
            for matched in pattern.find_iter(&padded) {
                let before = padded
                    .as_bytes()
                    .get(matched.start().wrapping_sub(1))
                    .copied();
                let after = padded.as_bytes().get(matched.end()).copied();
                if before.is_some_and(is_strict_word) || after.is_some_and(is_strict_word) {
                    continue;
                }
                candidates.push((
                    matched.start(),
                    index,
                    matched.end(),
                    matched.as_str().to_owned(),
                ));
            }
        }
    }
    candidates.sort_by_key(|(start, index, _, _)| (*start, *index));
    let mut cursor = 0;
    let mut found = Vec::new();
    for (start, _, end, original) in candidates {
        if start < cursor {
            continue;
        }
        cursor = end;
        let char_start = padded[..start].chars().count();
        let char_end = padded[..end].chars().count();
        if quotes.partition_point(|bound| *bound < char_start) % 2 == 1 {
            continue;
        }
        let replacement = FORMS
            .iter()
            .find(|(term, _)| *term == original.to_lowercase())
            .map(|(_, replacement)| *replacement);
        push(
            &mut found,
            char_start,
            char_end,
            crate::line_col_of(char_start, &lines),
            replacement,
        );
    }

    for matched in MEANTIME.find_iter(&padded) {
        let start = padded[..matched.start()].chars().count();
        let end = padded[..matched.end()].chars().count();
        if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
            continue;
        }
        push(
            &mut found,
            start,
            end,
            crate::line_col_of(start, &lines),
            Some("Meanwhile,"),
        );
    }
    for matched in MEANTIME_CLAUSE.find_iter(&padded) {
        let start_byte = matched.start() + 2;
        let start = padded[..start_byte].chars().count();
        let end = padded[..matched.end()].chars().count();
        if quotes.partition_point(|bound| *bound < start) % 2 == 1 {
            continue;
        }
        push(
            &mut found,
            start,
            end,
            crate::line_col_of(start, &lines),
            Some("meanwhile,"),
        );
    }

    found.sort_by_key(|(start, order, _)| (*start, *order));
    found.into_iter().map(|(_, _, value)| value).collect()
}

fn is_strict_word(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'\'' || byte == b'-'
}

fn has_non_ascii_alphabetic(ch: char) -> bool {
    !ch.is_ascii() && ch.is_alphabetic()
}

fn push(
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
            "check_path": "misc.preferred_forms",
            "message": format!("'{}' is the preferred form.", replacement.unwrap_or("None")),
            "span": [start, end],
            "pos": [line, column],
            "replacements": replacement,
        }),
    ));
}
