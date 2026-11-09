use serde_json::json;

use crate::parser::tests::helpers::value_at;
use crate::parser::tests::test_types::ParseValueTestCase;

#[test]
fn given_an_oracle_join_marker_when_parsing_then_the_column_is_marked() {
    let test_cases = [ParseValueTestCase {
        description: "(+) after a column marks it for an outer join",
        dialect: None,
        sql: "select a.baz from a where a.baz = b.baz (+)",
        pointer: "/0/select/where_clause/this/eq/right/column/join_mark",
        expected_value: Ok(json!(true)),
    }];

    for test_case in test_cases {
        assert_eq!(
            value_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
