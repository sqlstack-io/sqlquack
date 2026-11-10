use sqlquack_syntax::Dialect;

use crate::dialects::constants::POSTGRES;
use crate::lineage::models::TransformKind;
use crate::lineage::tests::helpers::{duckdb_shop_schema, typed_duckdb_shop_schema};
use crate::lineage::tests::test_types::QueryLineageTestCase;
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
        assert_eq!(traced, expected, "{}", test_case.description);
    }
}
