use std::collections::HashMap;

use crate::lineage::tests::helpers::{duckdb_output_type, replayed_engine_types, typed_column};
use crate::lineage::tests::test_types::{
    DuckdbTypeTestCase, EngineTypeReplayTestCase, TypedShopSchema,
};

/// Result types DuckDB itself bound for synthetic expressions, recorded by polyglot's tooling.
const ENGINE_TYPES: &str = include_str!("fixtures/duckdb_engine_types.json");

#[test]
fn given_engine_recorded_duckdb_types_when_typing_outputs_then_none_is_wrong() {
    let test_cases = [EngineTypeReplayTestCase {
        description: "every recorded DuckDB result type, replayed",
        fixture: ENGINE_TYPES,
        expected_wrong: &[],
        expected_minimum_typed: 2556,
    }];
    for test_case in test_cases {
        let replay = replayed_engine_types(test_case.fixture);
        assert_eq!(
            (
                replay.wrong.as_slice(),
                replay.typed >= test_case.expected_minimum_typed
            ),
            (test_case.expected_wrong, true),
            "{}: {:?}",
            test_case.description,
            replay.tally
        );
    }
}

#[test]
fn given_duckdb_functions_when_typing_outputs_then_each_has_the_engine_type() {
    let test_cases = [
        DuckdbTypeTestCase {
            description: "UPPER and LOWER give text",
            sql: "SELECT UPPER(name) AS v FROM t",
            expected_type: Some("VARCHAR"),
        },
        DuckdbTypeTestCase {
            description: "LOWER of a cast gives text",
            sql: "SELECT LOWER(CAST(quantity AS VARCHAR)) AS v FROM t",
            expected_type: Some("VARCHAR"),
        },
        DuckdbTypeTestCase {
            description: "ROW_NUMBER is BIGINT",
            sql: "SELECT ROW_NUMBER() OVER (ORDER BY quantity) AS v FROM t",
            expected_type: Some("BIGINT"),
        },
        DuckdbTypeTestCase {
            description: "SUM over DECIMAL(10,2) widens to DECIMAL(38,2)",
            sql: "SELECT SUM(price) AS v FROM t",
            expected_type: Some("DECIMAL(38,2)"),
        },
        DuckdbTypeTestCase {
            description: "SUM over INTEGER is HUGEINT",
            sql: "SELECT SUM(quantity) AS v FROM t",
            expected_type: Some("HUGEINT"),
        },
        DuckdbTypeTestCase {
            description: "LAG keeps its value's type as spelled",
            sql: "SELECT LAG(price) OVER (ORDER BY quantity) AS v FROM t",
            expected_type: Some("DECIMAL(10,2)"),
        },
        DuckdbTypeTestCase {
            description: "COALESCE of INTEGER and BIGINT is BIGINT",
            sql: "SELECT COALESCE(quantity, line_count) AS v FROM t",
            expected_type: Some("BIGINT"),
        },
        DuckdbTypeTestCase {
            description: "a cast to a STRUCT spells its fields",
            sql: "SELECT CAST({'a': quantity} AS STRUCT(\"café\" INTEGER, b VARCHAR[])) AS v FROM t",
            expected_type: Some("STRUCT(\"café\" INTEGER, b VARCHAR[])"),
        },
        DuckdbTypeTestCase {
            description: "a cast to sized text keeps the length as written (DuckDB VARCHAR(n) is VARCHAR)",
            sql: "SELECT CAST(name AS VARCHAR(3)) AS v FROM t",
            expected_type: Some("VARCHAR(3)"),
        },
        DuckdbTypeTestCase {
            description: "an unknown function stays untyped",
            sql: "SELECT unknown_udf(quantity) AS v FROM t",
            expected_type: None,
        },
    ];
    let schema: TypedShopSchema = TypedShopSchema {
        closed: HashMap::from([(
            "t".to_owned(),
            vec![
                typed_column("quantity", "INTEGER"),
                typed_column("line_count", "BIGINT"),
                typed_column("price", "DECIMAL(10,2)"),
                typed_column("name", "VARCHAR"),
            ],
        )]),
        open: HashMap::new(),
        functions: HashMap::new(),
        rules: HashMap::new(),
        declared: Vec::new(),
        arities: HashMap::new(),
    };
    for test_case in test_cases {
        assert_eq!(
            duckdb_output_type(&schema, test_case.sql).as_deref(),
            test_case.expected_type,
            "{}",
            test_case.description
        );
    }
}
