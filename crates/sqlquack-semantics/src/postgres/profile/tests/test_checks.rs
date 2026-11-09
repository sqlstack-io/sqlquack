use sqlquack_syntax::Dialect;

use crate::dialects::constants::POSTGRES;
use crate::lineage::tests::helpers::{found_findings, owned_findings, typed_duckdb_shop_schema};
use crate::lineage::tests::test_types::QueryFindingsTestCase;
use crate::postgres::profile::tests::helpers::typed_postgres_shop_schema;

#[test]
fn given_a_postgres_query_when_checking_scope_rules_then_only_postgres_errors_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "DISTINCT ON may order by columns it does not select",
            sql: "SELECT DISTINCT ON (status) status, quantity FROM orders \
                  ORDER BY status, ordered_at DESC",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CTE column list may name fewer columns than the CTE outputs",
            sql: "WITH c(a) AS (SELECT id, quantity FROM orders) SELECT a FROM c",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CTE column list naming more columns than the CTE outputs is an error",
            sql: "WITH c(a, b) AS (SELECT id FROM orders) SELECT a FROM c",
            expected_findings: &[(
                "E223",
                "CTE column alias count does not match its output column count",
                24,
            )],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::PostgreSQL, &POSTGRES, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_postgres_query_when_checking_types_and_names_then_findings_follow_postgresql_18_6() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an integer WHERE condition (18.6: 42804)",
            sql: "SELECT id FROM orders WHERE quantity",
            expected_findings: &[(
                "E211",
                "WHERE expects a boolean predicate, found integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "a string literal WHERE condition converts to BOOLEAN (18.6: accepted)",
            sql: "SELECT id FROM orders WHERE 'true'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an integer AND operand (18.6: 42804)",
            sql: "SELECT id FROM orders, shipments WHERE quantity AND rush",
            expected_findings: &[(
                "E211",
                "Logical left operand expects boolean, found integer",
                39,
            )],
        },
        QueryFindingsTestCase {
            description: "text compared with an integer column is an error, not a runtime cast (18.6: 42883)",
            sql: "SELECT id FROM orders WHERE status = quantity",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between string and integer",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "an integer compared with a boolean (18.6: 42883)",
            sql: "SELECT id FROM orders, shipments WHERE quantity = rush",
            expected_findings: &[(
                "E217",
                "Incompatible comparison between integer and boolean",
                39,
            )],
        },
        QueryFindingsTestCase {
            description: "UNION of text and integer columns (18.6: 42804)",
            sql: "SELECT status FROM orders UNION SELECT quantity FROM orders",
            expected_findings: &[(
                "E215",
                "UNION column 1 has incompatible types: string vs integer",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "UNION of boolean and integer (18.6: 42804)",
            sql: "SELECT rush FROM shipments UNION SELECT 1",
            expected_findings: &[(
                "E215",
                "UNION column 1 has incompatible types: boolean vs integer",
                0,
            )],
        },
        QueryFindingsTestCase {
            description: "a string literal UNION an integer converts (18.6: accepted)",
            sql: "SELECT '5' UNION SELECT quantity FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "integer plus text (18.6: 42883)",
            sql: "SELECT quantity + status FROM orders",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found integer and string",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "interval times and over a number, date plus time, date minus timestamp \
                          and time minus time (18.6: accepted)",
            sql: "SELECT took * 2, took / 2, shipped_on + cutoff, shipped_on - shipped_at, \
                  cutoff - cutoff FROM shipments",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a number over an interval (18.6: 42883)",
            sql: "SELECT 2 / took FROM shipments",
            expected_findings: &[(
                "E212",
                "Arithmetic operation expects numeric-compatible operands, found integer and interval",
                11,
            )],
        },
        QueryFindingsTestCase {
            description: "NULLIF of text and an integer is an error (18.6: 42883)",
            sql: "SELECT nullif(status, quantity) FROM orders",
            expected_findings: &[("E213", "Cannot unify string and integer", 14)],
        },
        QueryFindingsTestCase {
            description: "UPPER of an integer, which PostgreSQL does not cast (18.6: 42883)",
            sql: "SELECT upper(quantity) FROM orders",
            expected_findings: &[(
                "E213",
                "Function 'upper' argument 1 expects String, found integer",
                13,
            )],
        },
        QueryFindingsTestCase {
            description: "a timestamp cast to an integer (18.6: 42846)",
            sql: "SELECT ordered_at::int FROM orders",
            expected_findings: &[("E218", "Cannot cast timestamp to integer", 7)],
        },
        QueryFindingsTestCase {
            description: "two CTEs of one name (18.6: 42712)",
            sql: "WITH c AS (SELECT 1), c AS (SELECT 2) SELECT * FROM c",
            expected_findings: &[("E233", "Duplicate CTE name 'c'", 0)],
        },
        QueryFindingsTestCase {
            description: "an output name inside an ORDER BY expression (18.6: 42703)",
            sql: "SELECT quantity AS q FROM orders ORDER BY q + 1",
            expected_findings: &[("E201", "Unknown column 'q' in table 'orders'", 42)],
        },
        QueryFindingsTestCase {
            description: "a parenthesized output name as the ORDER BY key (18.6: accepted)",
            sql: "SELECT quantity AS q FROM orders ORDER BY (q)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an output name in WHERE (18.6: 42703)",
            sql: "SELECT quantity AS q FROM orders WHERE q > 1",
            expected_findings: &[("E201", "Unknown column 'q' in table 'orders'", 39)],
        },
        QueryFindingsTestCase {
            description: "an output name in ROLLUP (18.6: accepted)",
            sql: "SELECT quantity AS q FROM orders GROUP BY ROLLUP (q)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an output name in HAVING is neither known nor grouped (18.6: 42703)",
            sql: "SELECT quantity AS q FROM orders GROUP BY quantity HAVING q > 1",
            expected_findings: &[
                (
                    "E230",
                    "Column must be grouped or used in an aggregate function",
                    58,
                ),
                ("E201", "Unknown column 'q' in table 'orders'", 58),
            ],
        },
        QueryFindingsTestCase {
            description: "system columns of a stored table (18.6: accepted)",
            sql: "SELECT ctid, xmin FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a system column of two stored tables (18.6: 42702)",
            sql: "SELECT ctid FROM orders, customers",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'ctid' found in 2 referenced tables",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a derived table has no system columns (18.6: 42703)",
            sql: "SELECT ctid FROM (SELECT id FROM orders) s",
            expected_findings: &[("E201", "Unknown column 'ctid' in table 's'", 7)],
        },
        QueryFindingsTestCase {
            description: "a quoted upper-case system column name (18.6: 42703)",
            sql: "SELECT \"CTID\" FROM orders",
            expected_findings: &[("E201", "Unknown column 'ctid' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "a ranking function without OVER (18.6: 42809)",
            sql: "SELECT row_number() AS position FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "a distribution function without OVER (18.6: 42809)",
            sql: "SELECT quantity, cume_dist() FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 0)],
        },
        QueryFindingsTestCase {
            description: "nth_value without OVER inside an expression (18.6: 42809)",
            sql: "SELECT nth_value(quantity, 2) + 1 FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 17)],
        },
        QueryFindingsTestCase {
            description: "a word compared with a decimal column (18.6: 22P02)",
            sql: "SELECT id FROM orders WHERE price > 'placed'",
            expected_findings: &[(
                "E218",
                "Comparison contains an invalid coercible literal",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "a decimal compared with an integer column (18.6: 22P02)",
            sql: "SELECT id FROM orders WHERE quantity = '1.5'",
            expected_findings: &[(
                "E218",
                "Comparison contains an invalid coercible literal",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "integers PostgreSQL reads in hex, octal, binary and with underscores (18.6: accepted)",
            sql: "SELECT id FROM orders WHERE quantity IN (' 12 ', '0x1F', '0o17', '0b101', '1_000', '+5')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an IN list holding a word beside an integer (18.6: 22P02)",
            sql: "SELECT id FROM orders WHERE quantity IN ('1', 'many')",
            expected_findings: &[("E218", "IN list contains an invalid coercible literal", 28)],
        },
        QueryFindingsTestCase {
            description: "numbers some numeric type reads: NaN, inf, an exponent, underscores, hex (18.6: accepted)",
            sql: "SELECT id FROM orders WHERE price IN ('NaN', '-inf', '1e3', '1_000.5', '.5', '0x1F')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a misplaced underscore no numeric type reads (18.6: 22P02)",
            sql: "SELECT id FROM orders WHERE price < '1._5'",
            expected_findings: &[(
                "E218",
                "Comparison contains an invalid coercible literal",
                28,
            )],
        },
        QueryFindingsTestCase {
            description: "a date that does not exist, in BETWEEN (18.6: 22008)",
            sql: "SELECT rush FROM shipments WHERE shipped_on BETWEEN '2026-01-01' AND '2026-02-30'",
            expected_findings: &[("E218", "BETWEEN contains an invalid coercible literal", 33)],
        },
        QueryFindingsTestCase {
            description: "a leap second, a spaced day word and epoch (18.6: accepted)",
            sql: "SELECT rush FROM shipments WHERE shipped_at IN ('2026-01-01 23:59:60', ' today ', 'epoch')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a word that is no timestamp (18.6: 22007)",
            sql: "SELECT rush FROM shipments WHERE shipped_at > 'placed'",
            expected_findings: &[(
                "E218",
                "Comparison contains an invalid coercible literal",
                33,
            )],
        },
        QueryFindingsTestCase {
            description: "boolean prefixes, on and off (18.6: accepted)",
            sql: "SELECT rush FROM shipments WHERE rush IN ('tr', 'ye', 'n', 'on', 'of', 'off') AND 'on'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a lone o is neither on nor off (18.6: 22P02)",
            sql: "SELECT rush FROM shipments WHERE rush = 'o'",
            expected_findings: &[(
                "E218",
                "Comparison contains an invalid coercible literal",
                33,
            )],
        },
        QueryFindingsTestCase {
            description: "window functions with OVER, plain aggregates and a windowed subquery (18.6: accepted)",
            sql: "SELECT row_number() OVER (), cume_dist() OVER (ORDER BY quantity), max(quantity) OVER (), \
                  (SELECT nth_value(id, 2) OVER (ORDER BY id) FROM orders LIMIT 1) FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = typed_postgres_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::PostgreSQL, &POSTGRES, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}
