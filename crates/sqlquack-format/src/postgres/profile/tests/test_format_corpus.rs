use crate::format::models::FormatDialect;
use crate::format::tests::helpers::corpus_outcome;
use crate::format::tests::test_types::Departure;
use crate::postgres::profile::tests::test_types;

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [test_types::FormatCorpusTestCase {
        description: "PostgreSQL identity corpus",
        dialect: "postgres",
        expected_formatted: 124,
        expected_refused: 13,
        departures: &[
            Departure {
                sql: "SELECT id, name FROM xml_data AS t, XMLTABLE('/root/user' PASSING t.xml COLUMNS id INT PATH '@id', name TEXT PATH 'name/text()') AS x",
                formatted: Some(
                    "SELECT\n  id,\n  name\nFROM xml_data AS t, XMLTABLE(\n  '/root/user' PASSING t.xml COLUMNS id INT PATH '@id',\n  name TEXT PATH 'name/text()'\n) AS x",
                ),
                reason: "SQLClean's Polyglot printer put XMLTABLE's PASSING and COLUMNS on lines of their own; SQLQuack lays XMLTABLE out as any call, splitting its arguments at commas, and PostgreSQL 18.6 returns the same columns and rows for both",
                receipt: None,
            },
            Departure {
                sql: "SELECT id, value FROM xml_content AS t, XMLTABLE(XMLNAMESPACES('http://example.com/ns1' AS ns1, 'http://example.com/ns2' AS ns2), '/root/data' PASSING t.xml COLUMNS id INT PATH '@ns1:id', value TEXT PATH 'ns2:value/text()') AS x",
                formatted: Some(
                    "SELECT\n  id,\n  value\nFROM xml_content AS t, XMLTABLE(\n  XMLNAMESPACES('http://example.com/ns1' AS ns1, 'http://example.com/ns2' AS ns2),\n  '/root/data' PASSING t.xml COLUMNS id INT PATH '@ns1:id',\n  value TEXT PATH 'ns2:value/text()'\n) AS x",
                ),
                reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42P01",
                ),
            },
            Departure {
                sql: "SELECT * FROM ROWS FROM (FUNC1(col1) AS alias1(\"col1\" TEXT), FUNC2(col2) AS alias2(\"col2\" INT)) WITH ORDINALITY",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects a table alias on a ROWS FROM function, and SQLQuack's parser refuses it too; SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 41: syntax error at or near \"alias1\"",
                ),
            },
            Departure {
                sql: "SELECT * FROM table1, ROWS FROM (FUNC1(col1) AS alias1(\"col1\" TEXT)) WITH ORDINALITY AS alias3(\"col3\" INT, \"col4\" TEXT)",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects a table alias on a ROWS FROM function, and SQLQuack's parser refuses it too; SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 49: syntax error at or near \"alias1\"",
                ),
            },
            Departure {
                sql: "SELECT to_timestamp(123)::time without time zone",
                formatted: Some("SELECT TO_TIMESTAMP(123)::time without time zone"),
                reason: "SQLQuack upper-cases a called function name, as in every dialect, and PostgreSQL folds an unquoted function name to lower case; SQLClean kept the authored case",
                receipt: None,
            },
            Departure {
                sql: "SELECT ARRAY(SELECT 1)",
                formatted: Some("SELECT\n  ARRAY(\n    SELECT 1\n  )"),
                reason: "SQLClean aligned against Polyglot's layout, which broke the subquery after SELECT",
                receipt: None,
            },
            Departure {
                sql: "SELECT ~x",
                formatted: None,
                reason: "the spacing guard refuses a space between tokens written touching here; left for a later guard pass",
                receipt: None,
            },
            Departure {
                sql: "SELECT date_col - INTERVAL '30' DAY FROM t",
                formatted: Some("SELECT date_col - INTERVAL '30' DAY\nFROM t"),
                reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42P01",
                ),
            },
            Departure {
                sql: "SELECT date_col - INTERVAL '1' HOUR AS one_hour_later",
                formatted: Some("SELECT date_col - INTERVAL '1' HOUR AS one_hour_later"),
                reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42703",
                ),
            },
            Departure {
                sql: "SELECT date_add(current_date, interval '7' day)",
                formatted: Some("SELECT DATE_ADD(CURRENT_DATE, INTERVAL '7' DAY)"),
                reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too; SQLQuack upper-cases the called name, as in every dialect, and PostgreSQL folds an unquoted function name",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it and runs it",
                ),
            },
            Departure {
                sql: "select count() OVER(partition by a order by a range offset preceding exclude current row)",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 53: syntax error at or near \"offset\"",
                ),
            },
            Departure {
                sql: "SELECT JSON_ARRAY_ELEMENTS((foo->'sections')::JSON) AS sections",
                formatted: None,
                reason: "the spacing guard refuses a space between tokens written touching here; left for a later guard pass",
                receipt: None,
            },
            Departure {
                sql: "SELECT * FROM t1*",
                formatted: None,
                reason: "the spacing guard refuses a space between tokens written touching here; left for a later guard pass",
                receipt: None,
            },
            Departure {
                sql: "SELECT js, js IS JSON ARRAY WITH UNIQUE KEYS AS \"array w. UK?\", js IS JSON ARRAY WITHOUT UNIQUE KEYS AS \"array w/o UK?\", js IS JSON ARRAY UNIQUE KEYS AS \"array w UK 2?\" FROM t",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 146: syntax error at or near \"KEYS\"",
                ),
            },
            Departure {
                sql: "SELECT * FROM foo WHERE id = %s",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 30: syntax error at or near \"%\"",
                ),
            },
            Departure {
                sql: "SELECT * FROM foo WHERE id = %(id_param)s",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 30: syntax error at or near \"%\"",
                ),
            },
            Departure {
                sql: "SELECT * FROM foo WHERE id = ?",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 31: syntax error at end of input",
                ),
            },
            Departure {
                sql: "SELECT MLEAST(VARIADIC ARRAY[]::numeric[])",
                formatted: Some("SELECT MLEAST(VARIADIC ARRAY[]::numeric[])"),
                reason: "SQLClean refused it; PostgreSQL 18.6 parses it, and SQLQuack's parser does too",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 parses it, then stops at binding with 42883",
                ),
            },
            Departure {
                sql: "SELECT a, LOGICAL_OR(b) FROM table GROUP BY a",
                formatted: None,
                reason: "PostgreSQL 18.6 rejects it as a syntax error, and the native parser agrees",
                receipt: Some(
                    "crates/sqlquack-format/src/postgres/profile/tests/fixtures/pg18.6_receipts.jsonl: PostgreSQL 18.6 rejects it, 42601 at 30: syntax error at or near \"table\"",
                ),
            },
            Departure {
                sql: "SELECT U&'d!0061t!+000061' UESCAPE '!' AS label",
                formatted: None,
                reason: "PostgreSQL's scanner reads `U&'...' UESCAPE '!'` as one string, which the shared tokenizer splits",
                receipt: None,
            },
            Departure {
                sql: "SELECT a <<->> b",
                formatted: None,
                reason: "PostgreSQL's scanner reads `<<->>` as one operator, which the shared tokenizer splits",
                receipt: None,
            },
        ],
    }];
    for test_case in test_cases {
        let outcome = corpus_outcome(
            FormatDialect::PostgreSQL,
            test_case.dialect,
            test_case.departures,
        );
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
