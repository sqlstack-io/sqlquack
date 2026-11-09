use crate::snowflake::typing::models::SnowflakeType;

use crate::dialects::constants::SNOWFLAKE_TO_DUCKDB;
use crate::snowflake::names::main::relation_columns::relation_columns;
use crate::snowflake::names::tests::test_types::RelationColumnsTestCase;
use crate::snowflake::types::models::TypedLeaves;
use crate::snowflake::typing::types::{ColumnSpec, Schema};

#[test]
fn given_ctes_and_a_schema_when_reading_relations_then_the_innermost_name_wins() {
    let column = |name: &str| ColumnSpec::new(name, SnowflakeType::Varchar);
    let mut schema = Schema::default();
    schema
        .relations
        .insert("ORDERS".to_owned(), vec![column("ORDER_ID")]);
    schema
        .relations
        .insert("customers".to_owned(), vec![column("CUSTOMER_ID")]);
    let ctes = vec![
        ("ORDERS".to_owned(), vec![column("OUTER_ID")]),
        ("ORDERS".to_owned(), vec![column("INNER_ID")]),
    ];
    let test_cases = [
        RelationColumnsTestCase {
            description: "the innermost CTE hides an outer one and the schema",
            name: "ORDERS",
            expected_outcome: Ok(vec!["INNER_ID"]),
        },
        RelationColumnsTestCase {
            description: "a schema relation by its exact name",
            name: "customers",
            expected_outcome: Ok(vec!["CUSTOMER_ID"]),
        },
        RelationColumnsTestCase {
            description: "a name differing from a schema relation only by case refuses",
            name: "CUSTOMERS",
            expected_outcome: Err(
                "relation `CUSTOMERS` matches `customers` only without regard to case",
            ),
        },
        RelationColumnsTestCase {
            description: "an unknown relation has no known columns",
            name: "RETURNS",
            expected_outcome: Err("relation `RETURNS` has no known columns"),
        },
    ];
    for test_case in test_cases {
        let outcome = relation_columns(
            test_case.name,
            (&ctes, &schema),
            &TypedLeaves::default(),
            SNOWFLAKE_TO_DUCKDB,
        )
        .map(|columns| {
            columns
                .into_iter()
                .map(|column| column.name)
                .collect::<Vec<_>>()
        })
        .map_err(|refusal| refusal.reason);
        assert_eq!(
            outcome,
            test_case
                .expected_outcome
                .map(|names| names.into_iter().map(str::to_owned).collect())
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
