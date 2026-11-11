use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::test_types::{ParseErrorTestCase, ParseMessageTestCase};

#[test]
fn given_group_or_order_by_after_an_operator_when_parsing_snowflake_then_the_error_is_at_the_clause(
) {
    let test_cases = [
        ParseErrorTestCase {
            description: "AND before GROUP BY on the next line fails at GROUP, not at BY",
            sql: "SELECT customer_id, SUM(total) AS revenue\nFROM orders\nWHERE status = 'shipped' AND\nGROUP BY customer_id",
            expected_error_at: Some("GROUP"),
        },
        ParseErrorTestCase {
            description: "AND before ORDER BY on the same line fails at ORDER",
            sql: "SELECT order_id FROM orders WHERE total > 1 AND ORDER BY order_id",
            expected_error_at: Some("ORDER"),
        },
        ParseErrorTestCase {
            description: "NOT before GROUP BY fails at GROUP",
            sql: "SELECT status FROM orders WHERE NOT\ngroup by status",
            expected_error_at: Some("group"),
        },
        ParseErrorTestCase {
            description: "GROUP BY and ORDER BY after WHERE still parse",
            sql: "SELECT status FROM orders WHERE total > 1\nGROUP BY status\nORDER BY status",
            expected_error_at: None,
        },
        ParseErrorTestCase {
            description: "a quoted GROUP column still parses",
            sql: "SELECT \"GROUP\" FROM orders ORDER BY 1",
            expected_error_at: None,
        },
    ];

    for test_case in test_cases {
        let error_at = parse_statements_with(
            test_case.sql,
            Dialect::Snowflake,
            &ParseOptions::default(),
            CallMode::Faithful,
        )
        .err()
        .map(|error| {
            let start = error.start().unwrap_or_default();
            let end = error.end().unwrap_or_default();
            test_case.sql.get(start..end).unwrap_or_default().to_owned()
        });
        assert_eq!(
            error_at.as_deref(),
            test_case.expected_error_at,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_statement_ending_where_an_expression_belongs_when_parsing_snowflake_then_the_error_names_what_is_missing(
) {
    let test_cases = [
        ParseMessageTestCase {
            description: "an empty WHERE asks for a condition",
            sql: "SELECT * FROM orders WHERE",
            expected_message: "Expected a condition after WHERE",
        },
        ParseMessageTestCase {
            description: "an empty WHERE before a semicolon asks for a condition",
            sql: "SELECT * FROM orders WHERE ;",
            expected_message: "Expected a condition after WHERE",
        },
        ParseMessageTestCase {
            description: "an empty HAVING asks for a condition",
            sql: "SELECT status FROM orders GROUP BY status HAVING",
            expected_message: "Expected a condition after HAVING",
        },
        ParseMessageTestCase {
            description: "a join's empty ON asks for a condition",
            sql: "SELECT 1 FROM orders o JOIN customers c ON",
            expected_message: "Expected a condition after ON",
        },
        ParseMessageTestCase {
            description: "a comparison with no right side asks for an expression",
            sql: "SELECT 1 FROM orders WHERE total >",
            expected_message: "Expected an expression after >",
        },
    ];

    for test_case in test_cases {
        let message = parse_statements_with(
            test_case.sql,
            Dialect::Snowflake,
            &ParseOptions::default(),
            CallMode::Faithful,
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert!(
            message.contains(test_case.expected_message),
            "{}: {message}",
            test_case.description
        );
    }
}
