use crate::helpers::{convert_outcome, converted_column_names, run_convert};
use crate::test_types::{ColumnNameTestCase, ConvertTestCase};

const UNALIASED_SCHEMA: &str = "tests/cli/fixtures/unaliased_items.json";
const UNVERIFIED_NAME: &str = "has not verified the name Snowflake gives it";

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
            description: "CASE refuses with SQ0009",
            sql: "SELECT CASE WHEN n > 1 THEN 1 END FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "IFF refuses with SQ0009",
            sql: "SELECT IFF(n > 1, 1, 0) FROM input",
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
            description: "IS NULL refuses with SQ0009",
            sql: "SELECT n IS NULL FROM input",
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
            description: "NOT refuses with SQ0009",
            sql: "SELECT NOT TRUE FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "AND refuses with SQ0009",
            sql: "SELECT TRUE AND FALSE FROM input",
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
            description: "a nested call refuses with SQ0009",
            sql: "SELECT UPPER(LOWER(g)) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a parenthesised expression refuses with SQ0009",
            sql: "SELECT (n + 1) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a negated column refuses with SQ0009",
            sql: "SELECT -n FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "modulo refuses with SQ0009",
            sql: "SELECT n % 2 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a chained cast refuses with SQ0009",
            sql: "SELECT n::INT::STRING FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a cast spelled INT refuses with SQ0009",
            sql: "SELECT CAST(n AS INT) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "ROUND of an expression, which no receipt names, refuses with SQ0009",
            sql: "SELECT ROUND(n + 1, 1) FROM input",
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
            description: "DATEADD refuses with SQ0009",
            sql: "SELECT DATEADD(day, 1, t) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "TRY_CAST refuses with SQ0009",
            sql: "SELECT TRY_CAST(g AS INTEGER) FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a literal before a column refuses with SQ0009",
            sql: "SELECT 1 + n FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a tab refuses with SQ0009",
            sql: "SELECT n\t+ 1 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a double space refuses with SQ0009",
            sql: "SELECT n  + 1 FROM input",
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
            description: "a decimal without a leading digit refuses with SQ0009",
            sql: "SELECT .5 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "an integer with leading zeros refuses with SQ0009",
            sql: "SELECT 007 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "an exponent with a fraction and a sign refuses with SQ0009",
            sql: "SELECT 1.0e-3 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "an exponent with a plus sign refuses with SQ0009",
            sql: "SELECT 1E+3 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a spaced negative refuses with SQ0009",
            sql: "SELECT - 2 FROM input",
            flags: &["--schema", UNALIASED_SCHEMA],
            mention: UNVERIFIED_NAME,
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a negated parenthesised number refuses with SQ0009",
            sql: "SELECT -(2) FROM input",
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
