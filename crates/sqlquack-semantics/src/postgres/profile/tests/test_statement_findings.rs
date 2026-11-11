use sqlquack_syntax::Dialect;

use crate::dialects::constants::POSTGRES;
use crate::lineage::tests::helpers::{duckdb_shop_schema, found_findings, owned_findings};
use crate::lineage::tests::test_types::QueryFindingsTestCase;

#[test]
fn given_a_postgres_relation_with_a_finding_when_resolving_names_then_only_its_follow_ons_are_dropped()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a name read through a star of a faulted CTE is a follow-on of the root finding",
            sql: "WITH kept AS (SELECT customer_id_missing FROM orders), again AS (SELECT * FROM kept) \
                  SELECT customer_id FROM again",
            expected_findings: &[(
                "E201",
                "Unknown column 'customer_id_missing' in table 'orders'",
                21,
            )],
        },
        QueryFindingsTestCase {
            description: "the root finding and a finding independent of the faulted CTE both appear",
            sql: "WITH kept AS (SELECT customer_id_missing FROM orders), again AS (SELECT * FROM kept) \
                  SELECT a.customer_id, c.nam FROM again a, customers c",
            expected_findings: &[
                ("E201", "Unknown column 'nam' in table 'customers'", 109),
                (
                    "E201",
                    "Unknown column 'customer_id_missing' in table 'orders'",
                    21,
                ),
            ],
        },
        QueryFindingsTestCase {
            description: "a name read from a derived table with a finding is a follow-on of the root finding",
            sql: "SELECT d.customer_id FROM (SELECT customer_id_missing FROM orders) d",
            expected_findings: &[(
                "E201",
                "Unknown column 'customer_id_missing' in table 'orders'",
                34,
            )],
        },
        QueryFindingsTestCase {
            description: "a derived table reading a faulted CTE is faulted; an independent error still appears",
            sql: "WITH kept AS (SELECT customer_id_missing FROM orders) \
                  SELECT d.customer_id, c.nam FROM (SELECT * FROM kept) d, customers c",
            expected_findings: &[
                ("E201", "Unknown column 'nam' in table 'customers'", 78),
                (
                    "E201",
                    "Unknown column 'customer_id_missing' in table 'orders'",
                    21,
                ),
            ],
        },
        QueryFindingsTestCase {
            description: "a derived table without a finding still has its unknown columns found",
            sql: "SELECT d.nope FROM (SELECT customer_id FROM orders) d",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'd'", 9)],
        },
        QueryFindingsTestCase {
            description: "a CTE without a finding still has its unknown columns found",
            sql: "WITH kept AS (SELECT customer_id FROM orders) SELECT k.nope FROM kept k",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'kept'", 55)],
        },
    ];
    let schema = duckdb_shop_schema();
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
fn given_a_derived_table_inside_a_lateral_subquery_when_resolving_names_then_it_reads_the_enclosing_items()
 {
    // Each verdict is PostgreSQL 18.6's: a nested derived table may read the item before the
    // LATERAL subquery, but a derived table that is not LATERAL may not read its siblings.
    let test_cases = [
        QueryFindingsTestCase {
            description: "a comma-joined LATERAL whose nested derived table reads the item before it",
            sql: "SELECT k.n FROM orders AS p, \
                  LATERAL (SELECT f.u AS n FROM (SELECT p.items AS u) AS f) AS k",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "the same body under CROSS JOIN LATERAL",
            sql: "SELECT k.n FROM orders AS p \
                  CROSS JOIN LATERAL (SELECT f.u AS n FROM (SELECT p.items AS u) AS f) AS k",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a derived table that is not LATERAL still cannot read its sibling",
            sql: "SELECT k.n FROM orders AS p, (SELECT p.items AS n) AS k",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'p' referenced by column 'items'",
                37,
            )],
        },
    ];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::PostgreSQL, &POSTGRES, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}
