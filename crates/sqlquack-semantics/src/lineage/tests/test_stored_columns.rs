use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::lineage::main::stored_columns::stored_columns;
use crate::lineage::tests::test_types::StoredColumnsTestCase;
use crate::types::models::{ColumnSpec, Holding};

#[test]
fn given_stored_variants_when_read_back_then_typed_builds_come_back_as_json() {
    let test_cases = [
        StoredColumnsTestCase {
            description: "a VARIANT stored as JSON reads back as JSON",
            written: Holding::Json,
            expected_read: Holding::Json,
        },
        StoredColumnsTestCase {
            description: "a VARIANT stored from a typed build reads back chained",
            written: Holding::Variant,
            expected_read: Holding::Chained,
        },
        StoredColumnsTestCase {
            description: "a chained VARIANT stays chained",
            written: Holding::Chained,
            expected_read: Holding::Chained,
        },
    ];
    for test_case in test_cases {
        let column = ColumnSpec {
            holding: test_case.written,
            ..ColumnSpec::new("PAYLOAD", SnowflakeType::SemiStructured)
        };
        let read: Vec<Holding> = stored_columns(vec![column])
            .into_iter()
            .map(|column| column.holding)
            .collect();
        assert_eq!(read, [test_case.expected_read], "{}", test_case.description);
    }
}
