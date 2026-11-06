#[path = "../src/rules/spelling_er_or.rs"]
mod spelling_er_or;

#[test]
fn matches_the_frozen_er_or_cases() {
    let all = "abducter abbeter acquiror adaptor collecter conjuror corruptor digestor dispensor distributer endorsor erasor idolator imposter infiltrater invester manipulater mortgager persecuter promotor purveyer requestor revisor surveyer";
    let got = spelling_er_or::diagnostics(all);
    assert_eq!(got.len(), 24);
    assert_eq!(got[0]["check_path"], "spelling.er_or");
    assert_eq!(
        got[0]["message"],
        "-er vs. -or. 'abductor' is the preferred spelling."
    );
    assert_eq!(got[0]["span"], serde_json::json!([1, 9]));
    assert_eq!(got[0]["pos"], serde_json::json!([1, 1]));
    assert_eq!(got[0]["replacements"], "abductor");
    assert_eq!(
        got.iter()
            .map(|d| d["replacements"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "abductor",
            "abettor",
            "acquirer",
            "adapter",
            "collector",
            "conjurer",
            "corrupter",
            "digester",
            "dispenser",
            "distributor",
            "endorser",
            "eraser",
            "idolater",
            "impostor",
            "infiltrator",
            "investor",
            "manipulator",
            "mortgagor",
            "persecutor",
            "promoter",
            "purveyor",
            "requester",
            "reviser",
            "surveyor",
        ]
    );

    assert!(spelling_er_or::diagnostics("abductor abettor acquirer adapter").is_empty());
    assert!(spelling_er_or::diagnostics("“abducter” and “abductor”").is_empty());
    let repeated = spelling_er_or::diagnostics("AbDuCtEr abducter REQUESTOR requestor");
    assert_eq!(repeated.len(), 4);
    assert_eq!(repeated[0]["span"], serde_json::json!([1, 9]));
    assert_eq!(repeated[1]["span"], serde_json::json!([10, 18]));
    let boundaries = spelling_er_or::diagnostics("xabducter abducterx (abducter), “abducter”");
    assert_eq!(boundaries.len(), 3);
    assert_eq!(boundaries[0]["span"], serde_json::json!([2, 10]));
    assert_eq!(boundaries[1]["span"], serde_json::json!([11, 19]));
    assert_eq!(boundaries[2]["span"], serde_json::json!([22, 30]));
    let unicode = spelling_er_or::diagnostics("é abducter, Ω requestor");
    assert_eq!(unicode[0]["span"], serde_json::json!([3, 11]));
    assert_eq!(unicode[1]["span"], serde_json::json!([15, 24]));
    assert_eq!(unicode[1]["pos"], serde_json::json!([1, 15]));

    let order = spelling_er_or::diagnostics("requestor abducter");
    assert_eq!(order[0]["replacements"], "requester");
    assert_eq!(order[0]["span"], serde_json::json!([1, 10]));
    assert_eq!(order[1]["replacements"], "abductor");
    assert_eq!(order[1]["span"], serde_json::json!([11, 19]));

    let max_errors_one = spelling_er_or::diagnostics("requestor abducter erasor");
    assert_eq!(max_errors_one[0]["replacements"], "requester");
    assert_eq!(max_errors_one[0]["span"], serde_json::json!([1, 10]));

    // The CLI truncates this matcher-ordered list before sorting by position.
    let max_errors_order = spelling_er_or::diagnostics("abducter requestor abducter");
    assert_eq!(max_errors_order.len(), 3);
    assert_eq!(max_errors_order[0]["replacements"], "abductor");
    assert_eq!(max_errors_order[0]["span"], serde_json::json!([1, 9]));
    assert_eq!(max_errors_order[1]["replacements"], "abductor");
    assert_eq!(max_errors_order[1]["span"], serde_json::json!([20, 28]));
    assert_eq!(max_errors_order[2]["replacements"], "requester");
    assert_eq!(max_errors_order[2]["span"], serde_json::json!([10, 19]));

    let long_s = spelling_er_or::diagnostics("diſpenſor dispensor");
    assert_eq!(long_s.len(), 2);
    assert_eq!(
        long_s[0]["message"],
        "-er vs. -or. 'None' is the preferred spelling."
    );
    assert!(long_s[0]["replacements"].is_null());
}
