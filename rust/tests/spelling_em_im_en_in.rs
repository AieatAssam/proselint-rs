use proselint_rs::spelling_em_im_en_in_diagnostics as diagnostics;
use serde_json::json;

#[test]
fn retains_all_mappings_and_the_pinned_overlap() {
    let input = "imbalm imbark imbed imbitter imblaze imbody imbolden imbosom imbower imbrown impanel impower incage incapsulate incase inclasp incumber incumbrance indow indowment indue infold ingraft ingulf inlace inmesh insheathe inshrine insnare insoul insphere inthrall inthrone intitle intomb intreat intrench intrust intwine intwist inwind inwrap inwreathe embrue empale empoverish enflame engrain enure";
    let actual = diagnostics(input);
    assert_eq!(actual.len(), 50);
    assert_eq!(actual[0]["replacements"], "embalm");
    assert_eq!(actual[49]["replacements"], "inure");
}

#[test]
fn active_patterns_emit_raw_matches_in_mapping_order() {
    let actual = diagnostics("ximbalm imb alM imbalmx imbalm ENFLAME enflame");
    assert_eq!(actual.len(), 5);
    assert_eq!(
        actual
            .iter()
            .map(|d| d["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("embalm"),
            json!("embalm"),
            json!("embalm"),
            json!("inflame"),
            json!("inflame")
        ]
    );
    assert_eq!(actual[0]["span"], json!([2, 8]));
    assert_eq!(actual[1]["span"], json!([17, 23]));
    assert_eq!(actual[2]["span"], json!([25, 31]));
}

#[test]
fn quoted_matches_and_unicode_positions_follow_the_pinned_check() {
    let actual = diagnostics("é imbalm ‘imbalm’ INTHRALL");
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0]["span"], json!([3, 9]));
    assert_eq!(actual[0]["pos"], json!([1, 3]));
    assert_eq!(actual[0]["replacements"], "embalm");
    assert_eq!(actual[1]["replacements"], "enthrall");
}

#[test]
fn overlapping_prefix_form_is_emitted_once() {
    let actual = diagnostics("incapsulate");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["replacements"], "encapsulate");
}

#[test]
fn suppresses_matches_inside_paired_quotes_but_not_unpaired_quotes() {
    assert!(diagnostics("“imbalm” and “embalm”; \"enure\"").is_empty());
    let actual = diagnostics("x‘imbalm’x");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["replacements"], "embalm");
}

#[test]
fn active_patterns_follow_earliest_activation_before_raw_match_emission() {
    let actual = diagnostics("inwrap imbalm enflame");
    assert_eq!(
        actual
            .iter()
            .map(|d| d["replacements"].clone())
            .collect::<Vec<_>>(),
        vec![json!("enwrap"), json!("embalm"), json!("inflame")]
    );
}
