use crate::results::main::diff_results::diff_results;
use crate::results::models::StatementOutcome;
use crate::results::tests::helpers::baseline;
use crate::results::tests::test_types::DiffTestCase;

#[test]
fn given_a_changed_statement_when_diffing_then_only_drops_from_matching_and_new_differs_regress() {
    let test_cases = [
        DiffTestCase {
            description: "matching to refused",
            base: Some(StatementOutcome::Matching),
            head: Some(StatementOutcome::Refused),
            expected_regression: true,
        },
        DiffTestCase {
            description: "a matching error to syntax only",
            base: Some(StatementOutcome::MatchingError),
            head: Some(StatementOutcome::ShapeOnly),
            expected_regression: true,
        },
        DiffTestCase {
            description: "a matching statement removed",
            base: Some(StatementOutcome::Matching),
            head: None,
            expected_regression: true,
        },
        DiffTestCase {
            description: "refused to differs",
            base: Some(StatementOutcome::Refused),
            head: Some(StatementOutcome::Differs),
            expected_regression: true,
        },
        DiffTestCase {
            description: "matching to a matching error stays matching",
            base: Some(StatementOutcome::Matching),
            head: Some(StatementOutcome::MatchingError),
            expected_regression: false,
        },
        DiffTestCase {
            description: "refused to matching improves",
            base: Some(StatementOutcome::Refused),
            head: Some(StatementOutcome::Matching),
            expected_regression: false,
        },
        DiffTestCase {
            description: "a new refused statement",
            base: None,
            head: Some(StatementOutcome::Refused),
            expected_regression: false,
        },
    ];
    for test_case in test_cases {
        let diff = diff_results(&baseline(test_case.base), &baseline(test_case.head));

        assert_eq!(
            !diff.regressions.is_empty(),
            test_case.expected_regression,
            "{}",
            test_case.description
        );
        assert_eq!(
            diff.regressions.len() + diff.changes.len(),
            1,
            "{}",
            test_case.description
        );
    }
}
