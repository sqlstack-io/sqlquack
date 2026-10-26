use crate::analysis::tests::helpers::equivalence_case_checks;
use crate::analysis::tests::test_types::EquivalenceCasesTestCase;

#[test]
fn given_repository_pair_cases_when_checking_then_every_pair_is_declared_and_covered() {
    let test_cases = [EquivalenceCasesTestCase {
        description: "every pair file declares a known spelling pair, and every pair has cases",
        expected_mismatches: Vec::new(),
    }];

    for test_case in test_cases {
        assert_eq!(
            equivalence_case_checks(),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
    }
}
