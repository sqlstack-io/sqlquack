use std::collections::HashMap;

use sqlquack_syntax::Dialect;

use crate::dialects::constants::POSTGRES;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::{LineageContext, Nullability, SchemaColumn, TransformKind};
use crate::lineage::tests::helpers::output_ranges;
use crate::lineage::tests::helpers::{duckdb_shop_schema, typed_column, typed_duckdb_shop_schema};
use crate::lineage::tests::test_types::DialectTypeTestCase;
use crate::lineage::tests::test_types::QueryLineageTestCase;
use crate::lineage::tests::test_types::StarOrderTestCase;
use crate::lineage::tests::test_types::StrictRangeTestCase;
use crate::lineage::tests::test_types::TypedShopSchema;
use crate::postgres::profile::tests::helpers::lineage_outputs;
use crate::postgres::profile::tests::helpers::native_postgres_statement;

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
        let statement = sqlquack_syntax_postgres::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        assert_eq!(
            output_ranges(&statement, &POSTGRES, &duckdb_shop_schema(), test_case.sql),
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

#[test]
fn given_joined_and_comma_items_when_expanding_a_star_then_each_group_places_its_own_keys() {
    let test_cases = [
        StarOrderTestCase {
            description: "a USING key first in its own comma group, after the earlier item",
            sql: "SELECT * FROM customers a, customers b JOIN customers c USING (name)",
            expected_names: &[
                "id", "name", "country", "name", "id", "country", "id", "country",
            ],
        },
        StarOrderTestCase {
            description: "a nested USING key once, first",
            sql: "SELECT * FROM customers a JOIN customers b JOIN customers c USING (name) USING (name)",
            expected_names: &["name", "id", "country", "id", "country", "id", "country"],
        },
    ];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &POSTGRES,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let names: Vec<String> =
            statement_lineage(&native_postgres_statement(test_case.sql), context)
                .outputs
                .into_iter()
                .map(|output| output.name)
                .collect();
        assert_eq!(names, test_case.expected_names, "{}", test_case.description);
    }
}

#[test]
fn given_an_outer_join_after_a_comma_when_tracing_nullability_then_only_its_group_is_nulled() {
    let test_cases = [DialectTypeTestCase {
        description: "a RIGHT JOIN nulls only its own group's left side",
        sql: "SELECT a.name, b.status, c.name FROM customers a, orders b RIGHT JOIN customers c ON b.id = c.id",
        dialect: Dialect::PostgreSQL,
        profile: &POSTGRES,
        expected_outputs: &[
            ("name", Some("TEXT"), Nullability::NonNull),
            ("status", Some("TEXT"), Nullability::Nullable),
            ("name", Some("TEXT"), Nullability::NonNull),
        ],
    }];
    let column = |name: &str| SchemaColumn {
        nullability: Nullability::NonNull,
        ..typed_column(name, "TEXT")
    };
    let schema = TypedShopSchema {
        closed: HashMap::from([
            ("orders".to_owned(), vec![column("id"), column("status")]),
            ("customers".to_owned(), vec![column("id"), column("name")]),
        ]),
        ..TypedShopSchema::default()
    };
    for test_case in test_cases {
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let lineage = statement_lineage(&native_postgres_statement(test_case.sql), context);
        let outputs: Vec<(&str, Option<&str>, Nullability)> = lineage
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
            outputs, test_case.expected_outputs,
            "{}",
            test_case.description
        );
    }
}
