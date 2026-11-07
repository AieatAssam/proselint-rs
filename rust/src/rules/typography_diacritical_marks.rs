use serde_json::{Value, json};

// Copied in declaration order from Proselint 0.16.0 at dbed789caae662d06c7c8a5a13dd31f1acd36f5c.
const FORMS: [(&str, &str); 80] = [
    ("cafe", "café"),
    ("cliche", "cliché"),
    ("crepe", "crêpe"),
    ("decor", "décor"),
    ("facade", "façade"),
    ("fiance", "fiancé"),
    ("flambe", "flambé"),
    ("garcon", "garçon"),
    ("protege", "protégé"),
    ("puree", "purée"),
    ("risque", "risqué"),
    ("roue", "roué"),
    ("soiree", "soirée"),
    ("souffle", "soufflé"),
    ("touche", "touché"),
    ("voila", "voilà"),
    ("resume", "résumé"),
    ("chevre", "chèvre"),
    ("consomme", "consommé"),
    ("creme fraice", "crème fraîche"),
    ("creme fresh", "crème fraîche"),
    ("creme brulee", "crème brûlée"),
    ("creme de menthe", "crème de menthe"),
    ("debutante", "débutante"),
    ("deja vu", "déjà vu"),
    ("denouement", "dénouement"),
    ("fiancee", "fiancée"),
    ("lycee", "lycée"),
    ("maitre d", "maître d"),
    ("negligee", "négligée"),
    ("papier-mache", "papier-mâché"),
    ("paper mache", "papier-mâché"),
    ("paper-mache", "papier-mâché"),
    ("protegee", "protégée"),
    ("raison d'etre", "raison d'être"),
    ("soupcon", "soupçon"),
    ("tete-a-tete", "tête-à-tête"),
    ("a la carte", "à la carte"),
    ("a la mode", "à la mode"),
    ("emigre", "émigré"),
    ("vis-a-vis", "vis-à-vis"),
    ("cause celebre", "cause célèbre"),
    ("coup d'etat", "coup d'état"),
    ("coup de grace", "coup de grâce"),
    ("crudites", "crudités"),
    ("bric-a-brac", "bric-à-brac"),
    ("beau ideal", "beau idéal"),
    ("comme ci comme ca", "comme ci comme ça"),
    ("comsi comsa", "comme ci comme ça"),
    ("menage a trois", "ménage à trois"),
    ("senor", "señor"),
    ("senora", "señora"),
    ("el nino", "El Niño"),
    ("la nina", "La Niña"),
    ("senorita", "señorita"),
    ("jalapeno", "jalapeño"),
    ("pina colada", "piña colada"),
    ("acai", "açaí"),
    ("fuhrer", "Führer"),
    ("ubermensch", "Übermensch"),
    ("doppelganger", "doppelgänger"),
    ("gewurztraminer", "Gewürztraminer"),
    ("filmjolk", "filmjölk"),
    ("smorgasbord", "smörgåsbord"),
    ("skoda", "Škoda"),
    ("quebec", "Québec"),
    ("bronte", "Brontë"),
    ("beyonce", "Beyoncé"),
    ("curacao", "Curaçao"),
    ("nescafe", "Nescafé"),
    ("citroen", "Citroën"),
    ("angstrom", "Ångström"),
    ("quebecois", "Québécois"),
    ("lowenbrau", "Löwenbräu"),
    ("monegasque", "Monégasque"),
    ("motley crue", "Mötley Crüe"),
    ("haagen-dazs", "Häagen-Dazs"),
    ("haagen dazs", "Häagen-Dazs"),
    ("queensryche", "Queensrÿche"),
    ("champs-elysees", "Champs-Élysées"),
];

/// Return diagnostics for the pinned `typography.diacritical_marks` check.
pub fn diagnostics(text: &str) -> Vec<Value> {
    let padded = format!("\n{text}\n");
    let lines = crate::splitlines_bounds(&padded);
    let mut active = FORMS
        .iter()
        .enumerate()
        .filter_map(|(i, (form, _))| {
            crate::unicode_case_match_ends(&padded, form)
                .into_iter()
                .find(|(start, end)| {
                    !padded[*start..*end].contains('ſ')
                        && !ascii_word_before(&padded, *start)
                        && !ascii_word_at(&padded, *end)
                })
                .map(|(start, _)| (start, i))
        })
        .collect::<Vec<_>>();
    active.sort_unstable();
    let quote_bounds = crate::paired_quote_bounds(&padded);
    let mut matches = Vec::new();
    for (_, i) in active {
        let (form, _) = FORMS[i];
        for (start_byte, end_byte) in crate::unicode_case_match_ends(&padded, form) {
            let start = padded[..start_byte].chars().count();
            if quote_bounds.partition_point(|bound| *bound < start) % 2 == 1 {
                continue;
            }
            matches.push((i, start_byte, end_byte));
        }
    }
    let mut out = Vec::new();
    for (i, start_byte, end_byte) in matches {
        let (_, replacement) = FORMS[i];
        let start = padded[..start_byte].chars().count();
        let surface = &padded[start_byte..end_byte];
        let end = start + surface.chars().count();
        let (line, column) = crate::line_col_of(start, &lines);
        let preferred = crate::preferred_lookup(surface, replacement);
        out.push(json!({
            "check_path": "typography.diacritical_marks",
            "message": format!("Use diacritical marks in '{}'.", preferred.unwrap_or("None")),
            "span": [start, end],
            "pos": [line, column],
            "replacements": preferred,
        }));
    }
    out
}

fn ascii_word_before(text: &str, byte_index: usize) -> bool {
    byte_index > 0 && ascii_word(text.as_bytes()[byte_index - 1])
}

fn ascii_word_at(text: &str, byte_index: usize) -> bool {
    text.as_bytes()
        .get(byte_index)
        .copied()
        .is_some_and(ascii_word)
}

fn ascii_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
