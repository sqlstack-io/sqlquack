use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::DUCKDB;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::Nullability::NonNull;
use crate::lineage::tests::helpers::typed_shop_schema;
use crate::lineage::tests::test_types::DialectTypeTestCase;

#[test]
fn given_a_query_when_typing_outputs_then_nulls_and_numbers_follow_the_recorded_engines() {
    let test_cases = [DialectTypeTestCase {
        description: "DuckDB's plain ASOF JOIN is inner",
        sql: "SELECT o.ID, c.NAME FROM ORDERS o ASOF JOIN CUSTOMERS c ON o.ID >= c.ID",
        dialect: Dialect::DuckDB,
        profile: &DUCKDB,
        expected_outputs: &[
            ("ID", Some("NUMBER(38,0)"), NonNull),
            ("NAME", Some("VARCHAR(100)"), NonNull),
        ],
    }];
    let schema = typed_shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: None,
        };
        let lineage = query_lineage(
            test_case.sql,
            test_case.dialect,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", test_case.description));
        let outputs: Vec<(&str, Option<&str>, _)> = lineage
            .outputs
            .iter()
            .map(|output| {
                (
                    output.name.as_str(),
                    output.data_type.as_deref(),
                    output.nullability,
                )
            })
            .collect();
        assert_eq!(
            outputs,
            test_case.expected_outputs.to_vec(),
            "{}",
            test_case.description
        );
    }
}
