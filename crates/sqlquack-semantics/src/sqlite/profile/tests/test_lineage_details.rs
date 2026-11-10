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
fn given_union_branches_when_tracing_sqlite_lineage_then_only_integers_a_real_column_takes_are_cast()
 {
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
    let test_cases: [(&str, &str, TransformKind, [&str; 2]); 2] = [
        (
            "integers of two declared types keep their values",
            "SELECT id FROM orders UNION ALL SELECT customer_id FROM orders",
            TransformKind::Direct,
            ["orders.customer_id", "orders.id"],
        ),
        (
            "a compound read through a subquery turns integers into its REAL column's reals",
            "SELECT x FROM (SELECT amount AS x FROM orders UNION ALL SELECT id FROM orders)",
            TransformKind::Cast,
            ["orders.amount", "orders.id"],
        ),
    ];
    for (description, sql, kind, sources) in test_cases {
        let context = LineageContext {
            profile: &SQLITE,
            schema: &schema,
            sql: None,
        };
        let traced: Vec<(TransformKind, Vec<String>)> =
            query_lineage(sql, Dialect::SQLite, &ParseOptions::default(), context)
                .unwrap_or_else(|error| panic!("{sql}: {error}"))
                .outputs
                .iter()
                .map(|output| {
                    let read = output.sources.iter();
                    (
                        output.kind,
                        read.map(|source| format!("{}.{}", source.relation, source.column))
                            .collect(),
                    )
                })
                .collect();
        let expected = vec![(kind, sources.map(str::to_owned).to_vec())];
        assert_eq!(traced, expected, "{description}");
    }
}
