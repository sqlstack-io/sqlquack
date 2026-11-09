use std::collections::HashMap;

use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::POSTGRES;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::Nullability::{NonNull, Unknown};
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::{
    DialectTypeTestCase, EngineTypeReplayTestCase, TypedShopSchema,
};
use crate::postgres::profile::tests::helpers::canonical_postgres;
use crate::postgres::profile::tests::helpers::native_postgres_output_type;
use crate::postgres::profile::tests::helpers::replayed_native_postgres_engine_types;
use crate::postgres::profile::tests::helpers::replayed_postgres_engine_types;
use crate::postgres::profile::tests::helpers::typed_postgres_shop_schema;
use crate::postgres::profile::tests::test_types::PostgresTypeTestCase;

/// Result types the PostgreSQL 18.6 server bound for synthetic expressions, recorded through views.
const POSTGRES_ENGINE_TYPES: &str = include_str!("fixtures/postgres_engine_types.json");

/// Result types the PostgreSQL 18.6 server bound for the accepted query-shaped syntax cases.
const POSTGRES_SYNTAX_TYPES: &str = include_str!("fixtures/postgres_syntax_types.json");

#[test]
fn given_a_postgres_query_when_typing_outputs_then_types_follow_postgresql_18_6() {
    let test_cases = [
        DialectTypeTestCase {
            description: "literals, built-in calls, value functions and common types (18.6 format_type of a view)",
            sql: "SELECT 'lit' AS a, upper(status) AS b, status || 'x' AS c, length(status) AS d, \
              round(price) AS e, sqrt(quantity) AS f, coalesce(status, 'x') AS g, max(status) AS h, \
              sum(quantity) AS i, avg(quantity) AS j, now() AS k, current_date AS l, \
              CASE WHEN true THEN 'x' ELSE 'y' END AS m FROM orders GROUP BY status, price, quantity",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[
                ("a", Some("TEXT"), NonNull),
                ("b", Some("TEXT"), Unknown),
                ("c", Some("TEXT"), Unknown),
                ("d", Some("INT"), Unknown),
                ("e", Some("NUMERIC"), Unknown),
                ("f", Some("DOUBLE"), Unknown),
                ("g", Some("VARCHAR"), NonNull),
                ("h", Some("TEXT"), Unknown),
                ("i", Some("BIGINT"), Unknown),
                ("j", Some("NUMERIC"), Unknown),
                ("k", Some("TIMESTAMPTZ"), Unknown),
                ("l", Some("DATE"), Unknown),
                ("m", Some("TEXT"), Unknown),
            ],
        },
        DialectTypeTestCase {
            description: "aggregates under integer division and COALESCE stay bigint (18.6 format_type of a view)",
            sql: "SELECT sum(quantity) / count(DISTINCT id) AS a, coalesce(sum(quantity), 0) AS b FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[
                ("a", Some("BIGINT"), Unknown),
                ("b", Some("BIGINT"), NonNull),
            ],
        },
        DialectTypeTestCase {
            description: "operators, NULLIF, casts and literals resolve through the catalogue (18.6 format_type of a view)",
            sql: "SELECT -price AS c, quantity * 1.5 AS d, nullif(quantity, price) AS e, CAST(price AS real) AS f, \
              3000000000 AS g, CASE WHEN true THEN status ELSE 'x' END AS h FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[
                ("c", Some("NUMERIC"), Unknown),
                ("d", Some("NUMERIC"), Unknown),
                ("e", Some("NUMERIC"), Unknown),
                ("f", Some("REAL"), Unknown),
                ("g", Some("BIGINT"), NonNull),
                ("h", Some("VARCHAR"), Unknown),
            ],
        },
    ];
    let schema = typed_postgres_shop_schema();
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

