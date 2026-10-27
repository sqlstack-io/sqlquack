use serde_json::json;

use crate::parser::models::Parser;
use crate::parser::tests::helpers::configured_parse_outcome;
use crate::parser::tests::helpers::data_type_outcome;
use crate::parser::tests::helpers::eof_keeps_the_parse;
use crate::parser::tests::helpers::fragment_changes_with_eof;
use crate::parser::tests::helpers::nonempty_fragments;
use crate::parser::tests::helpers::parse_outcome;
use crate::parser::tests::helpers::parse_sql_outcome;
use crate::parser::tests::helpers::sourced_parse_outcome;
use crate::parser::tests::helpers::statement_outcome;
use crate::parser::tests::helpers::stream;
use crate::parser::tests::helpers::terminated_value_at;
use crate::parser::tests::test_types::EmptyStreamTestCase;
use crate::parser::tests::test_types::EntryPointTestCase;
use crate::parser::tests::test_types::FragmentEofTestCase;
use crate::parser::tests::test_types::SourceEntryTestCase;
use crate::parser::tests::test_types::TerminatedValueTestCase;
use crate::parser::tests::test_types::TerminatorTestCase;
use crate::parser::tests::test_types::TrailingKeywordTestCase;

#[test]
fn given_streams_empty_once_normalized_when_parsing_fragments_then_no_expression_is_found() {
    let test_cases = [
        EmptyStreamTestCase {
            description: "a stream of only a terminator gives every fragment parser nothing, twice",
            parts: &["<EOF>"],
            expected_nonempty_fragments: vec![],
        },
        EmptyStreamTestCase {
            description: "an empty stream gives every fragment parser nothing, twice",
            parts: &[],
            expected_nonempty_fragments: vec![],
        },
        EmptyStreamTestCase {
            description: "a leading terminator gives every fragment parser nothing, twice",
            parts: &["<EOF>", "SELECT 1"],
            expected_nonempty_fragments: vec![],
        },
        EmptyStreamTestCase {
            description: "a duplicate terminator gives every fragment parser nothing, twice",
            parts: &["<EOF>", "<EOF>"],
            expected_nonempty_fragments: vec![],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            nonempty_fragments(test_case.parts),
            test_case.expected_nonempty_fragments,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_valid_fragment_when_a_terminator_follows_then_the_fragment_is_unchanged() {
    let test_cases = [
        FragmentEofTestCase {
            description: "disjunction reads the same fragment with every constructor",
            fragment: Parser::parse_disjunction,
            sql: "a OR b",
            expected_changed_constructors: vec![],
        },
        FragmentEofTestCase {
            description: "conjunction reads the same fragment with every constructor",
            fragment: Parser::parse_conjunction,
            sql: "a AND b",
            expected_changed_constructors: vec![],
        },
        FragmentEofTestCase {
            description: "select_or_expression reads the same fragment with every constructor",
            fragment: Parser::parse_select_or_expression,
            sql: "SELECT 1",
            expected_changed_constructors: vec![],
        },
        FragmentEofTestCase {
            description: "value reads the same fragment with every constructor",
            fragment: Parser::parse_value,
            sql: "(1, 2)",
            expected_changed_constructors: vec![],
        },
        FragmentEofTestCase {
            description: "set_item_assignment reads the same fragment with every constructor",
            fragment: Parser::parse_set_item_assignment,
            sql: "x = 1",
            expected_changed_constructors: vec![],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            fragment_changes_with_eof(test_case.fragment, test_case.sql),
            test_case.expected_changed_constructors,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_statement_when_a_terminator_follows_then_the_parse_is_unchanged() {
    let test_cases = [
        TerminatorTestCase {
            description: "SELECT BINARY parses the same with a terminator",
            sql: "SELECT BINARY",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT a OVERLAPS parses the same with a terminator",
            sql: "SELECT a OVERLAPS",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "BEGIN parses the same with a terminator",
            sql: "BEGIN",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "BEGIN TRANSACTION parses the same with a terminator",
            sql: "BEGIN TRANSACTION",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "START TRANSACTION parses the same with a terminator",
            sql: "START TRANSACTION",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "COMMIT parses the same with a terminator",
            sql: "COMMIT",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "ROLLBACK parses the same with a terminator",
            sql: "ROLLBACK",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "ALTER TABLE t UNSET prop parses the same with a terminator",
            sql: "ALTER TABLE t UNSET prop",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "ALTER TABLE t UNSET TAG x parses the same with a terminator",
            sql: "ALTER TABLE t UNSET TAG x",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "ALTER TABLE t UNSET PROJECTION POLICY parses the same with a terminator",
            sql: "ALTER TABLE t UNSET PROJECTION POLICY",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "ALTER TABLE t SET COMMENT = 'c' parses the same with a terminator",
            sql: "ALTER TABLE t SET COMMENT = 'c'",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "ALTER TABLE t ADD COLUMN c INT parses the same with a terminator",
            sql: "ALTER TABLE t ADD COLUMN c INT",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW TABLES parses the same with a terminator",
            sql: "SHOW TABLES",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW PRIMARY KEYS parses the same with a terminator",
            sql: "SHOW PRIMARY KEYS",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW TERSE DATABASES parses the same with a terminator",
            sql: "SHOW TERSE DATABASES",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW GRANTS FOR foo parses the same with a terminator",
            sql: "SHOW GRANTS FOR foo",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW PROFILE FOR QUERY 5 parses the same with a terminator",
            sql: "SHOW PROFILE FOR QUERY 5",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW GROUPS FOR ROLE parses the same with a terminator",
            sql: "SHOW GROUPS FOR ROLE",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SHOW CREATE TABLE t parses the same with a terminator",
            sql: "SHOW CREATE TABLE t",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT 1 parses the same with a terminator",
            sql: "SELECT 1",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT a, b FROM t WHERE c = 1 GROUP BY a HAVING COUNT(*) > 1 ORDER BY b LIMIT 10 parses the same with a terminator",
            sql: "SELECT a, b FROM t WHERE c = 1 GROUP BY a HAVING COUNT(*) > 1 ORDER BY b LIMIT 10",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT COUNT(*) OVER (PARTITION BY a ORDER BY b) FROM t parses the same with a terminator",
            sql: "SELECT COUNT(*) OVER (PARTITION BY a ORDER BY b) FROM t",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT CAST(x AS DECIMAL(10, 2)) FROM t parses the same with a terminator",
            sql: "SELECT CAST(x AS DECIMAL(10, 2)) FROM t",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "WITH c AS (SELECT 1 AS a) SELECT a FROM c parses the same with a terminator",
            sql: "WITH c AS (SELECT 1 AS a) SELECT a FROM c",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT a FROM t UNION ALL SELECT b FROM u parses the same with a terminator",
            sql: "SELECT a FROM t UNION ALL SELECT b FROM u",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "INSERT INTO t (a, b) VALUES (1, 2) parses the same with a terminator",
            sql: "INSERT INTO t (a, b) VALUES (1, 2)",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "UPDATE t SET a = 1 WHERE b = 2 parses the same with a terminator",
            sql: "UPDATE t SET a = 1 WHERE b = 2",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "DELETE FROM t WHERE a = 1 parses the same with a terminator",
            sql: "DELETE FROM t WHERE a = 1",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "MERGE INTO t USING u ON t.a = u.a WHEN MATCHED THEN UPDATE SET t.b = u.b parses the same with a terminator",
            sql: "MERGE INTO t USING u ON t.a = u.a WHEN MATCHED THEN UPDATE SET t.b = u.b",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "CREATE TABLE t (a INT, b VARCHAR(10)) parses the same with a terminator",
            sql: "CREATE TABLE t (a INT, b VARCHAR(10))",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "CREATE VIEW v AS SELECT 1 AS a parses the same with a terminator",
            sql: "CREATE VIEW v AS SELECT 1 AS a",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "CREATE INDEX i ON t (a ASC, b DESC) parses the same with a terminator",
            sql: "CREATE INDEX i ON t (a ASC, b DESC)",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "DROP TABLE IF EXISTS t parses the same with a terminator",
            sql: "DROP TABLE IF EXISTS t",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "TRUNCATE TABLE t parses the same with a terminator",
            sql: "TRUNCATE TABLE t",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "GRANT SELECT, INSERT ON t TO u parses the same with a terminator",
            sql: "GRANT SELECT, INSERT ON t TO u",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "REVOKE SELECT ON t FROM u parses the same with a terminator",
            sql: "REVOKE SELECT ON t FROM u",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "EXPLAIN SELECT 1 parses the same with a terminator",
            sql: "EXPLAIN SELECT 1",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SET x = 1 parses the same with a terminator",
            sql: "SET x = 1",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "COMMENT ON TABLE t IS 'c' parses the same with a terminator",
            sql: "COMMENT ON TABLE t IS 'c'",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT * FROM t WHERE a IN (SELECT b FROM u) parses the same with a terminator",
            sql: "SELECT * FROM t WHERE a IN (SELECT b FROM u)",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "SELECT CASE WHEN a THEN 1 ELSE 2 END FROM t parses the same with a terminator",
            sql: "SELECT CASE WHEN a THEN 1 ELSE 2 END FROM t",
            expected_outcome: (true, true),
        },
        TerminatorTestCase {
            description: "a trailing keyword in WITH cte AS (SELECT * FROM x) is read the same with a terminator",
            sql: "WITH cte AS (SELECT * FROM x)",
            expected_outcome: (eof_keeps_the_parse("WITH cte AS (SELECT * FROM x)").0, true),
        },
        TerminatorTestCase {
            description: "a trailing keyword in IF(a > 0) is read the same with a terminator",
            sql: "IF(a > 0)",
            expected_outcome: (eof_keeps_the_parse("IF(a > 0)").0, true),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            eof_keeps_the_parse(test_case.sql),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_trailing_keyword_when_a_terminator_follows_then_it_is_read_the_same() {
    let test_cases = [
        TrailingKeywordTestCase {
            description: "a trailing keyword in SELECT 1 is is read the same with a terminator",
            sql: "SELECT 1 is",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description:
                "a trailing keyword in SELECT * FROM t LIMIT 10% is read the same with a terminator",
            sql: "SELECT * FROM t LIMIT 10%",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description: "a trailing keyword in SELECT 1 limit is read the same with a terminator",
            sql: "SELECT 1 limit",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description: "a trailing keyword in SELECT 1 offset is read the same with a terminator",
            sql: "SELECT 1 offset",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description:
                "a trailing keyword in SELECT * FROM x prewhere is read the same with a terminator",
            sql: "SELECT * FROM x prewhere",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description:
                "a trailing keyword in SELECT * FROM x qualify is read the same with a terminator",
            sql: "SELECT * FROM x qualify",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description:
                "a trailing keyword in SELECT FROM x ORDER BY is read the same with a terminator",
            sql: "SELECT FROM x ORDER BY",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description: "a trailing keyword in CREATE TABLE a is read the same with a terminator",
            sql: "CREATE TABLE a",
            expected_same: true,
        },
        TrailingKeywordTestCase {
            description: "a trailing keyword in SELECT A[: is read the same with a terminator",
            sql: "SELECT A[:",
            expected_same: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            eof_keeps_the_parse(test_case.sql).1,
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unset_clauses_when_a_terminator_follows_then_raw_clauses_stop_at_it() {
    let test_cases = [
        TerminatedValueTestCase {
            description: "an unterminated raw UNSET clause keeps its text",
            sql: "ALTER TABLE t UNSET PROJECTION POLICY",
            terminated: false,
            pointer: "/alter_table/actions",
            expected_value: json!([{"Raw": {"sql": "UNSET PROJECTION POLICY"}}]),
        },
        TerminatedValueTestCase {
            description: "a terminated raw UNSET clause stops at the terminator",
            sql: "ALTER TABLE t UNSET PROJECTION POLICY",
            terminated: true,
            pointer: "/alter_table/actions",
            expected_value: json!([{"Raw": {"sql": "UNSET PROJECTION POLICY"}}]),
        },
        TerminatedValueTestCase {
            description: "a terminated UNSET of a property is not a raw clause",
            sql: "ALTER TABLE t UNSET prop",
            terminated: true,
            pointer: "/alter_table/actions",
            expected_value: json!([{"UnsetProperty": {"properties": ["prop"]}}]),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            terminated_value_at(test_case.sql, test_case.terminated, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_tokens_around_a_terminator_when_using_each_entry_point_then_tokens_after_it_are_refused() {
    let test_cases = [
        EntryPointTestCase {
            description: "parse refuses a statement after the terminator",
            parts: &["SELECT 1;", "<EOF>", "SELECT 2"],
            entry: parse_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "parse_statement refuses tokens after the terminator",
            parts: &["SELECT 1", "<EOF>", "SELECT 2"],
            entry: statement_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "data type parsing does not stop at the terminator",
            parts: &["INT", "<EOF>", "SELECT 2"],
            entry: data_type_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "SET does not read the terminator as its variable",
            parts: &["SET", "<EOF>", "x = 1"],
            entry: parse_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a trailing terminator is not an error",
            parts: &["SELECT 1", "<EOF>"],
            entry: parse_outcome,
            expected_outcome_part: "statements: 1",
        },
        EntryPointTestCase {
            description: "a data type before a trailing terminator parses",
            parts: &["INT", "<EOF>"],
            entry: data_type_outcome,
            expected_outcome_part: "data type",
        },
        EntryPointTestCase {
            description: "a leading terminator is refused by parse",
            parts: &["<EOF>", "SELECT 1"],
            entry: parse_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "the error points at the token after a leading terminator",
            parts: &["<EOF>", "SELECT 1"],
            entry: parse_outcome,
            expected_outcome_part: "line 1, column 7",
        },
        EntryPointTestCase {
            description: "a leading terminator is refused by parse_statement",
            parts: &["<EOF>", "SELECT 1"],
            entry: statement_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a leading terminator is refused by data type parsing",
            parts: &["<EOF>", "SELECT 1"],
            entry: data_type_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a leading terminator is refused with a config",
            parts: &["<EOF>", "SELECT 1"],
            entry: configured_parse_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a leading terminator is refused with the source",
            parts: &["<EOF>", "SELECT 1"],
            entry: sourced_parse_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a duplicate terminator is refused by parse",
            parts: &["<EOF>", "<EOF>"],
            entry: parse_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a duplicate terminator is refused by parse_statement",
            parts: &["<EOF>", "<EOF>"],
            entry: statement_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "a duplicate terminator is refused by data type parsing",
            parts: &["<EOF>", "<EOF>"],
            entry: data_type_outcome,
            expected_outcome_part: "Unexpected token after end of input",
        },
        EntryPointTestCase {
            description: "only a terminator parses to no statements",
            parts: &["<EOF>"],
            entry: parse_outcome,
            expected_outcome_part: "statements: 0",
        },
        EntryPointTestCase {
            description: "no tokens parse to no statements",
            parts: &[],
            entry: parse_outcome,
            expected_outcome_part: "statements: 0",
        },
        EntryPointTestCase {
            description: "only a terminator is no statement",
            parts: &["<EOF>"],
            entry: statement_outcome,
            expected_outcome_part: "Unexpected end of input",
        },
        EntryPointTestCase {
            description: "no tokens are no statement",
            parts: &[],
            entry: statement_outcome,
            expected_outcome_part: "Unexpected end of input",
        },
        EntryPointTestCase {
            description: "only a terminator is no data type",
            parts: &["<EOF>"],
            entry: data_type_outcome,
            expected_outcome_part: "Unexpected end of input",
        },
        EntryPointTestCase {
            description: "no tokens are no data type",
            parts: &[],
            entry: data_type_outcome,
            expected_outcome_part: "Unexpected end of input",
        },
    ];

    for test_case in test_cases {
        let outcome = (test_case.entry)(stream(test_case.parts));
        assert!(
            outcome.contains(test_case.expected_outcome_part),
            "{}: {outcome}",
            test_case.description
        );
    }
}

#[test]
fn given_blank_sql_when_parsing_then_there_are_no_statements() {
    let test_cases = [
        SourceEntryTestCase {
            description: "empty SQL has no statements",
            sql: "",
            expected_outcome: "statements: 0",
        },
        SourceEntryTestCase {
            description: "blank SQL has no statements",
            sql: "   ",
            expected_outcome: "statements: 0",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            parse_sql_outcome(test_case.sql),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
