use crate::check::_helpers::receipts::case_result;
use crate::check::models::{LocalVerdict, Reason};
use crate::check::tests::helpers::{SETUP, case_line, receipt_line, with_fields};
use crate::check::tests::test_types::EngineReleaseTestCase;
use crate::formats::_helpers::dialect::{set_dialect, snowflake};
use crate::formats::models::{Case, Dialect, Receipt};

/// A receipt of `case` recording `phase` at engine release `version`.
fn release_receipt(case: &str, phase: &str, version: &str) -> Receipt {
    let line: String = with_fields(
        &receipt_line(case, phase),
        serde_json::json!({"engine_version": version}),
    );
    serde_json::from_str(&line).expect("receipt")
}

#[test]
fn given_receipts_from_several_releases_when_pinned_then_only_the_pin_year_counts_and_the_newest_decides()
 {
    let test_cases = [
        EngineReleaseTestCase {
            description: "a receipt from an earlier release of the pinned year counts",
            receipts: &[("accepted", "2026.44")],
            expected_phase: Some("Accepted"),
            expected_reason: None,
        },
        EngineReleaseTestCase {
            description: "a receipt from an earlier year does not count",
            receipts: &[("accepted", "2025.50")],
            expected_phase: None,
            expected_reason: Some(Reason::StaleReceipt),
        },
        EngineReleaseTestCase {
            description: "a receipt from an earlier year cannot disagree with the pinned year",
            receipts: &[("syntax_error", "2025.50"), ("accepted", "2026.44")],
            expected_phase: Some("Accepted"),
            expected_reason: None,
        },
        EngineReleaseTestCase {
            description: "two releases of the pinned year agreeing count as the newest",
            receipts: &[("accepted", "2026.44"), ("accepted", "2026.45")],
            expected_phase: Some("Accepted"),
            expected_reason: None,
        },
        EngineReleaseTestCase {
            description: "two releases of the pinned year disagreeing need review",
            receipts: &[("syntax_error", "2026.44"), ("accepted", "2026.45")],
            expected_phase: None,
            expected_reason: Some(Reason::ReleaseDisagreement),
        },
    ];
    let pinned = || Dialect {
        engine_pin: Some("2026.45".to_owned()),
        ..snowflake()
    };
    let accepted = || LocalVerdict::Accepted {
        opaque: false,
        tree_sha256: None,
    };
    for test_case in test_cases {
        set_dialect(pinned());
        let line: String = case_line("select-17", "sf.query.select.o001", "SELECT 17", &[SETUP]);
        let case: Case = serde_json::from_str(&line).expect("case");
        let receipts: Vec<Receipt> = test_case
            .receipts
            .iter()
            .map(|(phase, version)| release_receipt(&line, phase, version))
            .collect();
        let borrowed: Vec<&Receipt> = receipts.iter().collect();
        let result = case_result(&case, &borrowed, &borrowed, accepted());
        assert_eq!(
            (
                result.phase.map(|phase| format!("{phase:?}")),
                result.reason
            ),
            (
                test_case.expected_phase.map(str::to_owned),
                test_case.expected_reason
            ),
            "{}",
            test_case.description
        );
    }
    set_dialect(pinned());
    let disputed: String = case_line("select-17", "sf.query.select.o001", "SELECT 17", &[SETUP]);
    let settled: String = case_line("select-18", "sf.query.select.o001", "SELECT 18", &[SETUP]);
    let receipts: Vec<Receipt> = vec![
        release_receipt(&disputed, "syntax_error", "2026.44"),
        release_receipt(&disputed, "accepted", "2026.45"),
        release_receipt(&settled, "accepted", "2026.44"),
    ];
    let reasons: Vec<Option<Reason>> = [&disputed, &settled]
        .into_iter()
        .map(|line| {
            let case: Case = serde_json::from_str(line).expect("case");
            let own: Vec<&Receipt> = receipts
                .iter()
                .filter(|receipt| receipt.case_id == case.case_id)
                .collect();
            case_result(&case, &own, &own, accepted()).reason
        })
        .collect();
    assert_eq!(
        reasons,
        vec![Some(Reason::ReleaseDisagreement), None],
        "a disagreement between releases flags only its own case"
    );
    set_dialect(snowflake());
}
