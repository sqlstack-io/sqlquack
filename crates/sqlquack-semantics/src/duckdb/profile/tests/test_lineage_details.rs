use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::DUCKDB;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::duckdb_shop_schema;
use crate::lineage::tests::helpers::output_ranges;
use crate::lineage::tests::helpers::{shop_schema, written};
use crate::lineage::tests::test_types::LineageDetailTestCase;
use crate::lineage::tests::test_types::StrictRangeTestCase;

#[test]
fn given_a_query_when_tracing_lineage_then_values_rows_and_untraced_outputs_are_reported() {
    let test_cases = [
        LineageDetailTestCase {
            description: "DuckDB's star keeps a USING key where the left side offers it",
            sql: "SELECT * FROM ORDERS JOIN (SELECT id AS customer_id, name FROM CUSTOMERS) c \
                  USING (customer_id)",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("ID", TransformKind::Direct, &["ORDERS.ID"], true),
                (
                    "CUSTOMER_ID",
                    TransformKind::Direct,
                    &["ORDERS.CUSTOMER_ID"],
                    true,
                ),
                ("AMOUNT", TransformKind::Direct, &["ORDERS.AMOUNT"], true),
                ("ITEMS", TransformKind::Direct, &["ORDERS.ITEMS"], true),
                ("STATUS", TransformKind::Direct, &["ORDERS.STATUS"], true),
                ("name", TransformKind::Direct, &["CUSTOMERS.NAME"], true),
            ],
            expected_row_sources: &["CUSTOMERS.ID", "ORDERS.CUSTOMER_ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "DuckDB's WINDOW clause adds what the named window reads",
            sql: "SELECT SUM(amount) OVER w AS running FROM orders \
                  WINDOW w AS (PARTITION BY customer_id ORDER BY id)",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[(
                "running",
                TransformKind::Aggregation,
                &["orders.amount", "orders.customer_id", "orders.id"],
                true,
            )],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "an unaliased DuckDB cast has no name lineage models",
            sql: "SELECT amount::INT FROM orders",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[("", TransformKind::Cast, &["orders.amount"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
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
        let outputs: Vec<(String, TransformKind, Vec<String>, bool)> = lineage
            .outputs
            .iter()
            .map(|output| {
                (
                    output.name.clone(),
                    output.kind,
                    output.sources.iter().map(written).collect(),
                    output.resolved,
                )
            })
            .collect();
        let expected: Vec<(String, TransformKind, Vec<String>, bool)> = test_case
            .expected_outputs
            .iter()
            .map(|(name, kind, sources, resolved)| {
                (
                    (*name).to_owned(),
                    *kind,
                    sources.iter().map(|source| (*source).to_owned()).collect(),
                    *resolved,
                )
            })
            .collect();
        let rows: Vec<String> = lineage.row_sources.iter().map(written).collect();
        assert_eq!(outputs, expected, "{}", test_case.description);
        assert_eq!(
            rows, test_case.expected_row_sources,
            "{}",
            test_case.description
        );
        assert_eq!(
            lineage.rows_resolved, test_case.expected_rows_resolved,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_strict_parse_when_tracing_lineage_then_each_output_has_its_items_byte_range() {
    let test_cases = [StrictRangeTestCase {
        description: "a literal aliased after a two-byte character, a qualified column, a labelled call, a bare column and a qualified star",
        sql: "SELECT 'é' AS e, w.status, UPPER(w.status) loud, id, w.* FROM orders AS w",
        expected_ranges: &[
            7..16,
            18..26,
            28..48,
            50..52,
            54..57,
            54..57,
            54..57,
            54..57,
            54..57,
        ],
    }];
    for test_case in test_cases {
        let statement = sqlquack_syntax_duckdb::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        assert_eq!(
            output_ranges(&statement, &DUCKDB, &duckdb_shop_schema(), test_case.sql),
            test_case
                .expected_ranges
                .iter()
                .cloned()
                .map(Some)
                .collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
}
