use std::ops::Range;

use sqlquack_syntax::{Dialect, Expression, ParseOptions};

use crate::dialects::constants::{
    BIGQUERY, CLICKHOUSE, DATABRICKS, DUCKDB, POSTGRES, SNOWFLAKE, SQL_SERVER, SQLITE,
};
use crate::dialects::types::DialectProfile;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::{duckdb_shop_schema, shop_schema};
use crate::lineage::tests::test_types::ShopSchema;

/// A literal with `AS` after a two-byte character, a qualified column, a call with a bare
/// label, a bare column, and a qualified star over the shop's five order columns.
const SQL: &str = "SELECT 'é' AS e, w.status, UPPER(w.status) loud, id, w.* FROM orders AS w";

/// Each output's UTF-8 byte range in `SQL`, counted by hand; the star's five share one.
const RANGES: [Range<usize>; 9] = [
    7..16,
    18..26,
    28..48,
    50..52,
    54..57,
    54..57,
    54..57,
    54..57,
    54..57,
];

struct StrictRangeTestCase {
    dialect: &'static str,
    profile: &'static dyn DialectProfile,
    schema: fn() -> ShopSchema,
    parse_strict: fn(&str) -> Expression,
}

#[test]
fn strict_parses_give_each_output_its_items_byte_range() {
    let test_cases = [
        StrictRangeTestCase {
            dialect: "Snowflake",
            profile: &SNOWFLAKE,
            schema: shop_schema,
            parse_strict: |sql| {
                sqlquack_parse::parse_strict(sql, Dialect::Snowflake, &ParseOptions::default())
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "DuckDB",
            profile: &DUCKDB,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_duckdb::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "Postgres",
            profile: &POSTGRES,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "BigQuery",
            profile: &BIGQUERY,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "SQLite",
            profile: &SQLITE,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_sqlite::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "SQL Server",
            profile: &SQL_SERVER,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_sqlserver::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "Databricks",
            profile: &DATABRICKS,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_databricks::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
        StrictRangeTestCase {
            dialect: "ClickHouse",
            profile: &CLICKHOUSE,
            schema: duckdb_shop_schema,
            parse_strict: |sql| {
                sqlquack_syntax_clickhouse::parse_strict(sql)
                    .unwrap_or_else(|failure| panic!("{failure:?}"))
                    .statements
                    .remove(0)
            },
        },
    ];
    for test_case in test_cases {
        let schema = (test_case.schema)();
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: Some(SQL),
        };
        let ranges: Vec<Option<Range<usize>>> =
            statement_lineage(&(test_case.parse_strict)(SQL), context)
                .outputs
                .into_iter()
                .map(|output| output.expression_range)
                .collect();
        assert_eq!(ranges, RANGES.map(Some).to_vec(), "{}", test_case.dialect);
    }
}
