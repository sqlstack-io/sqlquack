use crate::analysis::tests::helpers::setup_case_outcome;
use crate::analysis::tests::test_types::SetupCaseTestCase;

const NR: &[&str] = &[
    "CREATE TABLE nr (k NUMBER(38,0), v VARCHAR)",
    "INSERT INTO nr VALUES (1, 'one'), (2, 'two'), (3, NULL)",
];

#[test]
fn given_setup_tables_when_converting_a_case_then_duckdb_replays_them_or_it_refuses() {
    let test_cases = [
        SetupCaseTestCase {
            description: "a table read by the run database and schema placeholders",
            setup: NR,
            query: "SELECT t.v FROM input AS i JOIN {{run_database}}.{{run_schema}}.nr AS t ON t.k = i.a",
            expected_outcome: Ok("one"),
        },
        SetupCaseTestCase {
            description: "a table read by the schema placeholder alone",
            setup: NR,
            query: "SELECT t.v FROM input AS i JOIN {{run_schema}}.nr AS t ON t.k = i.a",
            expected_outcome: Ok("one"),
        },
        SetupCaseTestCase {
            description: "an inserted NULL stays NULL",
            setup: NR,
            query: "SELECT COUNT(v) AS n FROM nr",
            expected_outcome: Ok("2"),
        },
        SetupCaseTestCase {
            description: "a decimal keeps its column's scale",
            setup: &[
                "CREATE TABLE sales (id NUMBER(38,0), amount NUMBER(10,2))",
                "INSERT INTO sales VALUES (1, 2.5), (2, 4.00)",
            ],
            query: "SELECT SUM(amount) AS total FROM sales",
            expected_outcome: Ok("6.50"),
        },
        SetupCaseTestCase {
            description: "a value with more decimals than the column holds would round",
            setup: &[
                "CREATE TABLE sales (amount NUMBER(10,2))",
                "INSERT INTO sales VALUES (2.505)",
            ],
            query: "SELECT amount FROM sales",
            expected_outcome: Err("not a number it holds without rounding"),
        },
        SetupCaseTestCase {
            description: "a value with more whole digits than the column holds",
            setup: &[
                "CREATE TABLE sales (amount NUMBER(3,2))",
                "INSERT INTO sales VALUES (10)",
            ],
            query: "SELECT amount FROM sales",
            expected_outcome: Err("not a number it holds without rounding"),
        },
        SetupCaseTestCase {
            description: "a number into a VARCHAR column",
            setup: &["CREATE TABLE t (v VARCHAR)", "INSERT INTO t VALUES (1)"],
            query: "SELECT v FROM t",
            expected_outcome: Err("other than a string or NULL"),
        },
        SetupCaseTestCase {
            description: "a type other than NUMBER or VARCHAR",
            setup: &["CREATE TABLE t (d DATE)"],
            query: "SELECT d FROM t",
            expected_outcome: Err("which setup does not replay"),
        },
        SetupCaseTestCase {
            description: "a column constraint",
            setup: &["CREATE TABLE t (k NUMBER(38,0) PRIMARY KEY)"],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than unqualified, typed columns"),
        },
        SetupCaseTestCase {
            description: "a column default",
            setup: &["CREATE TABLE t (k NUMBER(38,0) DEFAULT 1)"],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than unqualified, typed columns"),
        },
        SetupCaseTestCase {
            description: "a table created twice",
            setup: &[
                "CREATE TABLE t (k NUMBER(38,0))",
                "CREATE TABLE t (v VARCHAR)",
            ],
            query: "SELECT k FROM t",
            expected_outcome: Err("creates T again"),
        },
        SetupCaseTestCase {
            description: "a quoted table name",
            setup: &["CREATE TABLE \"t\" (k NUMBER(38,0))"],
            query: "SELECT k FROM \"t\"",
            expected_outcome: Err("quotes the name t"),
        },
        SetupCaseTestCase {
            description: "an insert naming its columns",
            setup: &[
                "CREATE TABLE t (k NUMBER(38,0))",
                "INSERT INTO t (k) VALUES (1)",
            ],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than VALUES into an unqualified table"),
        },
        SetupCaseTestCase {
            description: "an insert from a query",
            setup: &["CREATE TABLE t (k NUMBER(38,0))", "INSERT INTO t SELECT 1"],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than VALUES"),
        },
        SetupCaseTestCase {
            description: "a row with fewer values than columns",
            setup: &[
                "CREATE TABLE t (k NUMBER(38,0), v VARCHAR)",
                "INSERT INTO t VALUES (1)",
            ],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a row of 1 values for 2 columns"),
        },
        SetupCaseTestCase {
            description: "an insert into a table setup did not create",
            setup: &["INSERT INTO t VALUES (1)"],
            query: "SELECT 1 AS k",
            expected_outcome: Err("which setup did not create"),
        },
        SetupCaseTestCase {
            description: "a session variable read in either case adds to a column",
            setup: &["SET MyVar = 10"],
            query: "SELECT a + $myvar AS n FROM input",
            expected_outcome: Ok("11"),
        },
        SetupCaseTestCase {
            description: "a pair of variables set in one statement keeps its order",
            setup: &["SET (lo, hi) = (40, 70)"],
            query: "SELECT $hi - $lo AS n",
            expected_outcome: Ok("30"),
        },
        SetupCaseTestCase {
            description: "a variable set from a subquery over a setup table",
            setup: &[
                "CREATE TABLE src (k NUMBER(38,0))",
                "INSERT INTO src VALUES (3), (7)",
                "SET top = (SELECT MAX(k) FROM src)",
            ],
            query: "SELECT $top AS n",
            expected_outcome: Ok("7"),
        },
        SetupCaseTestCase {
            description: "a variable the setup did not set",
            setup: &["SET n = 10"],
            query: "SELECT $m AS n",
            expected_outcome: Err("which the case's setup does not SET"),
        },
        SetupCaseTestCase {
            description: "a variable unset before the query",
            setup: &["SET n = 10", "UNSET n"],
            query: "SELECT $n AS n",
            expected_outcome: Err("which the case's setup does not SET"),
        },
        SetupCaseTestCase {
            description: "an unset of a variable never set",
            setup: &["UNSET n"],
            query: "SELECT 1 AS k",
            expected_outcome: Err("which setup did not set"),
        },
        SetupCaseTestCase {
            description: "a variable set to an expression, which SET computes once",
            setup: &["SET n = 10 + 1"],
            query: "SELECT $n AS n",
            expected_outcome: Err("other than a literal, NULL or a scalar subquery"),
        },
        SetupCaseTestCase {
            description: "a variable set from a subquery whose result changes between calls",
            setup: &["SET n = (SELECT RANDOM())"],
            query: "SELECT $n AS n",
            expected_outcome: Err("whose result can change"),
        },
        SetupCaseTestCase {
            description: "a variable set from a subquery reading the clock",
            setup: &["SET n = (SELECT CURRENT_TIMESTAMP())"],
            query: "SELECT $n AS n",
            expected_outcome: Err("whose result can change"),
        },
        SetupCaseTestCase {
            description: "a cast to a user-defined type named in another case casts to its base type",
            setup: &["CREATE TYPE Age AS NUMBER(3,0)"],
            query: "SELECT (a + 9)::AGE AS r FROM input",
            expected_outcome: Ok("10"),
        },
        SetupCaseTestCase {
            description: "a cast to a user-defined type keeps its base type's scale",
            setup: &["CREATE TYPE price AS NUMBER(5,2)"],
            query: "SELECT CAST(a AS price)::VARCHAR AS r FROM input",
            expected_outcome: Ok("1.00"),
        },
        SetupCaseTestCase {
            description: "setup table columns of a user-defined type add as its base type",
            setup: &[
                "CREATE TYPE age AS NUMBER(3,0)",
                "CREATE TABLE t (a age, b age)",
                "INSERT INTO t VALUES (10, 20)",
            ],
            query: "SELECT a + b AS n FROM t",
            expected_outcome: Ok("30"),
        },
        SetupCaseTestCase {
            description: "a user-defined type defined as another resolves to the first's base type",
            setup: &["CREATE TYPE a1 AS NUMBER(3,0)", "CREATE TYPE a2 AS a1"],
            query: "SELECT a::a2 AS r FROM input",
            expected_outcome: Ok("1"),
        },
        SetupCaseTestCase {
            description: "a quoted lower-case type name names no type setup created",
            setup: &["CREATE TYPE age AS NUMBER(3,0)"],
            query: "SELECT a::\"age\" AS r FROM input",
            expected_outcome: Err("unknown Snowflake type `\"age\"`"),
        },
        SetupCaseTestCase {
            description: "a user-defined type created twice",
            setup: &[
                "CREATE TYPE age AS NUMBER(3,0)",
                "CREATE TYPE age AS VARCHAR",
            ],
            query: "SELECT 1 AS k",
            expected_outcome: Err("creates the type AGE again"),
        },
        SetupCaseTestCase {
            description: "a user-defined type replacing another",
            setup: &["CREATE OR REPLACE TYPE age AS NUMBER(3,0)"],
            query: "SELECT 1 AS k",
            expected_outcome: Err("has a clause other than an unqualified name AS a data type"),
        },
        SetupCaseTestCase {
            description: "a user-defined type named for a built-in type",
            setup: &["CREATE TYPE number AS VARCHAR"],
            query: "SELECT 1 AS k",
            expected_outcome: Err("which names a built-in type"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            setup_case_outcome(
                (test_case.setup, test_case.query),
                test_case.expected_outcome
            ),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
