use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::{DUCKDB, SNOWFLAKE};
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::{shop_schema, written};
use crate::lineage::tests::test_types::LineageDetailTestCase;

const ALL_ORDER_COLUMNS: [&str; 5] = [
    "ORDERS.AMOUNT",
    "ORDERS.CUSTOMER_ID",
    "ORDERS.ID",
    "ORDERS.ITEMS",
    "ORDERS.STATUS",
];

#[test]
fn given_a_query_when_tracing_lineage_then_values_rows_and_untraced_outputs_are_reported() {
    let test_cases = [
        LineageDetailTestCase {
            description: "join, filter and qualify columns decide rows, not values",
            sql: "SELECT o.id FROM orders o JOIN customers c ON o.customer_id = c.id \
                  WHERE c.country = 'NZ' \
                  QUALIFY ROW_NUMBER() OVER (PARTITION BY o.status ORDER BY o.amount) = 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &[
                "CUSTOMERS.COUNTRY",
                "CUSTOMERS.ID",
                "ORDERS.AMOUNT",
                "ORDERS.CUSTOMER_ID",
                "ORDERS.STATUS",
            ],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "group by and having columns decide rows",
            sql: "SELECT customer_id, COUNT(*) AS n FROM orders \
                  GROUP BY customer_id HAVING SUM(amount) > 10",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["ORDERS.CUSTOMER_ID"],
                    true,
                ),
                ("n", TransformKind::Aggregation, &[], true),
            ],
            expected_row_sources: &["ORDERS.AMOUNT", "ORDERS.CUSTOMER_ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a correlated EXISTS decides rows by both levels' columns",
            sql: "SELECT c.name FROM customers c \
                  WHERE EXISTS (SELECT 1 FROM orders o WHERE o.customer_id = c.id)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("name", TransformKind::Direct, &["CUSTOMERS.NAME"], true)],
            expected_row_sources: &["CUSTOMERS.ID", "ORDERS.CUSTOMER_ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "an EXISTS value is untraced, never a constant",
            sql: "SELECT EXISTS (SELECT 1 FROM orders o WHERE o.customer_id = c.id) AS has_orders \
                  FROM customers c",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("has_orders", TransformKind::Constant, &[], false)],
            expected_row_sources: &["CUSTOMERS.ID", "ORDERS.CUSTOMER_ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "CONNECT BY decides rows and CONNECT_BY_ROOT reads its column",
            sql: "SELECT CONNECT_BY_ROOT amount AS root_amount FROM orders \
                  START WITH status = 'open' CONNECT BY PRIOR id = customer_id",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "root_amount",
                TransformKind::Direct,
                &["ORDERS.AMOUNT"],
                true,
            )],
            expected_row_sources: &["ORDERS.CUSTOMER_ID", "ORDERS.ID", "ORDERS.STATUS"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "CONNECT_BY_ROOT of a qualified column reads that column",
            sql: "SELECT CONNECT_BY_ROOT o.amount AS root_amount FROM orders o \
                  START WITH o.status = 'open' CONNECT BY PRIOR o.id = o.customer_id",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "root_amount",
                TransformKind::Direct,
                &["ORDERS.AMOUNT"],
                true,
            )],
            expected_row_sources: &["ORDERS.CUSTOMER_ID", "ORDERS.ID", "ORDERS.STATUS"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a star EXCLUDE of a quoted lower-case name drops that column",
            sql: "WITH n AS (SELECT id, amount AS \"note\" FROM orders) \
                  SELECT * EXCLUDE (\"note\") FROM n",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "$n reads the n-th column of the one FROM item",
            sql: "SELECT $2 AS cust FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("cust", TransformKind::Direct, &["ORDERS.CUSTOMER_ID"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "$n reads a CTE's column, and a VALUES column's constant",
            sql: "WITH c AS (SELECT amount FROM orders) \
                  SELECT c.$1 AS first, v.$1 AS second FROM c, (SELECT $1 FROM VALUES (1)) v",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("first", TransformKind::Direct, &["ORDERS.AMOUNT"], true),
                ("second", TransformKind::Constant, &[], true),
            ],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "$n past the known columns is untraced",
            sql: "SELECT $9 AS missing FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("missing", TransformKind::Constant, &[], false)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a user table function's columns also read its arguments",
            sql: "SELECT r.ticket_id FROM orders o, TABLE(tickets_for(o.amount)) AS r",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "ticket_id",
                TransformKind::Expression,
                &["ORDERS.AMOUNT", "TICKETS_FOR.ticket_id"],
                true,
            )],
            expected_row_sources: &["ORDERS.AMOUNT"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "IDENTIFIER names a relation at run time, so its columns are untraced",
            sql: "SELECT id FROM IDENTIFIER('orders')",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Constant, &[], false)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a RIGHT join's USING key comes from the right side",
            sql: "SELECT id FROM orders RIGHT JOIN customers USING (id)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["CUSTOMERS.ID"], true)],
            expected_row_sources: &["CUSTOMERS.ID", "ORDERS.ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a NATURAL FULL join's key is an expression over both sides",
            sql: "SELECT id FROM orders NATURAL FULL JOIN customers",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "id",
                TransformKind::Expression,
                &["CUSTOMERS.ID", "ORDERS.ID"],
                true,
            )],
            expected_row_sources: &["CUSTOMERS.ID", "ORDERS.ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a star over a USING join selects the merged key once, first",
            sql: "SELECT * FROM customers JOIN orders USING (id)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("ID", TransformKind::Direct, &["CUSTOMERS.ID"], true),
                ("NAME", TransformKind::Direct, &["CUSTOMERS.NAME"], true),
                (
                    "COUNTRY",
                    TransformKind::Direct,
                    &["CUSTOMERS.COUNTRY"],
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
        },
        LineageDetailTestCase {
            description: "a CTE reading itself without RECURSIVE is recursive, not a stored table",
            sql: "WITH chain AS (SELECT id, customer_id AS parent FROM orders \
                  UNION ALL SELECT o.id, c.parent FROM chain c JOIN orders o ON o.customer_id = c.id) \
                  SELECT parent FROM chain",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "parent",
                TransformKind::Direct,
                &["ORDERS.CUSTOMER_ID"],
                true,
            )],
            expected_row_sources: &["ORDERS.CUSTOMER_ID", "ORDERS.ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a recursive CTE gathers columns that arrive in later iterations",
            sql: "WITH RECURSIVE r (a, b, c) AS (SELECT id, 0, 0 FROM orders \
                  UNION ALL SELECT c, a, b FROM r) SELECT a, b, c FROM r",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("a", TransformKind::Expression, &["ORDERS.ID"], true),
                ("b", TransformKind::Expression, &["ORDERS.ID"], true),
                ("c", TransformKind::Expression, &["ORDERS.ID"], true),
            ],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a union with a constant branch is an expression",
            sql: "SELECT id FROM orders UNION ALL SELECT 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Expression, &["ORDERS.ID"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a union of one same column is direct",
            sql: "SELECT id FROM orders UNION SELECT id FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "INTERSECT values read both sides and the right side decides rows",
            sql: "SELECT id FROM orders INTERSECT SELECT id FROM customers",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "id",
                TransformKind::Expression,
                &["CUSTOMERS.ID", "ORDERS.ID"],
                true,
            )],
            expected_row_sources: &["CUSTOMERS.ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "VALUES rows mixing a column and a literal are an expression",
            sql: "SELECT v.x FROM orders o, LATERAL (SELECT * FROM VALUES (o.id), (1)) AS v (x)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("x", TransformKind::Expression, &["ORDERS.ID"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a select alias beside a relation of unknown columns is ambiguous",
            sql: "SELECT amount AS total, total * 2 AS doubled FROM support.tickets",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                (
                    "total",
                    TransformKind::Direct,
                    &["SUPPORT.TICKETS.amount"],
                    true,
                ),
                ("doubled", TransformKind::Constant, &[], false),
            ],
            expected_row_sources: &[],
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
            description: "a quoted column of an unknown relation keeps its quotes",
            sql: "SELECT \"ticketId\" FROM support.tickets",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "ticketId",
                TransformKind::Direct,
                &["SUPPORT.TICKETS.\"ticketId\""],
                true,
            )],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "an unaliased cast is named as Snowflake names it; a path or spaced text is not named",
            sql: "SELECT amount::INT, items:sku, amount + 1 FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("AMOUNT::INT", TransformKind::Cast, &["ORDERS.AMOUNT"], true),
                ("", TransformKind::Expression, &["ORDERS.ITEMS"], true),
                ("", TransformKind::Expression, &["ORDERS.AMOUNT"], true),
            ],
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
        LineageDetailTestCase {
            description: "PIVOT's FOR column decides rows",
            sql: "SELECT * FROM (SELECT customer_id, status, amount FROM orders) \
                  PIVOT (SUM(amount) FOR status IN ('open', 'paid'))",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["ORDERS.CUSTOMER_ID"],
                    true,
                ),
                (
                    "'open'",
                    TransformKind::Aggregation,
                    &["ORDERS.AMOUNT"],
                    true,
                ),
                (
                    "'paid'",
                    TransformKind::Aggregation,
                    &["ORDERS.AMOUNT"],
                    true,
                ),
            ],
            expected_row_sources: &["ORDERS.STATUS"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a star read as a value over every order column",
            sql: "SELECT OBJECT_CONSTRUCT(*) AS row_object FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "row_object",
                TransformKind::Expression,
                &ALL_ORDER_COLUMNS,
                true,
            )],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a filter on a select alias beside an unknown relation leaves rows unresolved",
            sql: "SELECT amount AS b FROM support.tickets WHERE b > 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "b",
                TransformKind::Direct,
                &["SUPPORT.TICKETS.amount"],
                true,
            )],
            expected_row_sources: &[],
            expected_rows_resolved: false,
        },
        LineageDetailTestCase {
            description: "a filter on a name two unknown relations may offer leaves rows unresolved",
            sql: "SELECT t.ticket_id FROM support.tickets t, support.queues q WHERE x = 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "ticket_id",
                TransformKind::Direct,
                &["SUPPORT.TICKETS.ticket_id"],
                true,
            )],
            expected_row_sources: &[],
            expected_rows_resolved: false,
        },
        LineageDetailTestCase {
            description: "IDENTIFIER in a filtering subquery leaves rows unresolved",
            sql: "SELECT id FROM orders WHERE id IN (SELECT id FROM IDENTIFIER('customers'))",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &["ORDERS.ID"],
            expected_rows_resolved: false,
        },
        LineageDetailTestCase {
            description: "a quoted name the relation lacks in a filter is read from it, rows unresolved",
            sql: "SELECT id FROM orders WHERE \"id\" = 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &["ORDERS.\"id\""],
            expected_rows_resolved: false,
        },
        LineageDetailTestCase {
            description: "ORDER BY under LIMIT decides rows",
            sql: "SELECT id FROM orders ORDER BY amount DESC LIMIT 5",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &["ORDERS.AMOUNT"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "ORDER BY without a limit does not decide rows",
            sql: "SELECT id FROM orders ORDER BY amount",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &[],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "ORDER BY an ordinal under TOP decides rows by that output",
            sql: "SELECT TOP 3 id FROM orders ORDER BY 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &["ORDERS.ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "ORDER BY under FETCH decides rows",
            sql: "SELECT id FROM orders ORDER BY status FETCH FIRST 2 ROWS ONLY",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &["ORDERS.STATUS"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "ORDER BY under a LIMIT on a union decides rows by both branches",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM customers ORDER BY id LIMIT 3",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[(
                "id",
                TransformKind::Expression,
                &["CUSTOMERS.ID", "ORDERS.ID"],
                true,
            )],
            expected_row_sources: &["CUSTOMERS.ID", "ORDERS.ID"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a FLATTEN input decides rows though no FLATTEN column is selected",
            sql: "SELECT o.id FROM orders o, LATERAL FLATTEN(input => o.items) f",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("id", TransformKind::Direct, &["ORDERS.ID"], true)],
            expected_row_sources: &["ORDERS.ITEMS"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "a GROUP BY ordinal decides rows by that output",
            sql: "SELECT status, COUNT(*) AS n FROM orders GROUP BY 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("status", TransformKind::Direct, &["ORDERS.STATUS"], true),
                ("n", TransformKind::Aggregation, &[], true),
            ],
            expected_row_sources: &["ORDERS.STATUS"],
            expected_rows_resolved: true,
        },
        LineageDetailTestCase {
            description: "GROUP BY ALL decides rows by every output that does not aggregate",
            sql: "SELECT status, customer_id, SUM(amount) AS total FROM orders GROUP BY ALL",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("status", TransformKind::Direct, &["ORDERS.STATUS"], true),
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["ORDERS.CUSTOMER_ID"],
                    true,
                ),
                (
                    "total",
                    TransformKind::Aggregation,
                    &["ORDERS.AMOUNT"],
                    true,
                ),
            ],
            expected_row_sources: &["ORDERS.CUSTOMER_ID", "ORDERS.STATUS"],
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
