#[path = "../src/rules/month_year_comma.rs"]
mod month_year_comma;

const MESSAGE: &str = "When specifying a month and year, no comma is needed.";

#[test]
fn matches_each_month_case_insensitively_and_globally() {
    let input = "January, 2024; February, 2024; March, 2024; April, 2024; May, 2024; June, 2024; July, 2024; August, 2024; September, 2024; October, 2024; November, 2024; December, 2024";
    let got = month_year_comma::diagnostics(input);
    assert_eq!(got.len(), 12);
    assert_eq!(got[0]["span"], serde_json::json!([1, 14]));
    assert_eq!(got[11]["span"], serde_json::json!([155, 169]));
    assert!(
        got.iter()
            .all(|d| d["check_path"] == "dates_times.dates.month_year_comma")
    );
    assert!(got.iter().all(|d| d["message"] == MESSAGE));
    assert!(got.iter().all(|d| d["replacements"].is_null()));

    let variants =
        month_year_comma::diagnostics("january, 2020; JANUARY, 2021; February, 99; March, 123");
    assert_eq!(variants.len(), 3);
    assert_eq!(variants[0]["span"], serde_json::json!([1, 14]));
    assert_eq!(variants[1]["span"], serde_json::json!([16, 29]));
    assert_eq!(variants[2]["span"], serde_json::json!([45, 55]));
}

#[test]
fn preserves_raw_pattern_spacing_threshold_and_substring_semantics() {
    let input = "April, 99; May, 100; June, 12345; July,2024; August,  2024";
    let got = month_year_comma::diagnostics(input);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0]["span"], serde_json::json!([12, 20]));
    assert_eq!(got[1]["span"], serde_json::json!([22, 33]));
    let embedded = month_year_comma::diagnostics("xJanuary, 2024x");
    assert_eq!(embedded.len(), 1);
    assert_eq!(embedded[0]["span"], serde_json::json!([2, 15]));
}

#[test]
fn suppresses_paired_quotes_and_reports_codepoint_positions() {
    assert!(month_year_comma::diagnostics("‘January, 2020’ and \"February, 2021\"").is_empty());
    let got = month_year_comma::diagnostics("éJanuary, 2024; February, ٢٠٢٤");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["span"], serde_json::json!([2, 15]));
    assert_eq!(got[0]["pos"], serde_json::json!([1, 2]));
}
