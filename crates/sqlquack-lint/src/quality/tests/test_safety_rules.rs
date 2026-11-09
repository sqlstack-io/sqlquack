use serde_json::Value;

use crate::quality::tests::helpers::{anchors, lint_dialect, lint_unavailable};
use crate::quality::tests::test_types::SafetyRuleTestCase;

#[test]
fn given_safety_rule_inputs_when_linting_then_exactly_the_mistakes_are_reported()
-> Result<(), String> {
    let test_cases = [
        SafetyRuleTestCase {
            description: "unquoted CTE names collide regardless of case",
            dialect: "duckdb",
            sql: "WITH paid AS (SELECT 1 AS x), PAID AS (SELECT 2 AS x) SELECT x FROM paid",
            rule: "SQLCLEAN046",
            expected_anchors: &["PAID"],
        },
        SafetyRuleTestCase {
            description: "a quoted name equals the folded unquoted name in Snowflake",
            dialect: "snowflake",
            sql: r#"WITH "PAID" AS (SELECT 1 AS x), paid AS (SELECT 2 AS x) SELECT x FROM paid"#,
            rule: "SQLCLEAN046",
            expected_anchors: &["paid"],
        },
        SafetyRuleTestCase {
            description: "a nested WITH may reuse an outer CTE name",
            dialect: "duckdb",
            sql: "WITH paid AS (WITH paid AS (SELECT 1 AS x) SELECT x FROM paid) SELECT x FROM paid",
            rule: "SQLCLEAN046",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "simple CASE compares with NULL, and a singleton IN list holds only NULL",
            dialect: "duckdb",
            sql: "SELECT CASE status WHEN 'open' THEN 1 WHEN NULL THEN 2 END AS s FROM orders WHERE region IN (NULL)",
            rule: "SQLCLEAN047",
            expected_anchors: &["WHEN NULL", "IN (NULL)"],
        },
        SafetyRuleTestCase {
            description: "searched CASE, IS NULL, NOT IN and a longer IN list pass",
            dialect: "duckdb",
            sql: "SELECT CASE WHEN status IS NULL THEN 1 END AS s FROM orders WHERE region NOT IN (NULL) AND kind IN (1, NULL)",
            rule: "SQLCLEAN047",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "only the CASE whose operand compares with NULL is reported",
            dialect: "duckdb",
            sql: "SELECT CASE status WHEN 'open' THEN CASE WHEN NULL THEN 1 END END AS a, CASE WHEN amount > 0 THEN CASE region WHEN NULL THEN 2 END END AS b FROM orders",
            rule: "SQLCLEAN047",
            expected_anchors: &["WHEN NULL"],
        },
        SafetyRuleTestCase {
            description: "division and modulo by literal zero in operator and function form",
            dialect: "duckdb",
            sql: "SELECT amount / 0 AS a, amount % 0.0 AS b, MOD(amount, 0) AS c, amount / (-0) AS d FROM orders",
            rule: "SQLCLEAN048",
            expected_anchors: &["/ 0", "% 0.0", "MOD(amount, 0)", "/ (-0)"],
        },
        SafetyRuleTestCase {
            description: "a non-zero or computed divisor passes",
            dialect: "duckdb",
            sql: "SELECT amount / 0.5 AS a, amount / 1e-400 AS e, amount / 0 ** 2 AS b, MOD(amount, 0 + 1) AS c, amount / NULLIF(quantity, 0) AS d FROM orders",
            rule: "SQLCLEAN048",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "a missing or constant WHERE does not limit an UPDATE or DELETE",
            dialect: "duckdb",
            sql: "UPDATE orders SET status = 'closed'; DELETE FROM orders WHERE TRUE; DELETE FROM orders WHERE 1 = 1",
            rule: "SQLCLEAN049",
            expected_anchors: &["UPDATE", "DELETE", "DELETE"],
        },
        SafetyRuleTestCase {
            description: "a row predicate or a subquery limits the write",
            dialect: "duckdb",
            sql: "UPDATE orders SET status = 'closed' WHERE closed_at IS NOT NULL; DELETE FROM orders WHERE EXISTS (SELECT 1 FROM refunds)",
            rule: "SQLCLEAN049",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "an INSERT without a column list in query and VALUES form",
            dialect: "duckdb",
            sql: "INSERT INTO orders SELECT * FROM staged_orders; INSERT INTO orders VALUES (1, 'open')",
            rule: "SQLCLEAN050",
            expected_anchors: &["INSERT", "INSERT"],
        },
        SafetyRuleTestCase {
            description: "listed columns and BY NAME pass",
            dialect: "duckdb",
            sql: "INSERT INTO orders (order_id) SELECT order_id FROM staged_orders; INSERT INTO orders BY NAME SELECT 1 AS order_id",
            rule: "SQLCLEAN050",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "NATURAL joins are reported with their join keywords",
            dialect: "duckdb",
            sql: "SELECT * FROM orders NATURAL JOIN customers NATURAL LEFT OUTER JOIN regions JOIN items USING (order_id)",
            rule: "SQLCLEAN051",
            expected_anchors: &["NATURAL JOIN", "NATURAL LEFT OUTER JOIN"],
        },
        SafetyRuleTestCase {
            description: "reversed number, negative, decimal and date bounds, with or without NOT",
            dialect: "duckdb",
            sql: "SELECT * FROM orders WHERE a BETWEEN 10 AND 1 OR b NOT BETWEEN -1 AND -5 OR c BETWEEN 1.5 AND 1.25 OR d BETWEEN DATE '2026-02-01' AND DATE '2026-01-31'",
            rule: "SQLCLEAN052",
            expected_anchors: &[
                "BETWEEN 10 AND 1",
                "BETWEEN -1 AND -5",
                "BETWEEN 1.5 AND 1.25",
                "BETWEEN DATE '2026-02-01' AND DATE '2026-01-31'",
            ],
        },
        SafetyRuleTestCase {
            description: "ordered, equal, SYMMETRIC, computed, text and frame bounds pass",
            dialect: "duckdb",
            sql: "SELECT SUM(a) OVER (ORDER BY b ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS s FROM orders WHERE a BETWEEN 9 AND 10 OR b BETWEEN -5 AND -1 OR c BETWEEN 5 AND 5 OR d BETWEEN SYMMETRIC 10 AND 1 OR e BETWEEN 10 AND 1 + 20 OR f BETWEEN 'b' AND 'a'",
            rule: "SQLCLEAN052",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "aggregates and windows in WHERE, ON and after a GROUP BY comma",
            dialect: "duckdb",
            sql: "SELECT o.customer_id FROM orders AS o JOIN customers AS c ON c.id = o.customer_id AND COUNT(*) > 1 WHERE SUM(o.amount) > 100 AND ROW_NUMBER() OVER (PARTITION BY o.customer_id ORDER BY o.ordered_at) = 1 GROUP BY o.customer_id, MAX(o.amount)",
            rule: "SQLCLEAN053",
            expected_anchors: &[
                "COUNT(*)",
                "SUM(o.amount)",
                "ROW_NUMBER() OVER (PARTITION BY o.customer_id ORDER BY o.ordered_at)",
                "MAX(o.amount)",
            ],
        },
        SafetyRuleTestCase {
            description: "HAVING, QUALIFY, FILTER, subqueries and unknown functions pass",
            dialect: "duckdb",
            sql: "SELECT customer_id, COUNT(*) FILTER (WHERE amount > 0) AS paid FROM orders WHERE amount > (SELECT AVG(amount) FROM orders) AND order_total(amount) > 0 GROUP BY customer_id HAVING COUNT(*) > 1 QUALIFY ROW_NUMBER() OVER (ORDER BY customer_id) = 1",
            rule: "SQLCLEAN053",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "DuckDB PIVOT USING aggregates end the pivot ON clause",
            dialect: "duckdb",
            sql: "PIVOT orders ON status IN ('placed') USING MAX(amount), SUM(amount) GROUP BY customer_id",
            rule: "SQLCLEAN053",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "a join ON aggregate is still reported beside a pivot",
            dialect: "duckdb",
            sql: "SELECT o.id FROM orders AS o JOIN (PIVOT items ON kind USING MAX(qty)) AS p ON p.id = o.id AND MIN(o.amount) > 0",
            rule: "SQLCLEAN053",
            expected_anchors: &["MIN(o.amount)"],
        },
        SafetyRuleTestCase {
            description: "quoted names that differ in case are distinct in PostgreSQL",
            dialect: "postgres",
            sql: r#"WITH "paid" AS (SELECT 1 AS x), "PAID" AS (SELECT 2 AS x) SELECT x FROM "paid""#,
            rule: "SQLCLEAN046",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "UPDATE FROM without WHERE, and DELETE after a WITH clause",
            dialect: "postgres",
            sql: "UPDATE orders SET status = 'closed' FROM customers; WITH stale AS (SELECT 1 AS id) DELETE FROM orders",
            rule: "SQLCLEAN049",
            expected_anchors: &["UPDATE", "DELETE"],
        },
        SafetyRuleTestCase {
            description: "DEFAULT VALUES needs no column list",
            dialect: "postgres",
            sql: "INSERT INTO orders DEFAULT VALUES",
            rule: "SQLCLEAN050",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "ASYMMETRIC keeps the ordered meaning in PostgreSQL",
            dialect: "postgres",
            sql: "SELECT * FROM orders WHERE a BETWEEN ASYMMETRIC 10 AND 1",
            rule: "SQLCLEAN052",
            expected_anchors: &["BETWEEN ASYMMETRIC 10 AND 1"],
        },
        SafetyRuleTestCase {
            description: "another dialect's aggregate name is unknown in PostgreSQL",
            dialect: "postgres",
            sql: "SELECT t.a FROM t WHERE MEDIAN(t.x) > 1",
            rule: "SQLCLEAN053",
            expected_anchors: &[],
        },
        SafetyRuleTestCase {
            description: "another dialect's aggregate name still counts for no particular engine",
            dialect: "generic",
            sql: "SELECT t.a FROM t WHERE MEDIAN(t.x) > 1",
            rule: "SQLCLEAN053",
            expected_anchors: &["MEDIAN(t.x)"],
        },
        SafetyRuleTestCase {
            description: "a PostgreSQL catalogue aggregate is reported for PostgreSQL and no particular engine",
            dialect: "generic",
            sql: "SELECT t.a FROM t WHERE BOOL_AND(t.x) OR EVERY(t.y)",
            rule: "SQLCLEAN053",
            expected_anchors: &["BOOL_AND(t.x)", "EVERY(t.y)"],
        },
        SafetyRuleTestCase {
            description: "BOOL_AND and EVERY are PostgreSQL aggregates",
            dialect: "postgres",
            sql: "SELECT t.a FROM t WHERE BOOL_AND(t.x) OR EVERY(t.y)",
            rule: "SQLCLEAN053",
            expected_anchors: &["BOOL_AND(t.x)", "EVERY(t.y)"],
        },
        SafetyRuleTestCase {
            description: "the SQL/JSON aggregates are PostgreSQL aggregates",
            dialect: "postgres",
            sql: "SELECT t.a FROM t WHERE JSON_ARRAYAGG(t.x) IS NULL OR JSON_OBJECTAGG(t.k VALUE t.v) IS NULL",
            rule: "SQLCLEAN053",
            expected_anchors: &["JSON_ARRAYAGG(t.x)", "JSON_OBJECTAGG(t.k VALUE t.v)"],
        },
        SafetyRuleTestCase {
            description: "multi-argument MAX is the scalar function in SQLite",
            dialect: "sqlite",
            sql: "SELECT order_id FROM orders WHERE max(amount, refunded) > 100",
            rule: "SQLCLEAN053",
            expected_anchors: &[],
        },
    ];
    let unparsed_dialect_cases = [SafetyRuleTestCase {
        description: "a joined DELETE and a LIMIT bound the rows in MySQL",
        dialect: "mysql",
        sql: "DELETE o FROM orders o JOIN customers c ON c.id = o.customer_id; DELETE FROM orders LIMIT 10",
        rule: "SQLCLEAN049",
        expected_anchors: &[],
    }];
    for test_case in &unparsed_dialect_cases {
        assert!(
            lint_unavailable(test_case.dialect)?,
            "{}: lint is unavailable until SQLQuack parses {}",
            test_case.description,
            test_case.dialect
        );
    }
    for test_case in &test_cases {
        let diagnostics: Vec<Value> =
            lint_dialect(test_case.sql, test_case.dialect, test_case.rule)?;
        assert_eq!(
            anchors(test_case.sql, &diagnostics),
            test_case.expected_anchors,
            "{}",
            test_case.description
        );
    }
    Ok(())
}