#[test]
fn given_a_native_postgres_tree_when_typing_outputs_then_types_follow_postgresql_18_6() {
    let test_cases = [
        PostgresTypeTestCase {
            description: "an array joined to an array keeps the array type",
            sql: "SELECT c_int_array || c_int_array AS v FROM t",
            expected_type: Some("integer[]"),
        },
        PostgresTypeTestCase {
            description: "an array joined to its element keeps the array type",
            sql: "SELECT c_int_array || c_integer AS v FROM t",
            expected_type: Some("integer[]"),
        },
        PostgresTypeTestCase {
            description: "jsonb joined to jsonb is jsonb",
            sql: "SELECT c_jsonb || c_jsonb AS v FROM t",
            expected_type: Some("jsonb"),
        },
        PostgresTypeTestCase {
            description: "bytea joined to bytea is bytea",
            sql: "SELECT c_bytea || c_bytea AS v FROM t",
            expected_type: Some("bytea"),
        },
        PostgresTypeTestCase {
            description: "lag drops its argument's length",
            sql: "SELECT lag(c_varchar) OVER () AS v FROM t",
            expected_type: Some("character varying"),
        },
        PostgresTypeTestCase {
            description: "lead drops its argument's precision and scale",
            sql: "SELECT lead(c_num10_2, 1, c_num10_2) OVER () AS v FROM t",
            expected_type: Some("numeric"),
        },
        PostgresTypeTestCase {
            description: "lower of a range is its element type",
            sql: "SELECT lower(int4range(1, 5)) AS v",
            expected_type: Some("integer"),
        },
        PostgresTypeTestCase {
            description: "upper of a multirange is its element type",
            sql: "SELECT upper(int8multirange(int8range(1, 5))) AS v",
            expected_type: Some("bigint"),
        },
        PostgresTypeTestCase {
            description: "a hexadecimal literal that fits four bytes is integer",
            sql: "SELECT 0x1F AS v",
            expected_type: Some("integer"),
        },
        PostgresTypeTestCase {
            description: "a separated hexadecimal literal past four bytes is bigint",
            sql: "SELECT 0x_FFFF_FFFF AS v",
            expected_type: Some("bigint"),
        },
        PostgresTypeTestCase {
            description: "a binary literal is integer",
            sql: "SELECT 0b101 AS v",
            expected_type: Some("integer"),
        },
        PostgresTypeTestCase {
            description: "current_timestamp keeps the precision it is given",
            sql: "SELECT current_timestamp(3) AS v",
            expected_type: Some("timestamp(3) with time zone"),
        },
        PostgresTypeTestCase {
            description: "localtime keeps the precision it is given",
            sql: "SELECT localtime(2) AS v",
            expected_type: Some("time(2) without time zone"),
        },
        PostgresTypeTestCase {
            description: "a cast to a two-dimensional array is the one array type",
            sql: "SELECT CAST(c_int_array AS integer[][]) AS v FROM t",
            expected_type: Some("integer[]"),
        },
        PostgresTypeTestCase {
            description: "a cast to unknown abstains: a view column of it is text",
            sql: "SELECT CAST(NULL AS unknown) AS v",
            expected_type: None,
        },
    ];
    let schema: TypedShopSchema = TypedShopSchema {
        closed: HashMap::from([(
            "t".to_owned(),
            vec![
                typed_column("c_integer", "integer"),
                typed_column("c_varchar", "varchar(20)"),
                typed_column("c_num10_2", "numeric(10,2)"),
                typed_column("c_jsonb", "jsonb"),
                typed_column("c_bytea", "bytea"),
                typed_column("c_int_array", "integer[]"),
            ],
        )]),
        ..TypedShopSchema::default()
    };
    for test_case in test_cases {
        assert_eq!(
            native_postgres_output_type(&schema, test_case.sql)
                .as_deref()
                .map(canonical_postgres),
            test_case.expected_type.map(canonical_postgres),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_engine_recorded_postgres_types_when_typing_outputs_then_none_is_wrong() {
    let test_cases = [
        EngineTypeReplayTestCase {
            description: "every recorded PostgreSQL 18.6 result type, replayed",
            fixture: POSTGRES_ENGINE_TYPES,
            replayed: replayed_postgres_engine_types,
            expected_wrong: &[],
            expected_minimum_typed: 1322,
            expected_maximum_untyped: 45,
        },
        EngineTypeReplayTestCase {
            description: "every recorded PostgreSQL 18.6 result type, replayed through the native parser",
            fixture: POSTGRES_ENGINE_TYPES,
            replayed: replayed_native_postgres_engine_types,
            expected_wrong: &[],
            expected_minimum_typed: 1345,
            expected_maximum_untyped: 22,
        },
        EngineTypeReplayTestCase {
            description: "every typed query of the PostgreSQL syntax cases, replayed through the native parser \
              (a query the parser cannot read whole counts as untyped)",
            fixture: POSTGRES_SYNTAX_TYPES,
            replayed: replayed_native_postgres_engine_types,
            expected_wrong: &[],
            expected_minimum_typed: 683,
            expected_maximum_untyped: 276,
        },
    ];
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
