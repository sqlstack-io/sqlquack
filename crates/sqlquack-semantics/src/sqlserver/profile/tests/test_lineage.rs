use crate::dialects::constants::SQL_SERVER;
use crate::lineage::tests::helpers::duckdb_shop_schema;
use crate::lineage::tests::helpers::output_ranges;
use crate::lineage::tests::helpers::owned_findings;
use crate::lineage::tests::test_types::QueryFindingsTestCase;
use crate::lineage::tests::test_types::StrictRangeTestCase;
use crate::sqlserver::profile::tests::helpers::sqlserver_findings;
use crate::sqlserver::profile::tests::helpers::sqlserver_outputs;
use crate::sqlserver::profile::tests::helpers::sqlserver_shop_schema;
use crate::sqlserver::profile::tests::test_types::SqlServerOutputsTestCase;

#[test]
fn given_a_sql_server_query_the_engine_rejects_when_checking_then_the_broken_rule_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an unknown column (207)",
            sql: "SELECT nope FROM orders",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "an unknown qualifier (4104)",
            sql: "SELECT zq.id FROM orders o",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a column two joined tables offer, unqualified (209)",
            sql: "SELECT id FROM orders o JOIN customers c ON o.id = c.id",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'id' found in 2 referenced tables",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a column neither grouped nor aggregated (8120)",
            sql: "SELECT status, SUM(amount) AS total FROM orders GROUP BY id",
            expected_findings: &[(
                "E230",
                "Column must be grouped or used in an aggregate function",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "an output alias read in the same select list (207)",
            sql: "SELECT id AS x, x + 1 AS y FROM orders",
            expected_findings: &[("E201", "Unknown column 'x' in table 'orders'", 16)],
        },
        QueryFindingsTestCase {
            description: "an output alias read in WHERE (207)",
            sql: "SELECT id AS x FROM orders WHERE x > 1",
            expected_findings: &[("E201", "Unknown column 'x' in table 'orders'", 33)],
        },
        QueryFindingsTestCase {
            description: "an output alias grouped by (207)",
            sql: "SELECT status AS s, COUNT(*) AS n FROM orders GROUP BY s",
            expected_findings: &[("E201", "Unknown column 's' in table 'orders'", 55)],
        },
        QueryFindingsTestCase {
            description: "an output alias in an ORDER BY expression (207)",
            sql: "SELECT id AS x FROM orders ORDER BY x + 1",
            expected_findings: &[("E201", "Unknown column 'x' in table 'orders'", 36)],
        },
        QueryFindingsTestCase {
            description: "a window-only function without OVER (10753)",
            sql: "SELECT ROW_NUMBER() AS r FROM orders",
            expected_findings: &[("E232", "Window function requires an OVER clause", 7)],
        },
    ];
    let schema = sqlserver_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            sqlserver_findings(&schema, test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_sql_server_query_the_engine_accepts_when_checking_then_nothing_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a delimited column in another case resolves",
            sql: "SELECT [ID], O.[Status] FROM Orders o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a window-only function with OVER",
            sql: "SELECT ROW_NUMBER() OVER (ORDER BY id) AS r FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a grouped expression matches whatever case its columns are written in",
            sql: "SELECT CAST([CREATED_AT] AS DATE) AS d, SUM(amount) AS total FROM orders \
                GROUP BY CAST(created_at AS DATE)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an output alias is a bare ORDER BY key, parenthesized or not",
            sql: "SELECT id AS x, status AS s FROM orders ORDER BY x, (s) DESC",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a set operation converts by SQL Server's type precedence, unchecked here",
            sql: "SELECT id FROM orders UNION ALL SELECT status FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an int reads as a datetime and + joins strings, as SQL Server converts them",
            sql: "SELECT DATEADD(hour, DATEDIFF(hour, 0, created_at), 0) AS h, 'a' + status AS s \
                FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = sqlserver_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            sqlserver_findings(&schema, test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_sql_server_query_when_typing_then_outputs_take_the_engines_types() {
    let test_cases = [
        SqlServerOutputsTestCase {
            description: "a TIMESTAMP column is datetime2, as SQLBuild's adapter creates it",
            sql: "SELECT created_at, logged_at FROM orders",
            expected_outputs: &[
                ("created_at", Some("DATETIME2")),
                ("logged_at", Some("DATETIME2")),
            ],
        },
        SqlServerOutputsTestCase {
            description: "an N string is nvarchar and a plain one varchar",
            sql: "SELECT N'x' AS n, 'x' AS v",
            expected_outputs: &[("n", Some("NVARCHAR")), ("v", Some("VARCHAR"))],
        },
        SqlServerOutputsTestCase {
            description: "ROUND keeps float and decimal and widens integers to int",
            sql: "SELECT ROUND(amount, 2) AS r, ROUND(price, 1) AS p, ROUND(id, 0) AS i FROM orders",
            expected_outputs: &[
                ("r", Some("FLOAT")),
                ("p", Some("DECIMAL(12, 2)")),
                ("i", Some("INT")),
            ],
        },
        SqlServerOutputsTestCase {
            description: "UPPER, LOWER and the trims keep nvarchar, and CONCAT with nvarchar is nvarchar",
            sql: "SELECT UPPER(label) AS u, LOWER(label) AS l, RTRIM(label) AS r, UPPER(status) AS s, \
                CONCAT(status, label) AS c, REPLACE(label, 'a', 'b') AS p FROM orders",
            expected_outputs: &[
                ("u", Some("NVARCHAR(40)")),
                ("l", Some("NVARCHAR(40)")),
                ("r", Some("NVARCHAR(40)")),
                ("s", Some("VARCHAR")),
                ("c", Some("NVARCHAR")),
                ("p", Some("NVARCHAR")),
            ],
        },
        SqlServerOutputsTestCase {
            description: "TRIM keeps its argument's type, and LOWER of it too",
            sql: "SELECT TRIM(label) AS t, LOWER(TRIM(label)) AS l FROM orders",
            expected_outputs: &[("t", Some("NVARCHAR(40)")), ("l", Some("NVARCHAR(40)"))],
        },
        SqlServerOutputsTestCase {
            description: "string functions read a non-string as varchar, NCHAR as NVARCHAR, and an \
                untyped argument stays untyped",
            sql: "SELECT UPPER(id) AS i, LTRIM(created_at) AS c, LOWER(amount) AS a, \
                UPPER(CAST(label AS NCHAR(5))) AS n, UPPER(CAST(status AS CHAR(5))) AS h, \
                UPPER(ISNULL(label, N'x')) AS u, TRIM(id) AS t FROM orders",
            expected_outputs: &[
                ("i", Some("VARCHAR")),
                ("c", Some("VARCHAR")),
                ("a", Some("VARCHAR")),
                ("n", Some("NVARCHAR(5)")),
                ("h", Some("VARCHAR(5)")),
                ("u", None),
                ("t", None),
            ],
        },
        SqlServerOutputsTestCase {
            description: "a literal with an exponent is float",
            sql: "SELECT 1e10 AS e, 1.5E3 AS f",
            expected_outputs: &[("e", Some("FLOAT")), ("f", Some("FLOAT"))],
        },
        SqlServerOutputsTestCase {
            description: "an integer literal past int is numeric of its digits, but -2147483648 is int",
            sql: "SELECT 2147483647 AS i, 2147483648 AS d, 0012345678901 AS z, -2147483648 AS m, \
                -2147483649 AS n",
            expected_outputs: &[
                ("i", Some("INT")),
                ("d", Some("DECIMAL(10,0)")),
                ("z", Some("DECIMAL(11,0)")),
                ("m", Some("INT")),
                ("n", Some("DECIMAL(10,0)")),
            ],
        },
        SqlServerOutputsTestCase {
            description: "FLOAT casts are float, REAL and FLOAT(24) real, and a bare CHAR thirty long",
            sql: "SELECT CAST(id AS FLOAT) AS f, CAST(id AS REAL) AS r, CAST(id AS FLOAT(24)) AS g, \
                CAST(id AS FLOAT(25)) AS h, CAST(id AS CHAR) AS c FROM orders",
            expected_outputs: &[
                ("f", Some("FLOAT")),
                ("r", Some("REAL")),
                ("g", Some("REAL")),
                ("h", Some("FLOAT")),
                ("c", Some("CHAR(30)")),
            ],
        },
        SqlServerOutputsTestCase {
            description: "COUNT is int and COUNT_BIG bigint",
            sql: "SELECT COUNT(*) AS n, COUNT(DISTINCT status) AS d, COUNT_BIG(*) AS b FROM orders",
            expected_outputs: &[
                ("n", Some("INT")),
                ("d", Some("INT")),
                ("b", Some("BIGINT")),
            ],
        },
        SqlServerOutputsTestCase {
            description: "SUM of float is float, of int int and of bigint bigint",
            sql: "SELECT SUM(amount) AS a, SUM(id) AS i, SUM(big) AS b FROM orders",
            expected_outputs: &[
                ("a", Some("FLOAT")),
                ("i", Some("INT")),
                ("b", Some("BIGINT")),
            ],
        },
    ];
    let schema = sqlserver_shop_schema();
    for test_case in test_cases {
        let outputs: Vec<(String, Option<String>)> = sqlserver_outputs(&schema, test_case.sql);
        let borrowed: Vec<(&str, Option<&str>)> = outputs
            .iter()
            .map(|(name, data_type)| (name.as_str(), data_type.as_deref()))
            .collect();
        assert_eq!(
            borrowed, test_case.expected_outputs,
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
        let statement = sqlquack_syntax_sqlserver::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        assert_eq!(
            output_ranges(
                &statement,
                &SQL_SERVER,
                &duckdb_shop_schema(),
                test_case.sql
            ),
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
