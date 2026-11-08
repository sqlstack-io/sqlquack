//! Tokens of the SQLite scanner, as SQLite 3.53.4's `sqlite3GetToken` splits the same text.

use crate::scan::main::scan::scan;
use crate::scan::main::uncovered::uncovered;
use crate::scan::models::{NumberKind, Quote, TokenKind};
use crate::scan::tests::test_types::{TokensTestCase, UncoveredTestCase};

#[test]
fn given_sql_when_scanning_then_the_tokens_are_sqlite_s() {
    let test_cases = [
        TokensTestCase {
            description: "each identifier quote is unquoted, a doubled quote standing for one",
            sql: "\"a\"\"b\" `c``d` [e\"f]",
            expected_tokens: &[
                (TokenKind::Quoted(Quote::Double), "a\"b"),
                (TokenKind::Quoted(Quote::Backtick), "c`d"),
                (TokenKind::Quoted(Quote::Bracket), "e\"f"),
            ],
        },
        TokensTestCase {
            description: "a bracketed name ends at the first ], so a doubled ] is not one",
            sql: "[a]]",
            expected_tokens: &[
                (TokenKind::Quoted(Quote::Bracket), "a"),
                (TokenKind::Fault, "unrecognized token: \"]\""),
            ],
        },
        TokensTestCase {
            description: "an unterminated quote is unreadable to the end of the input",
            sql: "'it''s' \"open",
            expected_tokens: &[
                (TokenKind::String, "it's"),
                (TokenKind::Fault, "unrecognized token: \"\"open\""),
            ],
        },
        TokensTestCase {
            description: "|| is concatenation, | alone a bitwise or, and -> and ->> one token each",
            sql: "a||b|c->d->>e",
            expected_tokens: &[
                (TokenKind::Ident, "a"),
                (TokenKind::Op("CONCAT"), "||"),
                (TokenKind::Ident, "b"),
                (TokenKind::Op("BITOR"), "|"),
                (TokenKind::Ident, "c"),
                (TokenKind::Op("PTR"), "->"),
                (TokenKind::Ident, "d"),
                (TokenKind::Op("PTR"), "->>"),
                (TokenKind::Ident, "e"),
            ],
        },
        TokensTestCase {
            description: "== and <> are equality operators, and ! is only part of !=",
            sql: "== <> != !",
            expected_tokens: &[
                (TokenKind::Op("EQ"), "=="),
                (TokenKind::Op("NE"), "<>"),
                (TokenKind::Op("NE"), "!="),
                (TokenKind::Fault, "unrecognized token: \"!\""),
            ],
        },
        TokensTestCase {
            description: "a blob is an even number of hex digits; its value is the digits",
            sql: "x'0aFF' X'abc'",
            expected_tokens: &[
                (TokenKind::Blob, "0aFF"),
                (TokenKind::Fault, "unrecognized token: \"X'abc'\""),
            ],
        },
        TokensTestCase {
            description: "0x starts a hexadecimal integer, kept as written",
            sql: "0x1F 0x",
            expected_tokens: &[
                (TokenKind::Number(NumberKind::Integer), "0x1F"),
                (TokenKind::Fault, "unrecognized token: \"0x\""),
            ],
        },
        TokensTestCase {
            description: "points and exponents make a float",
            sql: "1e5 .5 1. 1e",
            expected_tokens: &[
                (TokenKind::Number(NumberKind::Float), "1e5"),
                (TokenKind::Number(NumberKind::Float), ".5"),
                (TokenKind::Number(NumberKind::Float), "1."),
                (TokenKind::Fault, "unrecognized token: \"1e\""),
            ],
        },
        TokensTestCase {
            description: "digit separators make a QNUMBER, its value without them unless misplaced",
            sql: "1_000 1__0",
            expected_tokens: &[
                (TokenKind::Number(NumberKind::Separated), "1000"),
                (TokenKind::Number(NumberKind::Misseparated), "1__0"),
            ],
        },
        TokensTestCase {
            description: "keywords keep their spelling; several words share one grammar token",
            sql: "Left glob current_timestamp temporary",
            expected_tokens: &[
                (TokenKind::Keyword("JOIN_KW"), "Left"),
                (TokenKind::Keyword("LIKE_KW"), "glob"),
                (TokenKind::Keyword("CTIME_KW"), "current_timestamp"),
                (TokenKind::Keyword("TEMP"), "temporary"),
            ],
        },
        TokensTestCase {
            description: "OVER and FILTER are keywords only after ) and before ( or a name",
            sql: "f(x) filter (where 1) over w, over (x) filter",
            expected_tokens: &[
                (TokenKind::Ident, "f"),
                (TokenKind::Op("LP"), "("),
                (TokenKind::Ident, "x"),
                (TokenKind::Op("RP"), ")"),
                (TokenKind::Keyword("FILTER"), "filter"),
                (TokenKind::Op("LP"), "("),
                (TokenKind::Keyword("WHERE"), "where"),
                (TokenKind::Number(NumberKind::Integer), "1"),
                (TokenKind::Op("RP"), ")"),
                (TokenKind::Keyword("OVER"), "over"),
                (TokenKind::Ident, "w"),
                (TokenKind::Op("COMMA"), ","),
                (TokenKind::Ident, "over"),
                (TokenKind::Op("LP"), "("),
                (TokenKind::Ident, "x"),
                (TokenKind::Op("RP"), ")"),
                (TokenKind::Ident, "filter"),
            ],
        },
        TokensTestCase {
            description: "WINDOW is a keyword only when a name and AS follow",
            sql: "window w as window",
            expected_tokens: &[
                (TokenKind::Keyword("WINDOW"), "window"),
                (TokenKind::Ident, "w"),
                (TokenKind::Keyword("AS"), "as"),
                (TokenKind::Ident, "window"),
            ],
        },
        TokensTestCase {
            description: "parameters, with Tcl's suffix, and $ inside a name",
            sql: "? ?12 :a @b $c::d(e) a$b",
            expected_tokens: &[
                (TokenKind::Variable, "?"),
                (TokenKind::Variable, "?12"),
                (TokenKind::Variable, ":a"),
                (TokenKind::Variable, "@b"),
                (TokenKind::Variable, "$c::d(e)"),
                (TokenKind::Ident, "a$b"),
            ],
        },
        TokensTestCase {
            description: "comments and a byte order mark are blanks; an open /* runs to the end",
            sql: "\u{feff}a -- b\n/* c */ d /* e",
            expected_tokens: &[(TokenKind::Ident, "a"), (TokenKind::Ident, "d")],
        },
    ];
    for test_case in test_cases {
        let tokens: Vec<(TokenKind, String)> = scan(test_case.sql)
            .into_iter()
            .map(|token| (token.kind, token.value))
            .collect();
        let tokens: Vec<(TokenKind, &str)> = tokens
            .iter()
            .map(|(kind, value)| (*kind, value.as_str()))
            .collect();
        assert_eq!(
            tokens, test_case.expected_tokens,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_finding_uncovered_bytes_then_only_text_sqlite_never_reads_is() {
    let test_cases = [
        UncoveredTestCase {
            description: "tokens, blanks and comments cover the whole input",
            sql: "SELECT 1 -- done",
            expected_uncovered: None,
        },
        UncoveredTestCase {
            description: "SQLite stops reading at a NUL byte",
            sql: "SELECT 1\0; DROP TABLE t",
            expected_uncovered: Some(8),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            uncovered(test_case.sql),
            test_case.expected_uncovered,
            "{}",
            test_case.description
        );
    }
}
