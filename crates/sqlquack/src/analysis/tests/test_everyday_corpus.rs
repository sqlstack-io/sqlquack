use crate::analysis::tests::helpers::{
    EVERYDAY_CORPUS, EVERYDAY_MODELS_CORPUS, everyday_outcome_mismatches,
    everyday_snowflake_rejections_not_caught, everyday_translation_rates,
};
use crate::analysis::tests::test_types::EverydayCorpusTestCase;

#[test]
fn given_everyday_snowflake_queries_when_checked_and_converted_then_each_keeps_its_outcome() {
    let test_cases = [EverydayCorpusTestCase {
        description: "every corpora/everyday query parses, checks, converts or refuses as recorded, under the strict built-in policy and the repository policy",
        directory: EVERYDAY_CORPUS,
        expected_mismatches: Vec::new(),
        expected_rates: "287 of 436 strict, 356 of 436 repository",
        // SQL Snowflake rejects (snowflake-receipts.json) that SQLQuack neither fails to parse nor
        // flags in its checks; each should become a parse or check error.
        expected_rejections_not_caught: vec![
            "SELECT COUNT(*) FILTER (WHERE status = 'shipped') AS shipped FROM orders",
            "SELECT order_id, total FROM orders PIVOT (SUM(total) FOR status IN ('shipped', 'pending')) AS p",
            "SELECT order_id FROM orders WHERE status == 'shipped'",
            "SELECT order_id FROM orders WHERE status IN 'shipped'",
            "SELECT order_id FROM orders WHERE total >> 10",
            "SELECT order_id FROM orders WHERE ordered_at > 5",
            "SELECT LENGTH(order_id, 2) AS x FROM orders",
            "SELECT order_id FROM orders o, orders o",
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            everyday_outcome_mismatches(test_case.directory),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_translation_rates(test_case.directory),
            test_case.expected_rates,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_snowflake_rejections_not_caught(),
            test_case.expected_rejections_not_caught,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_dbt_style_snowflake_models_when_checked_and_converted_then_each_keeps_its_outcome() {
    let test_cases = [EverydayCorpusTestCase {
        description: "every corpora/everyday-models query parses, checks, converts or refuses as recorded, under the strict built-in policy and the repository policy",
        directory: EVERYDAY_MODELS_CORPUS,
        expected_mismatches: Vec::new(),
        expected_rates: "175 of 300 strict, 247 of 300 repository",
        expected_rejections_not_caught: Vec::new(),
    }];

    for test_case in test_cases {
        assert_eq!(
            everyday_outcome_mismatches(test_case.directory),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_translation_rates(test_case.directory),
            test_case.expected_rates,
            "{}",
            test_case.description
        );
    }
}
