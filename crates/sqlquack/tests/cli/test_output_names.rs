use crate::helpers::{convert_outcome, converted_column_names, names, run_convert};
use crate::test_types::{ColumnNameTestCase, ConvertTestCase};

const UNALIASED_SCHEMA: &str = "tests/cli/fixtures/unaliased_items.json";
const UNVERIFIED_NAME: &str = "has not verified the name Snowflake gives it";
/// The 392-character sum's name: its first 255 characters, ending in "52 + ", as recorded.
const LONG_NAME: &str = "X + 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 + 17 + 18 + 19 + 20 + 21 + 22 + 23 + 24 + 25 + 26 + 27 + 28 + 29 + 30 + 31 + 32 + 33 + 34 + 35 + 36 + 37 + 38 + 39 + 40 + 41 + 42 + 43 + 44 + 45 + 46 + 47 + 48 + 49 + 50 + 51 + 52 + ";

#[test]
fn given_unaliased_items_when_converted_and_run_on_duckdb_then_they_take_snowflakes_names() {
    let test_cases = [
        ColumnNameTestCase {
            description: "aggregates and a literal, named as recorded",
            sql: "SELECT COUNT(*), 'MiXeD text', Count( DISTINCT n ) FROM (SELECT 1 AS n) AS input",
            flags: &[],
            expected_names: vec![
                "COUNT(*)".to_owned(),
                "'MIXED TEXT'".to_owned(),
                "COUNT( DISTINCT N )".to_owned(),
            ],
        },
        ColumnNameTestCase {
            description: "a column-and-number expression across a line break keeps the break",
            sql: "SELECT n\n  + 1 FROM (SELECT 1 AS n) AS input",
            flags: &[],
            expected_names: vec!["N\n  + 1".to_owned()],
        },
        ColumnNameTestCase {
            description: "ROUND and the current-time items, named as recorded",
            sql: "SELECT ROUND(n, 1), ROUND(n), round( n, 0 ), current_date(), CURRENT_DATE, \
                  CURRENT_TIMESTAMP() FROM (SELECT 1.25 AS n) AS input",
            flags: &[],
            expected_names: vec![
                "ROUND(N, 1)".to_owned(),
                "ROUND(N)".to_owned(),
                "ROUND( N, 0 )".to_owned(),
                "CURRENT_DATE()".to_owned(),
                "CURRENT_DATE".to_owned(),
                "CURRENT_TIMESTAMP()".to_owned(),
            ],
        },
        ColumnNameTestCase {
            description: "an alias spelled as the current-time keyword runs on DuckDB",
            sql: "SELECT CURRENT_DATE() AS current_date, CURRENT_TIMESTAMP() AS current_timestamp",
            flags: &[],
            expected_names: vec!["CURRENT_DATE".to_owned(), "CURRENT_TIMESTAMP".to_owned()],
        },
        ColumnNameTestCase {
            description: "DISTINCT aggregates, named as recorded",
            sql: "SELECT SUM(DISTINCT n), MAX(DISTINCT n) FROM (SELECT 1.25 AS n) AS input",
            flags: &[],
            expected_names: vec!["SUM(DISTINCT N)".to_owned(), "MAX(DISTINCT N)".to_owned()],
        },
        ColumnNameTestCase {
            description: "COALESCE of three and of numbers, a text cast and a spaced call, as recorded",
            sql: "SELECT COALESCE(n, 0, 1), COALESCE(1, 2), CAST(g AS INTEGER), lower(g ) \
                  FROM (SELECT 1.25 AS n, '7' AS g) AS input",
            flags: &[],
            expected_names: vec![
                "COALESCE(N, 0, 1)".to_owned(),
                "COALESCE(1, 2)".to_owned(),
                "CAST(G AS INTEGER)".to_owned(),
                "LOWER(G )".to_owned(),
            ],
        },
        ColumnNameTestCase {
            description: "numbers keep their written form, upper-cased",
            sql: "SELECT 1.50, 1e3, -2, NULL, false",
            flags: &[],
            expected_names: vec![
                "1.50".to_owned(),
                "1E3".to_owned(),
                "-2".to_owned(),
                "NULL".to_owned(),
                "FALSE".to_owned(),
            ],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_column_names(test_case.sql, test_case.flags),
            test_case.expected_names,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unaliased_expressions_from_the_receipts_when_run_on_duckdb_then_they_take_snowflakes_names()
 {
    let test_cases = [
        ColumnNameTestCase {
            description: "a first query's basics: arithmetic, the clock, a cast, IFF, DATE_TRUNC and ||",
            sql: "SELECT 1 + 1, CURRENT_DATE(), x::VARCHAR, IFF(x > 1, 'big', 'small'), \
                  DATE_TRUNC('month', ts), a || b, total - discount FROM (SELECT 2 AS x, 'p' AS a, \
                  'q' AS b, 10.50 AS total, 1.25 AS discount, \
                  '2024-03-15 10:30:00'::TIMESTAMP_NTZ AS ts) AS input",
            flags: &[],
            expected_names: names(&[
                "1 + 1",
                "CURRENT_DATE()",
                "X::VARCHAR",
                "IFF(X > 1, 'BIG', 'SMALL')",
                "DATE_TRUNC('MONTH', TS)",
                "A || B",
                "TOTAL - DISCOUNT",
            ]),
        },
        ColumnNameTestCase {
            description: "nested expressions keep their spacing, tabs and comments, upper-cased",
            sql: "SELECT (total - discount) * 2, iff(x is null, 0, x), CAST(total - discount AS VARCHAR), \
                  DATEADD(day, 1, d), a  ||  b, x\t+ 1, NOT x > 1, x % 2, -x, x /* note */ + 1 \
                  FROM (SELECT 2 AS x, 'p' AS a, 'q' AS b, 10.50 AS total, 1.25 AS discount, \
                  '2024-03-15'::DATE AS d) AS input",
            flags: &[],
            expected_names: names(&[
                "(TOTAL - DISCOUNT) * 2",
                "IFF(X IS NULL, 0, X)",
                "CAST(TOTAL - DISCOUNT AS VARCHAR)",
                "DATEADD(DAY, 1, D)",
                "A  ||  B",
                "X\t+ 1",
                "NOT X > 1",
                "X % 2",
                "-X",
                "X /* NOTE */ + 1",
            ]),
        },
        ColumnNameTestCase {
            description: "literals are named as written, numbers too, and typed literals with their keyword",
            sql: "SELECT 1., .5, 1e3, 1E-2, -0.25, true, 'text', date '2024-01-01', \
                  TIMESTAMP '2024-01-01 10:00:00'",
            flags: &[],
            expected_names: names(&[
                "1.",
                ".5",
                "1E3",
                "1E-2",
                "-0.25",
                "TRUE",
                "'TEXT'",
                "DATE '2024-01-01'",
                "TIMESTAMP '2024-01-01 10:00:00'",
            ]),
        },
        ColumnNameTestCase {
            description: "casts to text, TRY_CAST, CASE and literal arithmetic, as recorded",
            sql: "SELECT CAST(total AS VARCHAR), total::VARCHAR, TO_VARCHAR(total), \
                  TRY_CAST(status AS VARCHAR), CASE WHEN total > 100 THEN 'big' ELSE 'small' END, \
                  1 / 2, 'a' || 'b' FROM (SELECT 10.50 AS total, 'open' AS status) AS input",
            flags: &[],
            expected_names: names(&[
                "CAST(TOTAL AS VARCHAR)",
                "TOTAL::VARCHAR",
                "TO_VARCHAR(TOTAL)",
                "TRY_CAST(STATUS AS VARCHAR)",
                "CASE WHEN TOTAL > 100 THEN 'BIG' ELSE 'SMALL' END",
                "1 / 2",
                "'A' || 'B'",
            ]),
        },
        ColumnNameTestCase {
            description: "MIN and MAX of an expression, as recorded",
            sql: "SELECT MIN(total), max(total) - min(total) FROM (SELECT 10.50 AS total) AS input",
            flags: &[],
            expected_names: names(&["MIN(TOTAL)", "MAX(TOTAL) - MIN(TOTAL)"]),
        },
        ColumnNameTestCase {
            description: "a name longer than 255 characters is cut there, as recorded",
            sql: "SELECT x + 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 + 17 + 18 + 19 + 20 + 21 + 22 + 23 + 24 + 25 + 26 + 27 + 28 + 29 + 30 + 31 + 32 + 33 + 34 + 35 + 36 + 37 + 38 + 39 + 40 + 41 + 42 + 43 + 44 + 45 + 46 + 47 + 48 + 49 + 50 + 51 + 52 + 53 + 54 + 55 + 56 + 57 + 58 + 59 + 60 + 61 + 62 + 63 + 64 + 65 + 66 + 67 + 68 + 69 + 70 + 71 + 72 + 73 + 74 + 75 + 76 + 77 + 78 + 79 + 80 FROM (SELECT 1 AS x) AS input",
            flags: &[],
            expected_names: vec![LONG_NAME.to_owned()],
        },
        ColumnNameTestCase {
            description: "a name repeated in the outermost query is kept twice, as Snowflake keeps it",
            sql: "SELECT x, x FROM (SELECT 1 AS x) AS input UNION ALL SELECT 2, 3",
            flags: &[],
            expected_names: names(&["X", "X"]),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_column_names(test_case.sql, test_case.flags),
            test_case.expected_names,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_avg_distinct_of_a_number_when_converting_then_it_refuses() {
    let test_cases = [ConvertTestCase {
        description: "AVG(DISTINCT) of a NUMBER, which Snowflake types NUMBER(25,5) for NUMBER(10,2), refuses",
        sql: "SELECT AVG(DISTINCT n) AS a FROM (SELECT 1.25 AS n) AS input",
        flags: &[],
        mention: "AVG(DISTINCT) of a NUMBER",
        expected_outcome: convert_outcome(1, false),
    }];
    for test_case in test_cases {
        assert_eq!(
            run_convert(test_case.sql, test_case.flags, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unaliased_items_no_receipt_names_when_converting_then_each_refuses_with_sq0009() {
    let test_cases = [
        ConvertTestCase {
            description: "a semi-structured path, which Snowflake names after the whole path refuses with SQ0009",
            sql: "SELECT v:a FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a nested path refuses with SQ0009",
            sql: "SELECT v:a.b FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a subscript by key refuses with SQ0009",
            sql: "SELECT v['a'] FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a path with a subscript by index refuses with SQ0009",
            sql: "SELECT v:a[0] FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a path from a quoted column refuses with SQ0009",
            sql: "SELECT \"V\":a FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a path from a qualified column refuses with SQ0009",
            sql: "SELECT input.v:a FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a path from a call refuses with SQ0009",
            sql: "SELECT PARSE_JSON('{}'):a FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a cast of a path refuses with SQ0009",
            sql: "SELECT v:a::STRING FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a scalar subquery refuses with SQ0009",
            sql: "SELECT (SELECT 1) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "EXISTS refuses with SQ0009",
            sql: "SELECT EXISTS (SELECT 1) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "IN refuses with SQ0009",
            sql: "SELECT n IN (1, 2) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "BETWEEN refuses with SQ0009",
            sql: "SELECT n BETWEEN 1 AND 2 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "LIKE refuses with SQ0009",
            sql: "SELECT g LIKE 'a%' FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "IS NOT DISTINCT FROM refuses with SQ0009",
            sql: "SELECT n IS NOT DISTINCT FROM 1 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a window function refuses with SQ0009",
            sql: "SELECT ROW_NUMBER() OVER (ORDER BY n) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "CURRENT_TIMESTAMP without parentheses, which no receipt names, refuses with SQ0009",
            sql: "SELECT CURRENT_TIMESTAMP FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "CURRENT_DATE( ) with a space, which no receipt names, refuses with SQ0009",
            sql: "SELECT CURRENT_DATE( ) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a space before the parenthesis refuses with SQ0009",
            sql: "SELECT COUNT (*) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a CRLF line break refuses with SQ0009",
            sql: "SELECT n\r\n+ 1 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "text with a tab refuses with SQ0009",
            sql: "SELECT 'a\tb' FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "empty text refuses with SQ0009",
            sql: "SELECT '' FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a quoted identifier that is not upper case refuses with SQ0009",
            sql: "SELECT \"lc\" + 1 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "non-ASCII text, which Snowflake upper-cases beyond ASCII, refuses with SQ0009",
            sql: "SELECT 'café' FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "text with a doubled quote, which has another spelling, refuses with SQ0009",
            sql: "SELECT 'it''s' FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a DATE literal spelled with two spaces, which the tree cannot show, refuses with SQ0009",
            sql: "SELECT DATE  '2024-01-01' FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a line comment inside the item refuses with SQ0009",
            sql: "SELECT n -- note\n+ 1 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "an item across two line breaks refuses with SQ0009",
            sql: "SELECT n\n+\n1 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a function no receipt names refuses with SQ0009 and suggests the exact alias",
            sql: "SELECT FLOOR(n) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: "add an alias, such as `AS \"FLOOR(N)\"`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a repeated name inside a subquery, which DuckDB renames, refuses",
            sql: "SELECT 1 AS one FROM (SELECT n, n FROM input)",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: "inside a subquery or CTE, where DuckDB renames the second column",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a repeated name in an ordered set operation, which the converted SQL wraps, refuses",
            sql: "SELECT n, n FROM input UNION ALL SELECT 1, 2 ORDER BY 1",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: "in a set operation the converted SQL wraps",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a star join repeating names beside a VARIANT, which the converted SQL wraps, refuses",
            sql: "SELECT * FROM input JOIN input AS t2 ON input.n = t2.n",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: "wraps the result to write VARIANT or NULL columns",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "ORDER BY a name two different items share, which no recording resolves, refuses",
            sql: "SELECT n AS a, g AS a FROM input ORDER BY a",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: "where ORDER BY, GROUP BY, HAVING or QUALIFY could read either column",
            expected_outcome: convert_outcome(1, false),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_convert(test_case.sql, test_case.flags, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
