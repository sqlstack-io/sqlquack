use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::{DUCKDB, POSTGRES, SNOWFLAKE};
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::{
    duckdb_shop_schema, lineage_outputs, shop_schema, typed_duckdb_shop_schema,
    typed_postgres_shop_schema, typed_shop_schema,
};
use crate::lineage::tests::test_types::TypedShopSchema;
use crate::lineage::tests::test_types::{QueryLineageTestCase, ResolvedReadTestCase};

#[test]
fn given_a_query_when_tracing_lineage_then_each_output_names_its_sources_and_transform() {
    let test_cases = [
        QueryLineageTestCase {
            description: "a qualified column is direct and an arithmetic one an expression",
            sql: "SELECT o.id, amount * 2 AS doubled FROM orders o",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", TransformKind::Direct, &["ORDERS.ID"]),
                ("doubled", TransformKind::Expression, &["ORDERS.AMOUNT"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a star in a CTE carries the schema's columns to a cast outside",
            sql: "WITH c AS (SELECT * FROM orders) SELECT id::INT AS id FROM c",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Cast, &["ORDERS.ID"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "aggregates are aggregations, COUNT(*) without sources",
            sql: "SELECT customer_id, SUM(amount) AS total, COUNT(*) AS n FROM orders GROUP BY 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["ORDERS.CUSTOMER_ID"],
                ),
                ("total", TransformKind::Aggregation, &["ORDERS.AMOUNT"]),
                ("n", TransformKind::Aggregation, &[]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a relation the schema does not know offers any column by its written name",
            sql: "SELECT ticket_id FROM support.tickets",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "ticket_id",
                TransformKind::Direct,
                &["SUPPORT.TICKETS.ticket_id"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union output reads every branch's column at its position, direct when each passes one through",
            sql: "SELECT id FROM orders UNION ALL SELECT sku FROM shop.products",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "id",
                TransformKind::Direct,
                &["ORDERS.ID", "SHOP.PRODUCTS.SKU"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "an unqualified column resolves to the joined relation that has it",
            sql: "SELECT name, o.id FROM orders o JOIN customers c ON c.id = o.customer_id",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("name", TransformKind::Direct, &["CUSTOMERS.NAME"]),
                ("id", TransformKind::Direct, &["ORDERS.ID"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a FLATTEN column derives from the flattened input",
            sql: "SELECT f.value:sku::VARCHAR AS sku FROM orders, LATERAL FLATTEN(input => orders.items) f",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("sku", TransformKind::Expression, &["ORDERS.ITEMS"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a correlated scalar subquery reads what its output reads",
            sql: "SELECT id, (SELECT MAX(amount) FROM orders o2 WHERE o2.customer_id = c.id) AS top \
                  FROM customers c",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", TransformKind::Direct, &["CUSTOMERS.ID"]),
                ("top", TransformKind::Aggregation, &["ORDERS.AMOUNT"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union by name matches columns by name, not position",
            sql: "SELECT id, status FROM orders UNION ALL BY NAME SELECT country AS status, id FROM customers",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", TransformKind::Direct, &["CUSTOMERS.ID", "ORDERS.ID"]),
                (
                    "status",
                    TransformKind::Direct,
                    &["CUSTOMERS.COUNTRY", "ORDERS.STATUS"],
                ),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a window value reads its partition and order columns too",
            sql: "SELECT ROW_NUMBER() OVER (PARTITION BY customer_id ORDER BY amount) AS rank_in_customer, \
                  SUM(amount) OVER (PARTITION BY customer_id) AS customer_total FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                (
                    "rank_in_customer",
                    TransformKind::Window,
                    &["ORDERS.AMOUNT", "ORDERS.CUSTOMER_ID"],
                ),
                (
                    "customer_total",
                    TransformKind::Window,
                    &["ORDERS.AMOUNT", "ORDERS.CUSTOMER_ID"],
                ),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a cast of COUNT(*) stays an aggregation",
            sql: "SELECT COUNT(*)::INT AS n FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("n", TransformKind::Aggregation, &[])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a PIVOT passes other columns through and aggregates into each value",
            sql: "SELECT p.customer_id, p.open_amount FROM (SELECT customer_id, status, amount FROM orders) \
                  PIVOT (SUM(amount) FOR status IN ('open', 'shipped')) AS p(customer_id, open_amount, shipped_amount)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["ORDERS.CUSTOMER_ID"],
                ),
                (
                    "open_amount",
                    TransformKind::Aggregation,
                    &["ORDERS.AMOUNT"],
                ),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a comment before a projection does not hide it",
            sql: "WITH c AS (SELECT id FROM orders) SELECT\n  -- keys\n  id\nFROM c",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a star read as a value reads every column it selects",
            sql: "SELECT OBJECT_CONSTRUCT(c.* EXCLUDE (country)) AS payload FROM customers c",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "payload",
                TransformKind::Expression,
                &["CUSTOMERS.ID", "CUSTOMERS.NAME"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a comment between union branches does not hide them",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM customers\n-- products\nUNION ALL SELECT sku FROM shop.products",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "id",
                TransformKind::Direct,
                &["CUSTOMERS.ID", "ORDERS.ID", "SHOP.PRODUCTS.SKU"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "SPLIT_TO_TABLE columns derive from the split text",
            sql: "SELECT s.value AS tag, s.index AS position FROM orders, LATERAL SPLIT_TO_TABLE(orders.status, ',') AS s",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("tag", TransformKind::Expression, &["ORDERS.STATUS"]),
                ("position", TransformKind::Expression, &["ORDERS.STATUS"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a user table function's columns come from the function",
            sql: "SELECT r.ticket_id FROM TABLE(support_tickets_for(1)) AS r",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "ticket_id",
                TransformKind::Direct,
                &["SUPPORT_TICKETS_FOR.ticket_id"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a quoted stored column matches only its exact quoted spelling; a column the relation lacks is still read from it",
            sql: "SELECT e.\"listingId\" AS listing, e.listingid AS unmatched, e.listing_status FROM exports e",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("listing", TransformKind::Direct, &["EXPORTS.listingId"]),
                ("unmatched", TransformKind::Direct, &["EXPORTS.listingid"]),
                (
                    "listing_status",
                    TransformKind::Direct,
                    &["EXPORTS.LISTING_STATUS"],
                ),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "EXCEPT outputs carry only the left branch's values",
            sql: "SELECT id FROM orders EXCEPT SELECT id FROM customers",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "UNPIVOT passes other columns through and its value reads every unpivoted one",
            sql: "SELECT u.id, u.field, u.reading FROM customers UNPIVOT (reading FOR field IN (name, country)) AS u",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", TransformKind::Direct, &["CUSTOMERS.ID"]),
                ("field", TransformKind::Constant, &[]),
                (
                    "reading",
                    TransformKind::Direct,
                    &["CUSTOMERS.COUNTRY", "CUSTOMERS.NAME"],
                ),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a PIVOT over ANY values has columns not known by name",
            sql: "SELECT * FROM (SELECT customer_id, status, amount FROM orders) PIVOT (SUM(amount) FOR status IN (ANY ORDER BY status))",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "customer_id",
                TransformKind::Direct,
                &["ORDERS.CUSTOMER_ID"],
            )],
            expected_open_star: true,
        },
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
            description: "a pivoted column is named by its value exactly, quotes included",
            sql: "SELECT p.\"'open'\" AS open_amount FROM (SELECT customer_id, status, amount FROM orders) \
                  PIVOT (SUM(amount) FOR status IN ('open')) AS p",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "open_amount",
                TransformKind::Aggregation,
                &["ORDERS.AMOUNT"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a literal is a constant",
            sql: "SELECT 1 AS one",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("one", TransformKind::Constant, &[])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a star expands the schema's columns less EXCLUDE, with RENAME",
            sql: "SELECT * EXCLUDE (country) RENAME (name AS customer_name) FROM customers",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("ID", TransformKind::Direct, &["CUSTOMERS.ID"]),
                ("customer_name", TransformKind::Direct, &["CUSTOMERS.NAME"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a star over an unknown relation is left open",
            sql: "SELECT * FROM support.tickets",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[],
            expected_open_star: true,
        },
        QueryLineageTestCase {
            description: "a select alias read by a later projection keeps its lineage",
            sql: "SELECT amount * 2 AS doubled, doubled + 1 AS next FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("doubled", TransformKind::Expression, &["ORDERS.AMOUNT"]),
                ("next", TransformKind::Expression, &["ORDERS.AMOUNT"]),
            ],
            expected_open_star: false,
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
fn given_a_column_its_relation_lacks_when_tracing_lineage_then_one_stored_relation_is_read_untraced()
 {
    let test_cases = [
        ResolvedReadTestCase {
            description: "a qualified column a stored relation lacks",
            sql: "SELECT o.qty FROM orders o",
            expected_sources: &["ORDERS.qty"],
            expected_resolved: false,
        },
        ResolvedReadTestCase {
            description: "a qualified column of a derived table it lacks",
            sql: "SELECT d.qty FROM (SELECT id FROM orders) d",
            expected_sources: &[],
            expected_resolved: false,
        },
        ResolvedReadTestCase {
            description: "an unqualified column the one stored relation lacks",
            sql: "SELECT qty FROM orders",
            expected_sources: &["ORDERS.qty"],
            expected_resolved: false,
        },
        ResolvedReadTestCase {
            description: "an unqualified column two stored relations lack",
            sql: "SELECT qty FROM orders, orders AS other",
            expected_sources: &[],
            expected_resolved: false,
        },
        ResolvedReadTestCase {
            description: "a qualified column a stored relation has",
            sql: "SELECT o.id FROM orders o",
            expected_sources: &["ORDERS.ID"],
            expected_resolved: true,
        },
    ];
    let schema = shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &SNOWFLAKE,
            schema: &schema,
            sql: None,
        };
        let lineage = query_lineage(
            test_case.sql,
            Dialect::Snowflake,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", test_case.description));
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
fn given_derivation_steps_when_tracing_lineage_then_each_output_takes_its_most_significant_kind() {
    let test_cases = [
        QueryLineageTestCase {
            description: "a cast of a constant is constant, of an aggregate aggregation; COUNT(*) is aggregation",
            sql: "SELECT CAST(NULL AS INT) AS a, CAST(SUM(amount) AS VARCHAR) AS b, COUNT(*) AS c FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("a", TransformKind::Constant, &[]),
                ("b", TransformKind::Aggregation, &["ORDERS.AMOUNT"]),
                ("c", TransformKind::Aggregation, &[]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "window functions are window, COUNT(*) OVER () too though it reads no column",
            sql: "SELECT COUNT(*) OVER () AS w, ROW_NUMBER() OVER (ORDER BY id) AS r FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("w", TransformKind::Window, &[]),
                ("r", TransformKind::Window, &["ORDERS.ID"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a CTE's pass-through composes with the cast outside it",
            sql: "WITH d AS (SELECT amount FROM orders) SELECT CAST(amount AS VARCHAR) AS v FROM d",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("v", TransformKind::Cast, &["ORDERS.AMOUNT"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union passing one column of the same type through each branch is direct",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT customer_id FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "v",
                TransformKind::Direct,
                &["ORDERS.CUSTOMER_ID", "ORDERS.ID"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union whose branch is implicitly coerced is a cast",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT amount FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("v", TransformKind::Cast, &["ORDERS.AMOUNT", "ORDERS.ID"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union with an expression branch is an expression though another branch is a cast",
            sql: "SELECT id + 1 AS v FROM orders UNION ALL SELECT CAST(amount AS INT) FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "v",
                TransformKind::Expression,
                &["ORDERS.AMOUNT", "ORDERS.ID"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a cast of a constant is constant, of an aggregate aggregation; COUNT(*) is aggregation",
            sql: "SELECT CAST(NULL AS INTEGER) AS a, CAST(SUM(price) AS VARCHAR) AS b, COUNT(*) AS c FROM orders",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("a", TransformKind::Constant, &[]),
                ("b", TransformKind::Aggregation, &["orders.price"]),
                ("c", TransformKind::Aggregation, &[]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "window functions are window, COUNT(*) OVER () too though it reads no column",
            sql: "SELECT COUNT(*) OVER () AS w, ROW_NUMBER() OVER (ORDER BY id) AS r FROM orders",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("w", TransformKind::Window, &[]),
                ("r", TransformKind::Window, &["orders.id"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a CTE's pass-through composes with the cast outside it",
            sql: "WITH d AS (SELECT price FROM orders) SELECT CAST(price AS VARCHAR) AS v FROM d",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[("v", TransformKind::Cast, &["orders.price"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union passing one column of the same type through each branch is direct",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT quantity FROM orders",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[(
                "v",
                TransformKind::Direct,
                &["orders.id", "orders.quantity"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union whose branch is implicitly coerced is a cast",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT price FROM orders",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[("v", TransformKind::Cast, &["orders.id", "orders.price"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union with an expression branch is an expression though another branch is a cast",
            sql: "SELECT id + 1 AS v FROM orders UNION ALL SELECT CAST(price AS INTEGER) FROM orders",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[(
                "v",
                TransformKind::Expression,
                &["orders.id", "orders.price"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a cast of a constant is constant, of an aggregate aggregation; COUNT(*) is aggregation",
            sql: "SELECT CAST(NULL AS integer) AS a, CAST(SUM(price) AS text) AS b, COUNT(*) AS c FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[
                ("a", TransformKind::Constant, &[]),
                ("b", TransformKind::Aggregation, &["orders.price"]),
                ("c", TransformKind::Aggregation, &[]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "window functions are window, COUNT(*) OVER () too though it reads no column",
            sql: "SELECT COUNT(*) OVER () AS w, ROW_NUMBER() OVER (ORDER BY id) AS r FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[
                ("w", TransformKind::Window, &[]),
                ("r", TransformKind::Window, &["orders.id"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a CTE's pass-through composes with the cast outside it",
            sql: "WITH d AS (SELECT price FROM orders) SELECT CAST(price AS text) AS v FROM d",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[("v", TransformKind::Cast, &["orders.price"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union passing one column of the same type through each branch is direct",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT quantity FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[(
                "v",
                TransformKind::Direct,
                &["orders.id", "orders.quantity"],
            )],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union whose branch is implicitly coerced is a cast",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT price FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[("v", TransformKind::Cast, &["orders.id", "orders.price"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a union with an expression branch is an expression though another branch is a cast",
            sql: "SELECT id + 1 AS v FROM orders UNION ALL SELECT CAST(price AS integer) FROM orders",
            dialect: Dialect::PostgreSQL,
            profile: &POSTGRES,
            expected_outputs: &[(
                "v",
                TransformKind::Expression,
                &["orders.id", "orders.price"],
            )],
            expected_open_star: false,
        },
    ];
    let schemas: [(Dialect, TypedShopSchema); 3] = [
        (Dialect::Snowflake, typed_shop_schema()),
        (Dialect::DuckDB, typed_duckdb_shop_schema()),
        (Dialect::PostgreSQL, typed_postgres_shop_schema()),
    ];
    for test_case in test_cases {
        let schema: &TypedShopSchema = schemas
            .iter()
            .find(|(dialect, _)| *dialect == test_case.dialect)
            .map(|(_, schema)| schema)
            .unwrap_or_else(|| panic!("no typed schema for {:?}", test_case.dialect));
        let traced = lineage_outputs(
            test_case.sql,
            (test_case.dialect, test_case.profile),
            schema,
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
            "{:?}: {}",
            test_case.dialect,
            test_case.description
        );
    }
}
