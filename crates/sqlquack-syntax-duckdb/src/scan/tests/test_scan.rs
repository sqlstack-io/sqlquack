//! Tokens of the DuckDB scanner, as DuckDB 2.0.0's PEG tokenizer splits the same text.

use std::path::Path;

use serde_json::Value;

use crate::scan::main::scan::scan;
use crate::scan::main::uncovered::uncovered;
use crate::scan::models::{StringPrefix, TokenKind};
use crate::scan::tests::test_types::{CorpusTestCase, TokensTestCase};

#[test]
fn given_sql_when_scanning_then_the_tokens_are_duckdb_s() {
    let test_cases = [
        TokensTestCase {
            description: "0xFF is the number 0 and the word xFF, as 2.0.0 reads SELECT 0xFF",
            sql: "0xFF",
            expected_tokens: &[(TokenKind::Number, "0"), (TokenKind::Word, "xFF")],
        },
        TokensTestCase {
            description: "an underscore between two digits is part of the number",
            sql: "1_000",
            expected_tokens: &[(TokenKind::Number, "1_000")],
        },
        TokensTestCase {
            description: "two underscores end the number, and the rest is a word",
            sql: "1__0",
            expected_tokens: &[(TokenKind::Number, "1"), (TokenKind::Word, "__0")],
        },
        TokensTestCase {
            description: "a doubled exponent letter gives back the first one",
            sql: "1ee5",
            expected_tokens: &[(TokenKind::Number, "1"), (TokenKind::Word, "ee5")],
        },
        TokensTestCase {
            description: "a sign after an exponent letter is part of the number, a later one is not",
            sql: "1e-5+1",
            expected_tokens: &[
                (TokenKind::Number, "1e-5"),
                (TokenKind::Op, "+"),
                (TokenKind::Number, "1"),
            ],
        },
        TokensTestCase {
            description: "1.2.3 is one number, which the grammar refuses",
            sql: "1.2.3",
            expected_tokens: &[(TokenKind::Number, "1.2.3")],
        },
        TokensTestCase {
            description: "a point before a word is a point, so t.c is three tokens",
            sql: "t.c",
            expected_tokens: &[
                (TokenKind::Word, "t"),
                (TokenKind::Char('.'), "."),
                (TokenKind::Word, "c"),
            ],
        },
        TokensTestCase {
            description: "an operator with ! keeps its trailing minus, as 1 !=- 1 calls !=-",
            sql: "1 !=- 1",
            expected_tokens: &[
                (TokenKind::Number, "1"),
                (TokenKind::Op, "!=-"),
                (TokenKind::Number, "1"),
            ],
        },
        TokensTestCase {
            description: "an operator without a special character gives back a trailing minus",
            sql: "1 <-1",
            expected_tokens: &[
                (TokenKind::Number, "1"),
                (TokenKind::Op, "<"),
                (TokenKind::Op, "-"),
                (TokenKind::Number, "1"),
            ],
        },
        TokensTestCase {
            description: ":: and := are operators, and ->> one run",
            sql: "a::INT x := j->>'$.k'",
            expected_tokens: &[
                (TokenKind::Word, "a"),
                (TokenKind::Op, "::"),
                (TokenKind::Word, "INT"),
                (TokenKind::Word, "x"),
                (TokenKind::Op, ":="),
                (TokenKind::Word, "j"),
                (TokenKind::Op, "->>"),
                (TokenKind::String(StringPrefix::Plain), "$.k"),
            ],
        },
        TokensTestCase {
            description: "a backslash is itself in a plain string and an escape after E",
            sql: r"'a\' E'b\'c' 'd''e'",
            expected_tokens: &[
                (TokenKind::String(StringPrefix::Plain), r"a\"),
                (TokenKind::String(StringPrefix::Escape), "b'c"),
                (TokenKind::String(StringPrefix::Plain), "d'e"),
            ],
        },
        TokensTestCase {
            description: "X'..' is a blob literal and N'..' a national string",
            sql: "x'41' N'n'",
            expected_tokens: &[
                (TokenKind::String(StringPrefix::Hex), "41"),
                (TokenKind::String(StringPrefix::National), "n"),
            ],
        },
        TokensTestCase {
            description: "both quote characters delimit identifiers, with a doubled one escaped",
            sql: "\"a\"\"b\" `c`",
            expected_tokens: &[(TokenKind::Quoted, "a\"b"), (TokenKind::Quoted, "c")],
        },
        TokensTestCase {
            description: "$$..$$ and $tag$..$tag$ are strings; $1 and $name are parameters",
            sql: "$$x$$ $a$y$a$ $1 $p",
            expected_tokens: &[
                (TokenKind::DollarString, "x"),
                (TokenKind::DollarString, "y"),
                (TokenKind::Char('$'), "$"),
                (TokenKind::Number, "1"),
                (TokenKind::Char('$'), "$"),
                (TokenKind::Word, "p"),
            ],
        },
        TokensTestCase {
            description: "nested block comments and line comments are skipped",
            sql: "/* a /* b */ c */ 1 -- d\n;",
            expected_tokens: &[(TokenKind::Number, "1"), (TokenKind::Char(';'), ";")],
        },
        TokensTestCase {
            description: "an unterminated string is a fault tokenizing raises",
            sql: "'a",
            expected_tokens: &[(TokenKind::Fault(true), "unterminated string literal")],
        },
        TokensTestCase {
            description: "an empty quoted identifier is a fault tokenizing raises",
            sql: "\"\"",
            expected_tokens: &[(TokenKind::Fault(true), "zero-length delimited identifier")],
        },
    ];
    for test_case in test_cases {
        let tokens = scan(test_case.sql);
        let found: Vec<(TokenKind, &str)> = tokens
            .iter()
            .map(|token| (token.kind, token.value.as_str()))
            .collect();
        assert_eq!(
            found, test_case.expected_tokens,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_the_core_corpus_when_scanning_then_every_statement_scans_without_a_gap() {
    let test_cases = [CorpusTestCase {
        description: "every DuckShop statement ported to DuckDB scans without a fault or a gap",
        corpus: "../../corpora/duckdb-core/duckshop.jsonl",
        expected_unscanned: &[],
    }];
    for test_case in test_cases {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(test_case.corpus);
        let text: String = std::fs::read_to_string(path).expect("the corpus is committed");
        let unscanned: Vec<String> = text
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("one JSON object per line"))
            .filter(|row| {
                let sql: &str = row["sql"].as_str().unwrap_or_default();
                let faulted: bool = scan(sql)
                    .iter()
                    .any(|token| matches!(token.kind, TokenKind::Fault(_)));
                faulted || uncovered(sql).is_some()
            })
            .map(|row| row["id"].to_string())
            .collect();
        assert_eq!(
            unscanned, test_case.expected_unscanned,
            "{}",
            test_case.description
        );
    }
}
