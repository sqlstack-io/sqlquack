use crate::cli::_helpers::local::parser_tree;
use crate::cli::tests::test_types::ParserTreeTestCase;
use crate::formats::_helpers::dialect::{set_dialect, snowflake};
use crate::formats::models::Dialect;

#[test]
fn given_an_uncommitted_parser_path_when_reading_the_tree_then_only_snowflake_fails() {
    let test_cases = [
        ParserTreeTestCase {
            description: "a Snowflake root whose parser path is missing still fails",
            dialect: "snowflake",
            parser_path: "crates/sqlquack-syntax-missing",
            expected_outcome: "error",
        },
        ParserTreeTestCase {
            description: "a Postgres root whose parser crate is not committed has no tree",
            dialect: "postgres",
            parser_path: "crates/sqlquack-syntax-missing",
            expected_outcome: "none",
        },
    ];
    for test_case in test_cases {
        set_dialect(Dialect {
            name: test_case.dialect.to_owned(),
            parser_path: test_case.parser_path.to_owned(),
            ..snowflake()
        });
        let outcome: &str = parser_tree().map_or("error", |tree| tree.map_or("none", |_| "tree"));
        assert_eq!(
            outcome, test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
