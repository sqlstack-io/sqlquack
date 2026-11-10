use std::collections::HashMap;

use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::SQLITE;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::{shop_schema, typed_column, written};
use crate::lineage::tests::test_types::{LineageDetailTestCase, TypedShopSchema};

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
fn given_union_branches_of_differing_types_when_tracing_sqlite_lineage_then_no_cast_is_reported() {
    let schema = TypedShopSchema {
        closed: HashMap::from([(
            "orders".to_owned(),
            vec![
                typed_column("id", "INTEGER"),
                typed_column("customer_id", "BIGINT"),
            ],
        )]),
        ..TypedShopSchema::default()
    };
    let context = LineageContext {
        profile: &SQLITE,
        schema: &schema,
        sql: None,
    };
    let sql = "SELECT id FROM orders UNION ALL SELECT customer_id FROM orders";
    let traced: Vec<(String, TransformKind, Vec<String>)> =
        query_lineage(sql, Dialect::SQLite, &ParseOptions::default(), context)
            .unwrap_or_else(|error| panic!("{sql}: {error}"))
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
    assert_eq!(
        traced,
        vec![(
            "id".to_owned(),
            TransformKind::Direct,
            vec!["orders.customer_id".to_owned(), "orders.id".to_owned()],
        )]
    );
}
