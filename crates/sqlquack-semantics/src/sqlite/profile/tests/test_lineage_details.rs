use std::collections::HashMap;

use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::SQLITE;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::duckdb_shop_schema;
use crate::lineage::tests::helpers::output_ranges;
use crate::lineage::tests::helpers::{shop_schema, typed_column, written};
use crate::lineage::tests::test_types::StrictRangeTestCase;
use crate::lineage::tests::test_types::{
    LineageDetailTestCase, QueryLineageTestCase, TypedShopSchema,
};

#[test]
fn given_a_query_when_tracing_lineage_then_values_rows_and_untraced_outputs_are_reported() {
    let test_cases = [LineageDetailTestCase {
        description: "SQLite's star keeps a FULL join's USING key in place, over both sides",
        sql: "SELECT * FROM (SELECT name, id FROM CUSTOMERS) c FULL JOIN ORDERS USING (id)",
        dialect: Dialect::SQLite,
        profile: &SQLITE,
        expected_outputs: &[
            ("name", TransformKind::Direct, &["CUSTOMERS.NAME"], true),
            (
                "id",
                TransformKind::Expression,
                &["CUSTOMERS.ID", "ORDERS.ID"],
                true,
            ),
            (
                "CUSTOMER_ID",
                TransformKind::Direct,
                &["ORDERS.CUSTOMER_ID"],
                true,
            ),
            ("AMOUNT", TransformKind::Direct, &["ORDERS.AMOUNT"], true),
            ("ITEMS", TransformKind::Direct, &["ORDERS.ITEMS"], true),
            ("STATUS", TransformKind::Direct, &["ORDERS.STATUS"], true),
        ],
        expected_row_sources: &["CUSTOMERS.ID", "ORDERS.ID"],
        expected_rows_resolved: true,
    }];
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
fn given_union_branches_when_tracing_sqlite_lineage_then_only_integers_a_real_column_takes_are_cast()
 {
    let test_cases = [
        QueryLineageTestCase {
            description: "integers of two declared types keep their values",
            sql: "SELECT id FROM orders UNION ALL SELECT customer_id FROM orders",
            dialect: Dialect::SQLite,
            profile: &SQLITE,
            expected_outputs: &[(
                "id",
                TransformKind::Direct,
                &["orders.customer_id", "orders.id"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a compound read through a subquery turns integers into its REAL column's reals",
            sql: "SELECT x FROM (SELECT amount AS x FROM orders UNION ALL SELECT id FROM orders)",
            dialect: Dialect::SQLite,
            profile: &SQLITE,
            expected_outputs: &[("x", TransformKind::Cast, &["orders.amount", "orders.id"])],
            expected_open_star: false,
        },
    ];
    let schema = TypedShopSchema {
        closed: HashMap::from([(
            "orders".to_owned(),
            vec![
                typed_column("id", "INTEGER"),
                typed_column("customer_id", "BIGINT"),
                typed_column("amount", "REAL"),
            ],
        )]),
        ..TypedShopSchema::default()
    };
    for test_case in test_cases {
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: None,
        };
        let traced: Vec<(String, TransformKind, Vec<String>)> = query_lineage(
            test_case.sql,
            test_case.dialect,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", test_case.description))
        .outputs
        .iter()
        .map(|output| {
            let sources = output.sources.iter();
            (
                output.name.clone(),
                output.kind,
                sources
                    .map(|source| format!("{}.{}", source.relation, source.column))
                    .collect(),
            )
        })
        .collect();
        let traced: Vec<(&str, TransformKind, Vec<&str>)> = traced
            .iter()
            .map(|(name, kind, sources)| {
                (
                    name.as_str(),
                    *kind,
                    sources.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            traced,
            test_case
                .expected_outputs
                .iter()
                .map(|(name, kind, sources)| (*name, *kind, sources.to_vec()))
                .collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_strict_parse_when_tracing_lineage_then_each_output_has_its_items_byte_range() {
    let test_cases = [StrictRangeTestCase {
        description: "a literal aliased after a two-byte character, a qualified column, a labelled call and a bare column",
        sql: "SELECT 'é' AS e, w.status, UPPER(w.status) loud, id FROM orders AS w",
        expected_ranges: &[7..16, 18..26, 28..48, 50..52],
    }];
    for test_case in test_cases {
        let statement = sqlquack_syntax_sqlite::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        assert_eq!(
            output_ranges(&statement, &SQLITE, &duckdb_shop_schema(), test_case.sql),
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
