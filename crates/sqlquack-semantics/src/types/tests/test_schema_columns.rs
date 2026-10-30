use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::types::main::schema_columns::schema_columns;
use crate::types::models::{ColumnSpec, TypedLeaves};
use crate::types::tests::test_types::SchemaColumnsTestCase;

#[test]
fn given_schema_columns_when_read_then_marks_follow_the_leaves_they_may_hold() {
    let test_cases = [
        SchemaColumnsTestCase {
            description: "a plain VARIANT is proven free and trusted as a source's",
            column: ColumnSpec::new("PAYLOAD", SnowflakeType::SemiStructured),
            expected_marks: (true, true),
            expected_refusal: false,
        },
        SchemaColumnsTestCase {
            description: "a VARIANT that may hold a DATE is unproven and enters a typed leaf",
            column: ColumnSpec {
                typed_leaves: true,
                ..ColumnSpec::new("PAYLOAD", SnowflakeType::SemiStructured)
            },
            expected_marks: (false, false),
            expected_refusal: true,
        },
        SchemaColumnsTestCase {
            description: "a VARCHAR is proven free but no source VARIANT",
            column: ColumnSpec::new("NOTE", SnowflakeType::Varchar),
            expected_marks: (true, false),
            expected_refusal: false,
        },
    ];
    for test_case in test_cases {
        let typed = TypedLeaves::default();
        let read = schema_columns(std::slice::from_ref(&test_case.column), &typed);
        let marks: Vec<(bool, bool)> = read
            .iter()
            .map(|column| (column.untainted, column.sourced))
            .collect();
        assert_eq!(
            marks,
            [test_case.expected_marks],
            "{}",
            test_case.description
        );
        typed.compare(&Construct::new(ConstructFamily::Function, "EQUAL_NULL"));
        assert_eq!(
            typed.refusal().is_some(),
            test_case.expected_refusal,
            "{}",
            test_case.description
        );
    }
}
