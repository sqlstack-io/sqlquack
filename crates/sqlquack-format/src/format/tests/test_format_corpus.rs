use serde_json::Value;

use crate::format::tests::helpers::check_dialect_corpus;
use crate::format::tests::test_types;

const FORMAT_CORPUS: &str = include_str!("fixtures/format_corpus.json");

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [
        test_types::FormatCorpusTestCase {
            description: "DuckDB identity corpus",
            dialect: "duckdb",
            expected_formatted: 189,
            expected_refused: 0,
            departures: &[
                test_types::Departure {
                    sql: "SELECT str[0:1]",
                    formatted: Some("SELECT str[0:1]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x, y : COALESCE(x, y, 0) + 1)",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x, y: COALESCE(x, y, 0) + 1)",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM(LIST_FILTER([0, 1, 2, 3, 4, 5], LAMBDA x : x % 2 = 0), LAMBDA y : y * y)",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM(LIST_FILTER([0, 1, 2, 3, 4, 5], LAMBDA x: x % 2 = 0), LAMBDA y: y * y)",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x : COALESCE(x, 0) + 1)",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x: COALESCE(x, 0) + 1)",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM(nbr, LAMBDA x : x + 1) FROM article AS a",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM(nbr, LAMBDA x: x + 1)\nFROM article AS a",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT JSON_EXTRACT(c, '$[*].id')[0:2]",
                    formatted: Some("SELECT JSON_EXTRACT(c, '$[*].id')[0:2]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT JSON_EXTRACT_STRING(c, '$[*].id')[0:2]",
                    formatted: Some("SELECT JSON_EXTRACT_STRING(c, '$[*].id')[0:2]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT [1, 2, 3][1 + 1:LENGTH([1, 2, 3]) + -1]",
                    formatted: Some("SELECT [1, 2, 3][1 + 1:LENGTH([1, 2, 3]) + -1]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT x::INT[3][3]",
                    formatted: Some("SELECT x::INT[3][3]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT ARRAY[[[1]]]::INT[1][1][1]",
                    formatted: Some("SELECT ARRAY[[[1]]]::INT[1][1][1]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "WITH t1 AS (FROM (FROM t2 SELECT foo1, foo2)) FROM t1",
                    formatted: Some(
                        "WITH t1 AS (\n  FROM (\n    FROM t2\n    SELECT\n      foo1,\n      foo2\n  )\n)\n\nFROM t1",
                    ),
                    reason: "SQLClean aligned against Polyglot's SELECT-first rewrite of the FROM-first query",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT e: 1 + 2, f: len('asdf'), s: (SELECT 42)",
                    formatted: Some(
                        "SELECT\n  e:1 + 2,\n  f:len('asdf'),\n  s:(\n    SELECT 42\n  )",
                    ),
                    reason: "SQLClean aligned against Polyglot's path reading of the prefix alias and lost the line break after a comma",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT (@col) + 1 FROM t",
                    formatted: Some("SELECT\n  (\n    @col\n  )\n  + 1\nFROM t"),
                    reason: "SQLClean aligned against Polyglot's ABS() rewrite of `@`",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT (@-1) + 1",
                    formatted: Some("SELECT\n  (\n    @-1\n  )\n  + 1"),
                    reason: "SQLClean aligned against Polyglot's ABS() rewrite of `@`",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT file[:256] FROM GLOB('*')",
                    formatted: Some("SELECT file[:256]\nFROM GLOB('*')"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
            ],
        },
        test_types::FormatCorpusTestCase {
            description: "SQL Server identity corpus",
            dialect: "tsql",
            expected_formatted: 52,
            expected_refused: 23,
            departures: &[
                test_types::Departure {
                    sql: "SELECT val FROM (VALUES ((TRUE), (FALSE), (NULL))) AS t(val)",
                    formatted: Some(
                        "SELECT val\nFROM (\n  VALUES\n    ((\n  TRUE\n), (\n  FALSE\n), (\n  NULL\n))\n) AS t(val)",
                    ),
                    reason: "The shared layout of a parenthesised value in a VALUES row, the same as Snowflake's; SQLClean broke after the values instead",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "WITH t1 AS (SELECT 1 AS a), t2 AS (SELECT 1 AS a) SELECT COUNT(*) FROM (SELECT TOP 10 a FROM t1 UNION ALL SELECT TOP 10 a FROM t2 ORDER BY a DESC) AS t",
                    formatted: Some(
                        "WITH t1 AS (\n  SELECT 1 AS a\n),\n\nt2 AS (\n  SELECT 1 AS a\n)\n\nSELECT COUNT(*)\nFROM (\n  SELECT TOP 10 a\n  FROM t1\n  UNION ALL\n  SELECT TOP 10 a\n  FROM t2\n  ORDER BY\n    a DESC\n) AS t",
                    ),
                    reason: "A set operation's ORDER BY sits at the depth of its set operation with its items on the next line; SQLClean outdented it to the enclosing parenthesis and kept the items on its line",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "WITH t1 AS (SELECT 1 AS a), t2 AS (SELECT 1 AS a) SELECT TOP 10 a FROM t1 UNION ALL SELECT TOP 10 a FROM t2 ORDER BY a DESC",
                    formatted: Some(
                        "WITH t1 AS (\n  SELECT 1 AS a\n),\n\nt2 AS (\n  SELECT 1 AS a\n)\n\nSELECT TOP 10 a\nFROM t1\nUNION ALL\nSELECT TOP 10 a\nFROM t2\nORDER BY\n  a DESC",
                    ),
                    reason: "A set operation's ORDER BY puts its items on the next line, as in every other dialect and SQL Server's own `ORDER BY\\n  1`; SQLClean kept them on the ORDER BY line",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT * FROM t TABLESAMPLE (10 PERCENT)",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a TABLESAMPLE clause opaque (gap `tablesample`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t TABLESAMPLE (20 ROWS)",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a TABLESAMPLE clause opaque (gap `tablesample`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t TABLESAMPLE (10 PERCENT) REPEATABLE (123)",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a TABLESAMPLE clause opaque (gap `tablesample`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t WHERE NOT c",
                    formatted: None,
                    reason: "SQL Server rejects it: `An expression of non-boolean type specified in a context where a condition is expected, near 'c'.` (4145); SQLQuack's parser refuses it (`An expression of non-boolean type specified in a context where a condition is expected.`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t WITH (TABLOCK, INDEX(myindex))",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a table hint list opaque (gap `table_hint`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT Employee_ID, Department_ID FROM @MyTableVar",
                    formatted: None,
                    reason: "SQL Server rejects it: `Must declare the table variable \"@MyTableVar\".` (1087); SQLQuack's parser keeps a table variable source opaque (gap `table_variable`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT x FROM @MyTableVar AS m JOIN Employee ON m.EmployeeID = Employee.EmployeeID",
                    formatted: None,
                    reason: "SQL Server rejects it: `Must declare the table variable \"@MyTableVar\".` (1087); SQLQuack's parser keeps a table variable source opaque (gap `table_variable`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t FOR XML PATH, BINARY BASE64, ELEMENTS XSINIL",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR XML, FOR JSON or FOR BROWSE clause opaque (gap `for_xml_json`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT (SELECT TOP 5 a, b FROM t ORDER BY b DESC FOR JSON PATH) AS j FROM x",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR XML, FOR JSON or FOR BROWSE clause opaque (gap `for_xml_json`) on FOR JSON in a scalar subquery, a parser false reject; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT j FROM (SELECT a AS j FROM t FOR JSON PATH) AS x",
                    formatted: None,
                    reason: "SQL Server rejects it at binding: `Invalid column name 'j'.` (207, then 8155), as a FOR JSON derived table needs a column list; SQLQuack's parser keeps a FOR XML, FOR JSON or FOR BROWSE clause opaque (gap `for_xml_json`), even with one; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t FOR JSON PATH, ROOT('Root'), INCLUDE_NULL_VALUES",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR XML, FOR JSON or FOR BROWSE clause opaque (gap `for_xml_json`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t FOR BROWSE",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR XML, FOR JSON or FOR BROWSE clause opaque (gap `for_xml_json`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT TOP 10 PERCENT",
                    formatted: None,
                    reason: "SQL Server rejects it: `Incorrect syntax near 'PERCENT'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near the end of the input.`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT TOP 10 PERCENT WITH TIES",
                    formatted: None,
                    reason: "SQL Server rejects it: `Incorrect syntax near 'TIES'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near the end of the input.`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME AS OF 'foo'",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR SYSTEM_TIME clause opaque (gap `system_time`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME AS OF 'foo' AS alias",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR SYSTEM_TIME clause opaque (gap `system_time`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME FROM c TO d",
                    formatted: None,
                    reason: "SQL Server rejects it: `Incorrect syntax near 'c'.` (102); SQLQuack's parser keeps a FOR SYSTEM_TIME clause opaque (gap `system_time`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME BETWEEN c AND d",
                    formatted: None,
                    reason: "SQL Server rejects it: `Incorrect syntax near 'c'.` (102); SQLQuack's parser keeps a FOR SYSTEM_TIME clause opaque (gap `system_time`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME CONTAINED IN (c, d)",
                    formatted: None,
                    reason: "SQL Server rejects it: `Incorrect syntax near 'c'.` (102); SQLQuack's parser keeps a FOR SYSTEM_TIME clause opaque (gap `system_time`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME ALL AS alias",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a FOR SYSTEM_TIME clause opaque (gap `system_time`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT x FROM a INNER LOOP JOIN b ON b.id = a.id",
                    formatted: None,
                    reason: "SQL Server accepts it; SQLQuack's parser keeps a join hint opaque (gap `join_hint`), so the formatter refuses rather than guess; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT NEXT VALUE FOR db.schema.sequence_name OVER (ORDER BY foo), col",
                    formatted: None,
                    reason: "SQL Server rejects it: `Incorrect syntax near the keyword 'schema'.` (156); SQLQuack's parser refuses it (`Incorrect syntax near 'schema'.`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM @x",
                    formatted: None,
                    reason: "SQL Server rejects it: `Must declare the table variable \"@x\".` (1087); SQLQuack's parser keeps a table variable source opaque (gap `table_variable`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/sqlserver_receipts.jsonl",
                    ),
                },
            ],
        },
        test_types::FormatCorpusTestCase {
            description: "BigQuery identity corpus",
            dialect: "bigquery",
            expected_formatted: 161,
            expected_refused: 7,
            departures: &[
                test_types::Departure {
                    sql: "SELECT * FROM x.*",
                    formatted: None,
                    reason: "BigQuery rejects it: `Syntax error: Unexpected \"*\" at [1:17]`; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/bigquery_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM x.y*",
                    formatted: None,
                    reason: "BigQuery rejects it: a wildcard table needs backquotes (`Syntax error: Expected end of input but got \"*\" at [1:18]`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/bigquery_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT 1 AS hash",
                    formatted: None,
                    reason: "BigQuery rejects it: HASH is a reserved keyword (`Syntax error: Unexpected keyword HASH at [1:13]`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/bigquery_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT 1 AS at",
                    formatted: None,
                    reason: "BigQuery rejects it: AT is a reserved keyword (`Syntax error: Unexpected keyword AT at [1:13]`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/bigquery_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "WITH cte(c) AS (SELECT * FROM t) SELECT * FROM cte",
                    formatted: None,
                    reason: "BigQuery rejects it: GoogleSQL has no CTE column list (`Syntax error: Expected \")\" but got identifier \"c\" at [1:10]`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/bigquery_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t AS t(c1, c2)",
                    formatted: None,
                    reason: "BigQuery rejects it: GoogleSQL has no alias column list (`Syntax error: Expected end of input but got \"(\" at [1:21]`); SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/bigquery_receipts.jsonl",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT ARRAY(SELECT AS STRUCT 1 a, 2 b)",
                    formatted: Some(
                        "SELECT\n  ARRAY(\n    SELECT AS STRUCT\n      1 a,\n      2 b\n  )",
                    ),
                    reason: "SQLClean's Polyglot printer left a subquery call argument on the call's line; SQLQuack lays it out as any other subquery, as for PostgreSQL's ARRAY(SELECT ...)",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT STRUCT<STRING>((SELECT 'foo')).*",
                    formatted: Some("SELECT\n  STRUCT<STRING> ((\n    SELECT 'foo'\n  )).*"),
                    reason: "SQLClean's Polyglot printer put a space before `.*`; SQLQuack keeps the field access touching its operand",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "select array_contains([1, 2, 3], 1)",
                    formatted: Some("SELECT ARRAY_CONTAINS([1, 2, 3], 1)"),
                    reason: "SQLQuack upper-cases a called function name, as in every dialect, and BigQuery function names are case-insensitive; SQLClean kept the authored case",
                    receipt: None,
                },
            ],
        },
        test_types::FormatCorpusTestCase {
            description: "PostgreSQL identity corpus",
            dialect: "postgres",
            expected_formatted: 124,
            expected_refused: 13,
            departures: &[
                test_types::Departure {
                    sql: "SELECT id, name FROM xml_data AS t, XMLTABLE('/root/user' PASSING t.xml COLUMNS id INT PATH '@id', name TEXT PATH 'name/text()') AS x",
                    formatted: Some(
                        "SELECT\n  id,\n  name\nFROM xml_data AS t, XMLTABLE(\n  '/root/user' PASSING t.xml COLUMNS id INT PATH '@id',\n  name TEXT PATH 'name/text()'\n) AS x",
                    ),
                    reason: "SQLClean's Polyglot printer put XMLTABLE's PASSING and COLUMNS on lines of their own; SQLQuack lays XMLTABLE out as any call, splitting its arguments at commas, and PostgreSQL 18.6 returns the same columns and rows for both",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT id, value FROM xml_content AS t, XMLTABLE(XMLNAMESPACES('http://example.com/ns1' AS ns1, 'http://example.com/ns2' AS ns2), '/root/data' PASSING t.xml COLUMNS id INT PATH '@ns1:id', value TEXT PATH 'ns2:value/text()') AS x",
                    formatted: Some(
                        "SELECT\n  id,\n  value\nFROM xml_content AS t, XMLTABLE(\n  XMLNAMESPACES('http://example.com/ns1' AS ns1, 'http://example.com/ns2' AS ns2),\n  '/root/data' PASSING t.xml COLUMNS id INT PATH '@ns1:id',\n  value TEXT PATH 'ns2:value/text()'\n) AS x",
                    ),
                    reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42P01",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM ROWS FROM (FUNC1(col1) AS alias1(\"col1\" TEXT), FUNC2(col2) AS alias2(\"col2\" INT)) WITH ORDINALITY",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects a table alias on a ROWS FROM function, and SQLQuack's parser refuses it too; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 41: syntax error at or near \"alias1\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM table1, ROWS FROM (FUNC1(col1) AS alias1(\"col1\" TEXT)) WITH ORDINALITY AS alias3(\"col3\" INT, \"col4\" TEXT)",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects a table alias on a ROWS FROM function, and SQLQuack's parser refuses it too; SQLClean formatted it",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 49: syntax error at or near \"alias1\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT to_timestamp(123)::time without time zone",
                    formatted: Some("SELECT TO_TIMESTAMP(123)::time without time zone"),
                    reason: "SQLQuack upper-cases a called function name, as in every dialect, and PostgreSQL folds an unquoted function name to lower case; SQLClean kept the authored case",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT ARRAY(SELECT 1)",
                    formatted: Some("SELECT\n  ARRAY(\n    SELECT 1\n  )"),
                    reason: "SQLClean aligned against Polyglot's layout, which broke the subquery after SELECT",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT ~x",
                    formatted: None,
                    reason: "the spacing guard refuses a space between tokens written touching here; left for a later guard pass",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT date_col - INTERVAL '30' DAY FROM t",
                    formatted: Some("SELECT date_col - INTERVAL '30' DAY\nFROM t"),
                    reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42P01",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT date_col - INTERVAL '1' HOUR AS one_hour_later",
                    formatted: Some("SELECT date_col - INTERVAL '1' HOUR AS one_hour_later"),
                    reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42703",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT date_add(current_date, interval '7' day)",
                    formatted: Some("SELECT DATE_ADD(CURRENT_DATE, INTERVAL '7' DAY)"),
                    reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too; SQLQuack upper-cases the called name, as in every dialect, and PostgreSQL folds an unquoted function name",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it and runs it",
                    ),
                },
                test_types::Departure {
                    sql: "select count() OVER(partition by a order by a range offset preceding exclude current row)",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 53: syntax error at or near \"offset\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT JSON_ARRAY_ELEMENTS((foo->'sections')::JSON) AS sections",
                    formatted: None,
                    reason: "the spacing guard refuses a space between tokens written touching here; left for a later guard pass",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT * FROM t1*",
                    formatted: None,
                    reason: "the spacing guard refuses a space between tokens written touching here; left for a later guard pass",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT js, js IS JSON ARRAY WITH UNIQUE KEYS AS \"array w. UK?\", js IS JSON ARRAY WITHOUT UNIQUE KEYS AS \"array w/o UK?\", js IS JSON ARRAY UNIQUE KEYS AS \"array w UK 2?\" FROM t",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 146: syntax error at or near \"KEYS\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM foo WHERE id = %s",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 30: syntax error at or near \"%\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM foo WHERE id = %(id_param)s",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 30: syntax error at or near \"%\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT * FROM foo WHERE id = ?",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 31: syntax error at end of input",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT MLEAST(VARIADIC ARRAY[]::numeric[])",
                    formatted: Some("SELECT MLEAST(VARIADIC ARRAY[]::numeric[])"),
                    reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42883",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT a, LOGICAL_OR(b) FROM table GROUP BY a",
                    formatted: None,
                    reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                    receipt: Some(
                        "crates/sqlquack-format/src/format/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 30: syntax error at or near \"table\"",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT U&'d!0061t!+000061' UESCAPE '!' AS label",
                    formatted: None,
                    reason: "PostgreSQL's scanner reads `U&'...' UESCAPE '!'` as one string, which the shared tokenizer splits",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT a <<->> b",
                    formatted: None,
                    reason: "PostgreSQL's scanner reads `<<->>` as one operator, which the shared tokenizer splits",
                    receipt: None,
                },
            ],
        },
        test_types::FormatCorpusTestCase {
            description: "Snowflake identity corpus",
            dialect: "snowflake",
            expected_formatted: 426,
            expected_refused: 9,
            departures: &[
                test_types::Departure {
                    sql: "WITH t (SELECT 1 AS c) SELECT c FROM t",
                    formatted: None,
                    reason: "Snowflake rejects a CTE body without AS, so SQLQuack's parser refuses it",
                    receipt: Some("sf.query.with.select.k8b29420d51a4"),
                },
                test_types::Departure {
                    sql: "SELECT DATE_PART('month' FROM CAST('2024-04-08' AS DATE))",
                    formatted: None,
                    reason: "Snowflake rejects a quoted date part in the FROM form of DATE_PART, so SQLQuack's parser refuses it",
                    receipt: Some("sf.functions.date-part.date-part-from.k10b598e047b5"),
                },
                test_types::Departure {
                    sql: "SELECT * FROM t, UNNEST(x) WITH ORDINALITY",
                    formatted: None,
                    reason: "Snowflake has no UNNEST, so SQLQuack's parser refuses it",
                    receipt: Some("sf.query.format-probe.k47586a1689b7"),
                },
                test_types::Departure {
                    sql: "WITH t AS (SELECT PARSE_JSON('{\"level1\": {\"level2\": {\"level3\": \"value\"}}}') AS data) SELECT data:     level1  : level2 : level3::VARIANT FROM t",
                    formatted: Some(
                        "WITH t AS (\n  SELECT PARSE_JSON('{\"level1\": {\"level2\": {\"level3\": \"value\"}}}') AS data\n)\n\nSELECT data:     level1  : level2 : level3::VARIANT\nFROM t",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, spaces included, so the item keeps its spacing; SQLClean respaced it",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT RPAD(TO_BINARY('Hi', 'UTF8'), 10, TO_BINARY('_', 'UTF8'))",
                    formatted: Some(
                        "SELECT\n  RPAD(TO_BINARY('Hi', 'UTF8'), 10, TO_BINARY('_', 'UTF8'))",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT TIMESTAMP_FROM_PARTS(CAST('2024-05-09' AS DATE), CAST('14:30:45' AS TIME))",
                    formatted: Some(
                        "SELECT\n  TIMESTAMP_FROM_PARTS(CAST('2024-05-09' AS DATE), CAST('14:30:45' AS TIME))",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT ARRAYS_ZIP([1, 2, 3], ['a', 'b', 'c'], [10, 20, 30])",
                    formatted: Some(
                        "SELECT\n  ARRAYS_ZIP([1, 2, 3], ['a', 'b', 'c'], [10, 20, 30])",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT AI_CLASSIFY('text', ['travel', 'cooking'], OBJECT_CONSTRUCT('output_mode', 'multi'))",
                    formatted: Some(
                        "SELECT\n  AI_CLASSIFY('text', ['travel', 'cooking'], OBJECT_CONSTRUCT('output_mode', 'multi'))",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT {fn DAYNAME('2022-5-13')}",
                    formatted: Some("SELECT {fn DAYNAME('2022-5-13')}"),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT {fn LOG(5)}",
                    formatted: Some("SELECT {fn LOG(5)}"),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT {fn CEILING(5.3)}",
                    formatted: Some("SELECT {fn CEILING(5.3)}"),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT TIME_SLICE(CAST('2024-05-09 08:50:57.891' AS TIMESTAMP), 1, 'HOUR', 'start')",
                    formatted: Some(
                        "SELECT\n  TIME_SLICE(CAST('2024-05-09 08:50:57.891' AS TIMESTAMP), 1, 'HOUR', 'start')",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT SEARCH(line, 'king', ANALYZER => 'UNICODE_ANALYZER', SEARCH_MODE => 'OR')",
                    formatted: Some(
                        "SELECT\n  SEARCH(line, 'king', ANALYZER => 'UNICODE_ANALYZER', SEARCH_MODE => 'OR')",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT SEARCH(line, 'king', SEARCH_MODE => 'AND', ANALYZER => 'PATTERN_ANALYZER')",
                    formatted: Some(
                        "SELECT\n  SEARCH(line, 'king', SEARCH_MODE => 'AND', ANALYZER => 'PATTERN_ANALYZER')",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
            ],
        },
    ];
    let corpus: Vec<Value> = serde_json::from_str(FORMAT_CORPUS).unwrap_or_default();
    for test_case in test_cases {
        let outcome = check_dialect_corpus(&corpus, &test_case);
        assert_eq!(
            (
                outcome.formatted,
                outcome.refused,
                outcome.departed,
                outcome.failures.len()
            ),
            (
                test_case.expected_formatted,
                test_case.expected_refused,
                test_case.departures.len(),
                0
            ),
            "{}: {}",
            test_case.description,
            outcome.failures.join("\n\n")
        );
    }
}
