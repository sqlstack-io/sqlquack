//! Reviewed rejections cover only their exact statement and dialects.

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_syntax::Dialect;

use crate::helpers::summary_of;
use crate::test_types::RejectionTestCase;

#[test]
fn given_rejected_statements_when_compared_then_only_the_reviewed_one_is_explained() {
    let test_cases = [
        RejectionTestCase {
            description: "the reviewed statement in DuckDB is rejected as reviewed",
            dialect: Dialect::DuckDB,
            sql: "SELECT __polyglot_typed(amount, 'NOT A TYPE (') FROM orders",
            expected_rejected: 1,
            expected_differences: 0,
        },
        RejectionTestCase {
            description: "the reviewed statement in Snowflake is rejected as reviewed",
            dialect: Dialect::Snowflake,
            sql: "SELECT __polyglot_typed(amount, 'NOT A TYPE (') FROM orders",
            expected_rejected: 1,
            expected_differences: 0,
        },
        RejectionTestCase {
            description: "other malformed type text is still a difference",
            dialect: Dialect::DuckDB,
            sql: "SELECT __polyglot_typed(amount, 'INT garbage') FROM orders",
            expected_rejected: 0,
            expected_differences: 1,
        },
    ];

    for test_case in test_cases {
        let summary = summary_of(&[CorpusEntry {
            dialect: test_case.dialect,
            sql: test_case.sql.to_owned(),
            from: test_case.description.to_owned(),
        }]);
        assert_eq!(
            (
                summary.rejected.values().sum::<usize>(),
                summary.differences.len()
            ),
            (test_case.expected_rejected, test_case.expected_differences),
            "{}",
            test_case.description
        );
    }
}
