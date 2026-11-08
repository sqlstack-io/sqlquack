use crate::lineage::tests::helpers::{databricks_findings, databricks_outputs, owned_findings};
use crate::lineage::tests::test_types::{DatabricksOutputsTestCase, QueryFindingsTestCase};

#[test]
fn given_a_databricks_query_when_checking_it_then_only_rules_databricks_shares_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "DATE_TRUNC over a STRING column, which Snowflake's rules refuse",
            sql: "SELECT DATE_TRUNC('day', status) AS d FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a STRING WHERE condition and a STRING compared with a number",
            sql: "SELECT id FROM orders WHERE status AND status > 1",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "two FROM items under one alias, which Databricks accepts",
            sql: "SELECT 1 AS one FROM orders o JOIN customers o ON TRUE",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a lateral column alias read by a later output",
            sql: "SELECT quantity AS q, q + 1 AS r FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "two CTEs under one name (DUPLICATED_CTE_NAMES)",
            sql: "WITH c AS (SELECT 1 AS x), c AS (SELECT 2 AS x) SELECT x FROM c",
            expected_findings: &[("E233", "Duplicate CTE name 'c'", 55)],
        },
        QueryFindingsTestCase {
            description: "QUALIFY without a window function (QUALIFY must be used with one)",
            sql: "SELECT id FROM orders QUALIFY id > 0",
            expected_findings: &[(
                "E232",
                "QUALIFY requires a window function in the same query scope",
                7,
            )],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            databricks_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_databricks_query_when_naming_and_typing_outputs_then_they_are_the_warehouses_or_none() {
    let test_cases = [
        DatabricksOutputsTestCase {
            description: "literals typed as typeof reports them",
            sql: "SELECT 1 AS a, 2147483648 AS b, 9223372036854775808 AS c, 1.50 AS d, \
                  0.05 AS e, 1e5 AS f, 'x' AS g, -2147483648 AS h",
            expected_outputs: &[
                ("a", Some("INT")),
                ("b", Some("BIGINT")),
                ("c", Some("DECIMAL(19,0)")),
                ("d", Some("DECIMAL(3,2)")),
                ("e", Some("DECIMAL(2,2)")),
                ("f", Some("DOUBLE")),
                ("g", Some("STRING")),
                ("h", Some("INT")),
            ],
        },
        DatabricksOutputsTestCase {
            description: "casts typed as DESCRIBE QUERY reports them",
            sql: "SELECT CAST(1 AS TIMESTAMP) AS i, CAST('2020-01-01' AS TIMESTAMP_NTZ) AS j, \
                  CAST(1 AS DECIMAL) AS k, CAST('a' AS VARCHAR(3)) AS l",
            expected_outputs: &[
                ("i", Some("TIMESTAMP")),
                ("j", Some("TIMESTAMP_NTZ")),
                ("k", Some("DECIMAL(10,0)")),
                ("l", Some("STRING")),
            ],
        },
        DatabricksOutputsTestCase {
            description: "operators and lateral aliases, untyped where Databricks is not recorded",
            sql: "SELECT status || status AS m, quantity + quantity AS n, \
                  quantity / quantity AS o, ROUND(price, 1) AS p, quantity AS q, q * 2 AS r \
                  FROM orders",
            expected_outputs: &[
                ("m", Some("STRING")),
                ("n", Some("INT")),
                ("o", None),
                ("p", None),
                ("q", Some("INT")),
                ("r", Some("INT")),
            ],
        },
        DatabricksOutputsTestCase {
            description: "VALUES columns named col1, col2",
            sql: "SELECT * FROM VALUES (1, 'a')",
            expected_outputs: &[("col1", Some("INT")), ("col2", Some("STRING"))],
        },
        DatabricksOutputsTestCase {
            description: "an unaliased expression left unnamed, not given Snowflake's name",
            sql: "SELECT status, UPPER(status) FROM orders",
            expected_outputs: &[("status", Some("STRING")), ("", None)],
        },
    ];
    for test_case in test_cases {
        let found: Vec<(String, Option<String>)> = databricks_outputs(test_case.sql);
        let found: Vec<(&str, Option<&str>)> = found
            .iter()
            .map(|(name, data_type)| (name.as_str(), data_type.as_deref()))
            .collect();
        assert_eq!(
            found, test_case.expected_outputs,
            "{}",
            test_case.description
        );
    }
}
