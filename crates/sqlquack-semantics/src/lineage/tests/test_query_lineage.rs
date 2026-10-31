use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::{DUCKDB, SNOWFLAKE};
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::shop_schema;
use crate::lineage::tests::test_types::QueryLineageTestCase;

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
            description: "a union output reads every branch's column at its position, an expression of several",
            sql: "SELECT id FROM orders UNION ALL SELECT sku FROM shop.products",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "id",
                TransformKind::Expression,
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
                (
                    "id",
                    TransformKind::Expression,
                    &["CUSTOMERS.ID", "ORDERS.ID"],
                ),
                (
                    "status",
                    TransformKind::Expression,
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
                    TransformKind::Expression,
                    &["ORDERS.AMOUNT", "ORDERS.CUSTOMER_ID"],
                ),
                (
                    "customer_total",
                    TransformKind::Aggregation,
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
                TransformKind::Expression,
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
            description: "a quoted stored column matches only its exact quoted spelling",
            sql: "SELECT e.\"listingId\" AS listing, e.listingid AS unmatched, e.listing_status FROM exports e",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("listing", TransformKind::Direct, &["EXPORTS.listingId"]),
                ("unmatched", TransformKind::Constant, &[]),
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
                    TransformKind::Expression,
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
