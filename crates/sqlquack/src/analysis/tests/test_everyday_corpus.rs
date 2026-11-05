use crate::analysis::tests::helpers::everyday_outcome_mismatches;
use crate::analysis::tests::test_types::EverydayCorpusTestCase;

#[test]
fn given_everyday_snowflake_queries_when_checked_and_converted_then_each_keeps_its_outcome() {
    let test_cases = [EverydayCorpusTestCase {
        description: "every corpora/everyday query parses, checks, converts or refuses as recorded",
        expected_mismatches: Vec::new(),
    }];

    for test_case in test_cases {
        assert_eq!(
            everyday_outcome_mismatches(),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
    }
}
