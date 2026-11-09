use sqlquack_syntax::Dialect;

use crate::helpers::{lossless_and_contiguous, missing_tokens, tokenizer_failed, trivia};
use crate::test_types::{
    LosslessTestCase, TokenPresenceTestCase, TokenizeFailureTestCase, TriviaTestCase,
};

#[test]
fn given_whitespace_and_comments_when_tokenizing_then_they_become_trivia() {
    let test_cases = [TriviaTestCase {
        description: "spaces, line comments and block comments are classified",
        sql: "SELECT a -- trailing\n  /* block */ FROM t\t// slashes\n",
        dialect: Dialect::Snowflake,
        expected_trivia: vec![
            ("Whitespace", " "),
            ("Whitespace", " "),
            ("LineComment", "-- trailing"),
            ("Whitespace", "\n  "),
            ("BlockComment", "/* block */"),
            ("Whitespace", " "),
            ("Whitespace", " "),
            ("Whitespace", "\t"),
            ("LineComment", "// slashes"),
            ("Whitespace", "\n"),
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            trivia(test_case.sql, test_case.dialect),
            test_case
                .expected_trivia
                .into_iter()
                .map(|(kind, text)| (kind.to_owned(), text.to_owned()))
                .collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_quoted_text_and_comments_when_tokenizing_then_each_keeps_its_exact_text() {
    let test_cases = [
        TokenPresenceTestCase {
            description: "strings and identifiers keep their quotes",
            sql: "SELECT 'it''s', \"Quoted Name\", $$dollar 'text'$$ FROM t",
            dialect: Dialect::Snowflake,
            wanted: &[
                ("Significant", "'it''s'"),
                ("Significant", "\"Quoted Name\""),
                ("Significant", "$$dollar 'text'$$"),
            ],
            expected_missing: vec![],
        },
        TokenPresenceTestCase {
            description: "a comment after multibyte text keeps its bytes",
            sql: "SELECT 'ünï' AS \"größe\" /* 日本 */ FROM t",
            dialect: Dialect::DuckDB,
            wanted: &[("BlockComment", "/* 日本 */")],
            expected_missing: vec![],
        },
        TokenPresenceTestCase {
            description: "DuckDB nests block comments",
            sql: "SELECT /* a /* b */ c */ 1",
            dialect: Dialect::DuckDB,
            wanted: &[("BlockComment", "/* a /* b */ c */")],
            expected_missing: vec![],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            missing_tokens(test_case.sql, test_case.dialect, test_case.wanted),
            test_case.expected_missing,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_tokenizing_then_every_byte_belongs_to_one_token_in_order() {
    let test_cases = [
        LosslessTestCase {
            description: "trivia joins back into the input",
            sql: "SELECT a -- trailing\n  /* block */ FROM t\t// slashes\n",
            dialect: Dialect::Snowflake,
            expected_lossless: true,
            expected_empty: false,
        },
        LosslessTestCase {
            description: "byte ranges follow multibyte text",
            sql: "SELECT 'ünï' AS \"größe\" /* 日本 */ FROM t",
            dialect: Dialect::DuckDB,
            expected_lossless: true,
            expected_empty: false,
        },
        LosslessTestCase {
            description: "Snowflake's flat block comments still cover the input",
            sql: "SELECT /* a /* b */ c */ 1",
            dialect: Dialect::Snowflake,
            expected_lossless: true,
            expected_empty: false,
        },
        LosslessTestCase {
            description: "hints are covered",
            sql: "SELECT /*+ hint */ 1;\n",
            dialect: Dialect::Snowflake,
            expected_lossless: true,
            expected_empty: false,
        },
        LosslessTestCase {
            description: "empty input has no tokens",
            sql: "",
            dialect: Dialect::Snowflake,
            expected_lossless: true,
            expected_empty: true,
        },
    ];

    for test_case in test_cases {
        let (lossless, count) = lossless_and_contiguous(test_case.sql, test_case.dialect);
        assert_eq!(
            (lossless, count == 0),
            (test_case.expected_lossless, test_case.expected_empty),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_untokenizable_sql_when_tokenizing_then_the_tokenizer_failure_is_reported() {
    let test_cases = [TokenizeFailureTestCase {
        description: "an unterminated string is a tokenizer failure",
        sql: "SELECT 'unterminated",
        dialect: Dialect::Snowflake,
        expected_failed: true,
    }];

    for test_case in test_cases {
        assert_eq!(
            tokenizer_failed(test_case.sql, test_case.dialect),
            test_case.expected_failed,
            "{}",
            test_case.description
        );
    }
}
