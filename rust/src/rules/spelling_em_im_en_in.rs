use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::sync::LazyLock;

// From proselint 0.16.0, checks/spelling/em_im_en_in.py at dbed789caae662d06c7c8a5a13dd31f1acd36f5c.
const FORMS: [(&str, &str); 49] = [
    ("imbalm", "embalm"),
    ("imbark", "embark"),
    ("imbed", "embed"),
    ("imbitter", "embitter"),
    ("imblaze", "emblaze"),
    ("imbody", "embody"),
    ("imbolden", "embolden"),
    ("imbosom", "embosom"),
    ("imbower", "embower"),
    ("imbrown", "embrown"),
    ("impanel", "empanel"),
    ("impower", "empower"),
    ("incage", "encage"),
    ("incapsulate", "encapsulate"),
    ("incase", "encase"),
    ("inclasp", "enclasp"),
    ("incumber", "encumber"),
    ("incumbrance", "encumbrance"),
    ("indow", "endow"),
    ("indowment", "endowment"),
    ("indue", "endue"),
    ("infold", "enfold"),
    ("ingraft", "engraft"),
    ("ingulf", "engulf"),
    ("inlace", "enlace"),
    ("inmesh", "enmesh"),
    ("insheathe", "ensheathe"),
    ("inshrine", "enshrine"),
    ("insnare", "ensnare"),
    ("insoul", "ensoul"),
    ("insphere", "ensphere"),
    ("inthrall", "enthrall"),
    ("inthrone", "enthrone"),
    ("intitle", "entitle"),
    ("intomb", "entomb"),
    ("intreat", "entreat"),
    ("intrench", "entrench"),
    ("intrust", "entrust"),
    ("intwine", "entwine"),
    ("intwist", "entwist"),
    ("inwind", "enwind"),
    ("inwrap", "enwrap"),
    ("inwreathe", "enwreathe"),
    ("embrue", "imbrue"),
    ("empale", "impale"),
    ("empoverish", "impoverish"),
    ("enflame", "inflame"),
    ("engrain", "ingrain"),
    ("enure", "inure"),
];

static ACTIVATION: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(form, _)| {
            RegexBuilder::new(&format!(r"(?i)(?-u:\b){form}(?-u:\b)"))
                .build()
                .expect("valid em/im/en/in activation form")
        })
        .collect()
});
static RAW: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    FORMS
        .iter()
        .map(|(form, _)| {
            RegexBuilder::new(&format!(r"(?i){form}"))
                .build()
                .expect("valid em/im/en/in form")
        })
        .collect()
});

/// Return diagnostics for the pinned spelling.em_im_en_in check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let lines = splitlines_bounds(&padded);
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let mut active = ACTIVATION
        .iter()
        .enumerate()
        .filter_map(|(index, pattern)| {
            pattern
                .find(&padded)
                .map(|matched| (matched.start(), index))
        })
        .collect::<Vec<_>>();
    active.sort_unstable();

    let mut diagnostics = Vec::new();
    for (_, index) in active {
        let (original, replacement) = FORMS[index];
        for matched in RAW[index].find_iter(&padded) {
            let start = padded[..matched.start()].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            let surface = matched.as_str();
            let end = start + surface.chars().count();
            let preferred = (surface.to_lowercase() == original).then_some(replacement);
            let (line, column) = line_col_of(start, &lines);
            diagnostics.push(json!({
                "check_path": "spelling.em_im_en_in",
                "message": format!("em-, im-, en-, and in-. '{}' is the preferred spelling.", preferred.unwrap_or("None")),
                "span": [start, end],
                "pos": [line, column],
                "replacements": preferred,
            }));
        }
    }
    diagnostics
}

#[cfg(test)]
pub fn form_text_for_test() -> String {
    FORMS
        .iter()
        .map(|(form, _)| *form)
        .collect::<Vec<_>>()
        .join(" ")
}

fn splitlines_bounds(text: &str) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut bounds = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let end = match chars[index] {
            '\r' if chars.get(index + 1) == Some(&'\n') => index + 2,
            '\r' | '\n' | '\u{000b}' | '\u{000c}' | '\u{001c}' | '\u{001d}' | '\u{001e}'
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
