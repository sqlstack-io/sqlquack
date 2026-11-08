//! The lexer against token lists ClickHouse 26.1's own `Lexer.cpp` produced for the same inputs.

use crate::scan::main::lex::lex;
use crate::scan::models::TokenKind as K;
use crate::scan::tests::test_types::LexTestCase;

#[test]
fn given_sql_when_lexing_then_the_tokens_are_clickhouses() {
    let test_cases = [
        LexTestCase {
            description: "a dot after a name or number is tuple access, and digits then letters are a word",
            sql: "SELECT a.1.2, 1abc, 0x1F, 1_000, .5e3",
            expected_tokens: vec![
                (K::BareWord, 0, 6),
                (K::Whitespace, 6, 7),
                (K::BareWord, 7, 8),
                (K::Dot, 8, 9),
                (K::Number, 9, 10),
                (K::Dot, 10, 11),
                (K::Number, 11, 12),
                (K::Comma, 12, 13),
                (K::Whitespace, 13, 14),
                (K::BareWord, 14, 18),
                (K::Comma, 18, 19),
                (K::Whitespace, 19, 20),
                (K::Number, 20, 24),
                (K::Comma, 24, 25),
                (K::Whitespace, 25, 26),
                (K::Number, 26, 31),
                (K::Comma, 31, 32),
                (K::Whitespace, 32, 33),
                (K::Number, 33, 37),
            ],
        },
        LexTestCase {
            description: "# starts a comment only before a blank or !, so #c is an error token then a word",
            sql: "SELECT 1 #c",
            expected_tokens: vec![
                (K::BareWord, 0, 6),
                (K::Whitespace, 6, 7),
                (K::Number, 7, 8),
                (K::Whitespace, 8, 9),
                (K::Error, 9, 10),
                (K::BareWord, 10, 11),
            ],
        },
        LexTestCase {
            description: "Unicode quotes make a string and a name, and the Unicode minus is a minus",
            sql: "SELECT ‘a’ − “b”",
            expected_tokens: vec![
                (K::BareWord, 0, 6),
                (K::Whitespace, 6, 7),
                (K::StringLiteral, 7, 14),
                (K::Whitespace, 14, 15),
                (K::Minus, 15, 18),
                (K::Whitespace, 18, 19),
                (K::QuotedIdentifier, 19, 26),
            ],
        },
        LexTestCase {
            description: "a heredoc, a $-led word and a word of digits with doubled underscores",
            sql: "SELECT $t$x$t$, $x, 1__2",
            expected_tokens: vec![
                (K::BareWord, 0, 6),
                (K::Whitespace, 6, 7),
                (K::HereDoc, 7, 14),
                (K::Comma, 14, 15),
                (K::Whitespace, 15, 16),
                (K::BareWord, 16, 18),
                (K::Comma, 18, 19),
                (K::Whitespace, 19, 20),
                (K::BareWord, 20, 24),
            ],
        },
        LexTestCase {
            description: "block comments nest",
            sql: "/* a /* b */ c */ x",
            expected_tokens: vec![
                (K::Comment, 0, 17),
                (K::Whitespace, 17, 18),
                (K::BareWord, 18, 19),
            ],
        },
    ];
    for test_case in test_cases {
        let tokens: Vec<(K, usize, usize)> = lex(test_case.sql)
            .into_iter()
            .map(|token| (token.kind, token.start, token.end))
            .collect();
        assert_eq!(
            tokens, test_case.expected_tokens,
            "{}",
            test_case.description
        );
    }
}
