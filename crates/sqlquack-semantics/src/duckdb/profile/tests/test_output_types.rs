use std::collections::HashMap;

use crate::dialects::constants::DUCKDB;
use crate::duckdb::profile::tests::helpers::duckdb_output_type;
use crate::duckdb::profile::tests::helpers::replayed_engine_types;
use crate::duckdb::profile::tests::test_types::DuckdbTypeTestCase;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::{EngineTypeReplayTestCase, TypedShopSchema};

/// Result types DuckDB itself bound for synthetic expressions, recorded by polyglot's tooling.
const ENGINE_TYPES: &str = include_str!("fixtures/duckdb_engine_types.json");

#[test]
fn given_engine_recorded_duckdb_types_when_typing_outputs_then_none_is_wrong() {
    let test_cases = [EngineTypeReplayTestCase {
        description: "every recorded DuckDB result type, replayed",
        fixture: ENGINE_TYPES,
        replayed: replayed_engine_types,
        expected_wrong: &[],
        expected_minimum_typed: 2568,
        expected_maximum_untyped: 1,
    }];
    for test_case in test_cases {
        let replay = (test_case.replayed)(test_case.fixture);
        let untyped: usize = replay.tally.values().map(|tally| tally.untyped).sum();
        assert_eq!(
            (
                replay.wrong.as_slice(),
                replay.typed >= test_case.expected_minimum_typed,
                untyped <= test_case.expected_maximum_untyped
            ),
            (test_case.expected_wrong, true, true),
            "{}: typed {}, untyped {untyped} {:?}",
            test_case.description,
            replay.typed,
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
            description: "a recursive CTE keeps its anchor's type when its recursive branch is text",
            sql: "WITH RECURSIVE r(n) AS (SELECT 1 UNION ALL SELECT 'x' FROM r WHERE n < 3) \
                  SELECT n AS v FROM r",
            expected_type: Some("INTEGER"),
        },
        DuckdbTypeTestCase {
            description: "a recursive CTE keeps its anchor's text type over an integer branch",
            sql: "WITH RECURSIVE r(n) AS (SELECT 'a' UNION ALL SELECT 1 FROM r WHERE length(n) < 3) \
                  SELECT n AS v FROM r",
            expected_type: Some("VARCHAR"),
        },
        DuckdbTypeTestCase {
            description: "a recursive CTE keeps an INTEGER anchor over a DECIMAL step",
            sql: "WITH RECURSIVE r(n) AS (SELECT 1 UNION SELECT n + 1.5 FROM r WHERE n < 3) \
                  SELECT n AS v FROM r",
            expected_type: Some("INTEGER"),
        },
        DuckdbTypeTestCase {
            description: "a plain UNION ALL of an integer and a text literal unifies to text",
            sql: "SELECT 1 AS v UNION ALL SELECT 'x'",
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
            description: "sized TEXT is DuckDB's VARCHAR of that length as written",
            sql: "SELECT CAST(name AS TEXT(5)) AS v FROM t",
            expected_type: Some("VARCHAR(5)"),
        },
        DuckdbTypeTestCase {
            description: "EXTRACT of a numbered part is BIGINT",
            sql: "SELECT EXTRACT(YEAR FROM created_at) AS v FROM t",
            expected_type: Some("BIGINT"),
        },
        DuckdbTypeTestCase {
            description: "EXTRACT of the epoch is DOUBLE",
            sql: "SELECT EXTRACT(EPOCH FROM created_at) AS v FROM t",
            expected_type: Some("DOUBLE"),
        },
        DuckdbTypeTestCase {
            description: "an INTERVAL is INTERVAL",
            sql: "SELECT INTERVAL 1 DAY AS v FROM t",
            expected_type: Some("INTERVAL"),
        },
        DuckdbTypeTestCase {
            description: "a timestamp plus an INTERVAL stays a timestamp",
            sql: "SELECT created_at + INTERVAL 1 DAY AS v FROM t",
            expected_type: Some("TIMESTAMP"),
        },
        DuckdbTypeTestCase {
            description: "a scalar subquery has its one column's type",
            sql: "SELECT (SELECT CAST(name AS VARCHAR(3)) FROM t LIMIT 1) AS v FROM t",
            expected_type: Some("VARCHAR(3)"),
        },
        DuckdbTypeTestCase {
            description: "a typed marker in a scalar subquery types the subquery",
            sql: "SELECT (SELECT __polyglot_typed(bar(LENGTH(name), 0, 10), 'VARCHAR') FROM t LIMIT 1) AS v FROM t",
            expected_type: Some("VARCHAR"),
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
                typed_column("created_at", "TIMESTAMP"),
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

#[test]
fn given_json_and_time_zone_expressions_when_typing_outputs_then_both_parsers_give_the_engine_type()
{
    // Each type is DuckDB 2.0.0's typeof over the same columns.
    let test_cases = [
        DuckdbTypeTestCase {
            description: "a cast to JSON is JSON",
            sql: "SELECT CAST(name AS JSON) AS v FROM t",
            expected_type: Some("JSON"),
        },
        DuckdbTypeTestCase {
            description: "a :: cast to JSON is JSON",
            sql: "SELECT name::JSON AS v FROM t",
            expected_type: Some("JSON"),
        },
        DuckdbTypeTestCase {
            description: "a cast to a JSON list is JSON[]",
            sql: "SELECT CAST(name AS JSON[]) AS v FROM t",
            expected_type: Some("JSON[]"),
        },
        DuckdbTypeTestCase {
            description: "to_json gives JSON",
            sql: "SELECT to_json(quantity) AS v FROM t",
            expected_type: Some("JSON"),
        },
        DuckdbTypeTestCase {
            description: "a JSON cast over to_json of a JSON COALESCE is JSON",
            sql: "SELECT CAST(to_json(COALESCE(name::JSON, '[]'::JSON)) AS JSON) AS v FROM t",
            expected_type: Some("JSON"),
        },
        DuckdbTypeTestCase {
            description: "TO_TIMESTAMP of a number is TIMESTAMPTZ",
            sql: "SELECT TO_TIMESTAMP(quantity / POWER(10, 3)) AS v FROM t",
            expected_type: Some("TIMESTAMPTZ"),
        },
        DuckdbTypeTestCase {
            description: "AT TIME ZONE gives a TIMESTAMP its zone",
            sql: "SELECT created_at AT TIME ZONE 'UTC' AS v FROM t",
            expected_type: Some("TIMESTAMPTZ"),
        },
        DuckdbTypeTestCase {
            description: "AT TIME ZONE takes a TIMESTAMPTZ's zone away",
            sql: "SELECT seen_at AT TIME ZONE 'UTC' AS v FROM t",
            expected_type: Some("TIMESTAMP"),
        },
        DuckdbTypeTestCase {
            description: "timezone() is AT TIME ZONE",
            sql: "SELECT timezone('UTC', seen_at) AS v FROM t",
            expected_type: Some("TIMESTAMP"),
        },
        DuckdbTypeTestCase {
            description: "a COALESCE of a TO_TIMESTAMP at a zone and a TIMESTAMP is TIMESTAMP",
            sql: "SELECT MIN(COALESCE(TO_TIMESTAMP(quantity / POWER(10, 3)) AT TIME ZONE 'UTC', created_at)) AS v FROM t",
            expected_type: Some("TIMESTAMP"),
        },
        DuckdbTypeTestCase {
            description: "AT TIME ZONE over a DATE stays untyped",
            sql: "SELECT CAST(created_at AS DATE) AT TIME ZONE 'UTC' AS v FROM t",
            expected_type: None,
        },
    ];
    let schema: TypedShopSchema = TypedShopSchema {
        closed: HashMap::from([(
            "t".to_owned(),
            vec![
                typed_column("quantity", "INTEGER"),
                typed_column("name", "VARCHAR"),
                typed_column("created_at", "TIMESTAMP"),
                typed_column("seen_at", "TIMESTAMPTZ"),
            ],
        )]),
        open: HashMap::new(),
        functions: HashMap::new(),
        rules: HashMap::new(),
        declared: Vec::new(),
        arities: HashMap::new(),
    };
    for test_case in test_cases {
        let statement = sqlquack_syntax_duckdb::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        let context = LineageContext {
            profile: &DUCKDB,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let native: Option<String> = statement_lineage(&statement, context).outputs[0]
            .data_type
            .as_deref()
            .map(str::to_owned);
        assert_eq!(
            (
                duckdb_output_type(&schema, test_case.sql).as_deref(),
                native.as_deref()
            ),
            (test_case.expected_type, test_case.expected_type),
            "{}",
            test_case.description
        );
    }
}
