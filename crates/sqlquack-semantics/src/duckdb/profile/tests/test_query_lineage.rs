use crate::duckdb::profile::tests::test_types::ResolvedReadTestCase;

use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::tests::helpers::typed_duckdb_shop_schema;
use crate::lineage::tests::test_types::TypedShopSchema;

use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::DUCKDB;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::shop_schema;
use crate::lineage::tests::test_types::QueryLineageTestCase;

#[test]
fn given_a_query_when_tracing_lineage_then_each_output_names_its_sources_and_transform() {
    let test_cases = [
        QueryLineageTestCase {
            description: "DuckDB names a pivoted column by its value's text, without quotes",
            sql: "SELECT * FROM (SELECT customer_id, status, amount FROM orders) PIVOT (SUM(amount) FOR status IN ('open', 'shipped'))",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["orders.customer_id"],
                ),
                ("open", TransformKind::Aggregation, &["orders.amount"]),
                ("shipped", TransformKind::Aggregation, &["orders.amount"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a DuckDB PIVOT with GROUP BY passes only the grouped columns through",
            sql: "SELECT * FROM (SELECT id, customer_id, status, amount FROM orders) \
                  PIVOT (SUM(amount) FOR status IN ('open') GROUP BY customer_id)",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["orders.customer_id"],
                ),
                ("open", TransformKind::Aggregation, &["orders.amount"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a DuckDB PIVOT statement keeps its GROUP BY columns; its pivoted ones are unnamed",
            sql: "PIVOT (SELECT id, customer_id, status, amount FROM orders) ON status USING SUM(amount) \
                  GROUP BY customer_id",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[(
                "customer_id",
                TransformKind::Direct,
                &["orders.customer_id"],
            )],
            expected_open_star: true,
        },
        QueryLineageTestCase {
            description: "DuckDB names VALUES columns col0 onwards and ignores case",
            sql: "SELECT v.col0 AS first, id FROM (VALUES (1)) v, ORDERS",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("first", TransformKind::Constant, &[]),
                ("id", TransformKind::Direct, &["ORDERS.ID"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "DuckDB names an unaliased number by its text",
            sql: "SELECT 1, id FROM ORDERS",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("1", TransformKind::Constant, &[]),
                ("id", TransformKind::Direct, &["ORDERS.ID"]),
            ],
            expected_open_star: false,
        },
    ];
    let schema = shop_schema();
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
        .unwrap_or_else(|error| panic!("{}: {error}", test_case.description));
        let outputs: Vec<(String, TransformKind, Vec<String>)> = lineage
            .outputs
            .iter()
            .map(|output| {
                (
                    output.name.clone(),
                    output.kind,
                    output
                        .sources
                        .iter()
                        .map(|source| format!("{}.{}", source.relation, source.column))
                        .collect(),
                )
            })
            .collect();
        let expected: Vec<(String, TransformKind, Vec<String>)> = test_case
            .expected_outputs
            .iter()
            .map(|(name, kind, sources)| {
                (
                    (*name).to_owned(),
                    *kind,
                    sources.iter().map(|source| (*source).to_owned()).collect(),
                )
            })
            .collect();
        assert_eq!(outputs, expected, "{}", test_case.description);
        assert_eq!(
            lineage.open_star, test_case.expected_open_star,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_duckdb_value_keyword_beside_an_open_relation_when_tracing_lineage_then_it_reads_no_column()
 {
    let test_cases = [
        ResolvedReadTestCase {
            description: "CURRENT_TIMESTAMP is DuckDB's function, not a column of a star over an open relation",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT CURRENT_TIMESTAMP AS loaded_at FROM src",
            expected_sources: &[],
            expected_resolved: true,
        },
        ResolvedReadTestCase {
            description: "current_date in lower case is the function too",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT current_date AS loaded_on FROM src",
            expected_sources: &[],
            expected_resolved: true,
        },
        ResolvedReadTestCase {
            description: "a qualified keyword is a column of the star over the open relation",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT src.current_date AS loaded_on FROM src",
            expected_sources: &["raw_orders.current_date"],
            expected_resolved: true,
        },
        ResolvedReadTestCase {
            description: "an ordinary name is a column of the star over the open relation",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT status FROM src",
            expected_sources: &["raw_orders.status"],
            expected_resolved: true,
        },
    ];
    let mut schema: TypedShopSchema = typed_duckdb_shop_schema();
    schema.open.insert("raw_orders".to_owned(), Vec::new());
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
        let lineage = statement_lineage(&statement, context);
        let output = &lineage.outputs[0];
        let sources: Vec<String> = output
            .sources
            .iter()
            .map(|source| format!("{}.{}", source.relation, source.column))
            .collect();
        assert_eq!(
            (sources, output.resolved),
            (
                test_case
                    .expected_sources
                    .iter()
                    .map(|source| (*source).to_owned())
                    .collect::<Vec<String>>(),
                test_case.expected_resolved
            ),
            "{}",
            test_case.description
        );
    }
}
