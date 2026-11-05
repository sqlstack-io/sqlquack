use crate::analysis::tests::helpers::{everyday_outcome_mismatches, everyday_translation_rates};
use crate::analysis::tests::test_types::EverydayCorpusTestCase;

#[test]
fn given_everyday_snowflake_queries_when_checked_and_converted_then_each_keeps_its_outcome() {
    let test_cases = [EverydayCorpusTestCase {
        description: "every corpora/everyday query parses, checks, converts or refuses as recorded, under the strict built-in policy and the repository policy",
        expected_mismatches: Vec::new(),
        expected_rates: "285 of 434 strict, 354 of 434 repository",
    }];

    for test_case in test_cases {
        assert_eq!(
            everyday_outcome_mismatches(),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_translation_rates(),
            test_case.expected_rates,
            "{}",
            test_case.description
        );
    }
}
