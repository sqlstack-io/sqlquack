use std::collections::HashMap;

use sqlquack_syntax::Dialect;

use crate::dialects::constants::POSTGRES;
use crate::lineage::models::TransformKind;
use crate::lineage::tests::helpers::{duckdb_shop_schema, typed_column, typed_duckdb_shop_schema};
use crate::lineage::tests::test_types::QueryLineageTestCase;
use crate::lineage::tests::test_types::TypedShopSchema;
use crate::postgres::profile::tests::helpers::lineage_outputs;

#[test]
fn given_unaliased_postgres_outputs_when_tracing_lineage_then_each_takes_figurecolnames_name() {
    let test_cases = [QueryLineageTestCase {
        description: "each output takes the name 18.6 reports for it in RowDescription",
        sql: "SELECT id, amount + 1, count(*), CAST(amount AS int), status::text, \
                  CASE WHEN true THEN 1 END, upper(status), 1, (SELECT max(id) FROM orders), \
                  DATE '2024-01-01', trim(status) FROM orders GROUP BY id, amount, status",
        dialect: Dialect::PostgreSQL,
        profile: &POSTGRES,
        expected_outputs: &[
            ("id", TransformKind::Direct, &["orders.id"]),
            ("?column?", TransformKind::Expression, &["orders.amount"]),
            ("count", TransformKind::Aggregation, &[]),
            ("amount", TransformKind::Cast, &["orders.amount"]),
            ("status", TransformKind::Cast, &["orders.status"]),
            ("case", TransformKind::Constant, &[]),
            ("upper", TransformKind::Expression, &["orders.status"]),
            ("?column?", TransformKind::Constant, &[]),
            ("max", TransformKind::Aggregation, &["orders.id"]),
            ("date", TransformKind::Constant, &[]),
            ("btrim", TransformKind::Expression, &["orders.status"]),
        ],
        expected_open_star: false,
    }];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        let traced = lineage_outputs(
            test_case.sql,
            (test_case.dialect, test_case.profile),
            &schema,
        );
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
fn given_postgres_set_operations_when_tracing_lineage_then_outputs_take_the_strongest_branch() {
    let test_cases = [
        QueryLineageTestCase {
            description: "a union of pass-through CTE columns from two relations is direct",
            sql: "WITH recent AS (SELECT id FROM orders WHERE quantity > 0), \
                  older AS (SELECT id FROM customers) \
                  SELECT id FROM recent UNION ALL SELECT id FROM older",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[("id", TransformKind::Direct, &["customers.id", "orders.id"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a branch the union implicitly casts to numeric makes the output a cast",
            sql: "SELECT quantity FROM orders UNION ALL SELECT price FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[(
                "quantity",
                TransformKind::Cast,
                &["orders.price", "orders.quantity"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a computed branch outranks a cast one",
            sql: "SELECT quantity + 1 AS n FROM orders UNION ALL SELECT price FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[(
                "n",
                TransformKind::Expression,
                &["orders.price", "orders.quantity"],
            )],
            expected_open_star: false,
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        let traced = lineage_outputs(
            test_case.sql,
            (test_case.dialect, test_case.profile),
            &schema,
        );
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
fn given_union_branches_postgres_does_not_convert_when_tracing_lineage_then_no_cast_is_reported() {
    let schema = TypedShopSchema {
        closed: HashMap::from([(
            "orders".to_owned(),
            vec![
                typed_column("id", "int4"),
                typed_column("customer_id", "integer"),
                typed_column("status", "varchar(10)"),
            ],
        )]),
        ..TypedShopSchema::default()
    };
    let test_cases = [
        QueryLineageTestCase {
            description: "one type spelled two ways",
            sql: "SELECT id FROM orders UNION ALL SELECT customer_id FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[(
                "id",
                TransformKind::Direct,
                &["orders.customer_id", "orders.id"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a literal branch, which drops only the column's length",
            sql: "SELECT status FROM orders UNION ALL SELECT 'x'",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[("status", TransformKind::Direct, &["orders.status"])],
            expected_open_star: false,
        },
    ];
    for test_case in test_cases {
        let traced = lineage_outputs(
            test_case.sql,
            (test_case.dialect, test_case.profile),
            &schema,
        );
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
fn given_a_union_of_untyped_columns_when_tracing_postgres_lineage_then_it_stays_an_expression() {
    let test_cases = [QueryLineageTestCase {
        description: "branches of unknown type merged from two columns stay an expression",
        sql: "SELECT id FROM orders UNION ALL SELECT customer_id FROM orders",
        dialect: Dialect::PostgreSQL,
        profile: &POSTGRES,
        expected_outputs: &[(
            "id",
            TransformKind::Expression,
            &["orders.customer_id", "orders.id"],
        )],
        expected_open_star: false,
    }];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        let traced = lineage_outputs(
            test_case.sql,
            (test_case.dialect, test_case.profile),
            &schema,
        );
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
