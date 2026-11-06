#[path = "../src/rules/month_of_year.rs"]
mod month_of_year;

const MESSAGE: &str = "When specifying a month and year, 'of' is unnecessary.";

#[test]
fn month_of_year_all_months() {
    let input = "January of 2024; February of 2024; March of 2024; April of 2024; May of 2024; June of 2024; July of 2024; August of 2024; September of 2024; October of 2024; November of 2024; December of 2024";
    let got = month_of_year::diagnostics(input);
    assert_eq!(got.len(), 12);
    assert_eq!(got[0]["span"], serde_json::json!([1, 16]));
    assert_eq!(got[11]["span"], serde_json::json!([177, 193]));
    assert!(
        got.iter()
            .all(|d| d["check_path"] == "dates_times.dates.month_of_year")
    );
    assert!(got.iter().all(|d| d["message"] == MESSAGE));
    assert!(got.iter().all(|d| d["replacements"].is_null()));
}

#[test]
fn month_of_year_digits_quotes_unicode_positions() {
    assert!(month_of_year::diagnostics("‘January of 2020’ and \"February of 2021\"").is_empty());
    let got = month_of_year::diagnostics("éJanuary of 2024; February of ٢٠٢٤");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["span"], serde_json::json!([2, 17]));
    assert_eq!(got[0]["pos"], serde_json::json!([1, 2]));
}

#[test]
fn month_of_year_ascii_digit_boundary() {
    let input = "May of 99; June of 123; July of 1234; August of ٢٠٢٤";
    let got = month_of_year::diagnostics(input);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0]["span"], serde_json::json!([12, 23]));
    assert_eq!(got[1]["span"], serde_json::json!([25, 37]));
}
