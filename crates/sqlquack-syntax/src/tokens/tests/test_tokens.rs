use crate::tokens::models::TokenType;
use crate::tokens::models::TokenizerConfig;
use crate::tokens::tests::helpers::backslash_escapes;
use crate::tokens::tests::helpers::fast_path_matches_character_path;
use crate::tokens::tests::helpers::parser_tokens_borrow_source;
use crate::tokens::tests::helpers::parser_tokens_match_public_tokens;
use crate::tokens::tests::helpers::terminal_backslash_recovery;
use crate::tokens::tests::helpers::trailing_comments;
use crate::tokens::tests::helpers::typed_texts;
use crate::tokens::tests::helpers::underscore_numbers;
use crate::tokens::tests::test_types::CommentTestCase;
use crate::tokens::tests::test_types::TokenPathTestCase;
use crate::tokens::tests::test_types::TokenizeTestCase;

#[test]
fn given_sql_when_tokenizing_then_each_token_has_its_type_and_text() {
    let test_cases = [
        TokenizeTestCase {
            description: "a number follows SELECT",
            config: TokenizerConfig::default(),
            sql: "SELECT 1",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "1"),
            ]),
        },
        TokenizeTestCase {
            description: "identifiers, commas and FROM",
            config: TokenizerConfig::default(),
            sql: "SELECT a, b FROM t",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Var, "a"),
                (TokenType::Comma, ","),
                (TokenType::Var, "b"),
                (TokenType::From, "FROM"),
                (TokenType::Var, "t"),
            ]),
        },
        TokenizeTestCase {
            description: "a string literal keeps its contents",
            config: TokenizerConfig::default(),
            sql: "SELECT 'hello'",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::String, "hello"),
            ]),
        },
        TokenizeTestCase {
            description: "a doubled quote is an escaped quote",
            config: TokenizerConfig::default(),
            sql: "SELECT 'it''s'",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::String, "it's"),
            ]),
        },
        TokenizeTestCase {
            description: "only configured escape followers decode",
            config: backslash_escapes(&['n']),
            sql: r"SELECT '\n\a\f\Z\x21'",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::String, "\nafZx21"),
            ]),
        },
        TokenizeTestCase {
            description: "numeric escapes need complete sequences",
            config: backslash_escapes(&['0', '1', '2', '3', '4', '5', '6', '7', 'x', 'u']),
            sql: r"SELECT '\041\x21\u26c4-\777\x2\u26c'",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::String, "!!\u{26c4}-777x2u26c"),
            ]),
        },
        TokenizeTestCase {
            description: "a terminal backslash before a quote is recovered",
            config: terminal_backslash_recovery(),
            sql: "SHOW FUNCTIONS LIKE 'a\\' OR 1=1",
            expected_tokens: Ok(vec![
                (TokenType::Show, "SHOW"),
                (TokenType::Var, "FUNCTIONS"),
                (TokenType::Like, "LIKE"),
                (TokenType::String, "a\\"),
                (TokenType::Or, "OR"),
                (TokenType::Number, "1"),
                (TokenType::Eq, "="),
                (TokenType::Number, "1"),
            ]),
        },
        TokenizeTestCase {
            description: "arithmetic operators",
            config: TokenizerConfig::default(),
            sql: "1 + 2 * 3",
            expected_tokens: Ok(vec![
                (TokenType::Number, "1"),
                (TokenType::Plus, "+"),
                (TokenType::Number, "2"),
                (TokenType::Star, "*"),
                (TokenType::Number, "3"),
            ]),
        },
        TokenizeTestCase {
            description: "== is one equality token by default",
            config: TokenizerConfig::default(),
            sql: "a == b",
            expected_tokens: Ok(vec![
                (TokenType::Var, "a"),
                (TokenType::Eq, "=="),
                (TokenType::Var, "b"),
            ]),
        },
        TokenizeTestCase {
            description: "comparison operators",
            config: TokenizerConfig::default(),
            sql: "a <= b >= c != d",
            expected_tokens: Ok(vec![
                (TokenType::Var, "a"),
                (TokenType::Lte, "<="),
                (TokenType::Var, "b"),
                (TokenType::Gte, ">="),
                (TokenType::Var, "c"),
                (TokenType::Neq, "!="),
                (TokenType::Var, "d"),
            ]),
        },
        TokenizeTestCase {
            description: "a national string is one token",
            config: TokenizerConfig::default(),
            sql: "N'abc'",
            expected_tokens: Ok(vec![(TokenType::NationalString, "abc")]),
        },
        TokenizeTestCase {
            description: "a hex string is one token",
            config: TokenizerConfig::default(),
            sql: "X'ABCD'",
            expected_tokens: Ok(vec![(TokenType::HexString, "ABCD")]),
        },
        TokenizeTestCase {
            description: "a bit string is one token",
            config: TokenizerConfig::default(),
            sql: "B'01010'",
            expected_tokens: Ok(vec![(TokenType::BitString, "01010")]),
        },
        TokenizeTestCase {
            description: "a trailing dot stays with the number",
            config: TokenizerConfig::default(),
            sql: "SELECT 1.",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "1."),
            ]),
        },
        TokenizeTestCase {
            description: "a fraction is one number",
            config: TokenizerConfig::default(),
            sql: "SELECT 1.5",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "1.5"),
            ]),
        },
        TokenizeTestCase {
            description: "a dot before letters ends the number",
            config: TokenizerConfig::default(),
            sql: "SELECT 1.a",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "1."),
                (TokenType::Var, "a"),
            ]),
        },
        TokenizeTestCase {
            description: "a range is a number, two dots and a number",
            config: TokenizerConfig::default(),
            sql: "SELECT 1..2",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "1"),
                (TokenType::Dot, "."),
                (TokenType::Dot, "."),
                (TokenType::Number, "2"),
            ]),
        },
        TokenizeTestCase {
            description: "a leading dot starts a number",
            config: TokenizerConfig::default(),
            sql: ".25",
            expected_tokens: Ok(vec![(TokenType::Number, ".25")]),
        },
        TokenizeTestCase {
            description: "a leading-dot number inside parentheses",
            config: TokenizerConfig::default(),
            sql: "SAMPLE (.25)",
            expected_tokens: Ok(vec![
                (TokenType::Sample, "SAMPLE"),
                (TokenType::LParen, "("),
                (TokenType::Number, ".25"),
                (TokenType::RParen, ")"),
            ]),
        },
        TokenizeTestCase {
            description: "a leading-dot number with an exponent",
            config: TokenizerConfig::default(),
            sql: ".5e10",
            expected_tokens: Ok(vec![(TokenType::Number, ".5e10")]),
        },
        TokenizeTestCase {
            description: "a dot between names is a dot",
            config: TokenizerConfig::default(),
            sql: "a.b",
            expected_tokens: Ok(vec![
                (TokenType::Var, "a"),
                (TokenType::Dot, "."),
                (TokenType::Var, "b"),
            ]),
        },
        TokenizeTestCase {
            description: "colon-equals is one token",
            config: TokenizerConfig::default(),
            sql: "a := 1",
            expected_tokens: Ok(vec![
                (TokenType::Var, "a"),
                (TokenType::ColonEq, ":="),
                (TokenType::Number, "1"),
            ]),
        },
        TokenizeTestCase {
            description: "a single colon is a colon",
            config: TokenizerConfig::default(),
            sql: "a:b",
            expected_tokens: Ok(vec![
                (TokenType::Var, "a"),
                (TokenType::Colon, ":"),
                (TokenType::Var, "b"),
            ]),
        },
        TokenizeTestCase {
            description: "a double colon is a cast",
            config: TokenizerConfig::default(),
            sql: "a::INT",
            expected_tokens: Ok(vec![
                (TokenType::Var, "a"),
                (TokenType::DColon, "::"),
                (TokenType::Var, "INT"),
            ]),
        },
        TokenizeTestCase {
            description: "underscores leave numbers when configured",
            config: underscore_numbers(),
            sql: "SELECT 1_2_3_4_5",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "12345"),
            ]),
        },
        TokenizeTestCase {
            description: "underscores leave thousands separators",
            config: underscore_numbers(),
            sql: "SELECT 20_000",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "20000"),
            ]),
        },
        TokenizeTestCase {
            description: "underscores leave exponents",
            config: underscore_numbers(),
            sql: "SELECT 1_2E+1_0",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "12E+10"),
            ]),
        },
        TokenizeTestCase {
            description: "underscores stay by default",
            config: TokenizerConfig::default(),
            sql: "SELECT 1_2_3_4_5",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::Number, "1_2_3_4_5"),
            ]),
        },
        TokenizeTestCase {
            description: "curly quotes are strings",
            config: TokenizerConfig::default(),
            sql: "SELECT \u{2018}hello\u{2019}",
            expected_tokens: Ok(vec![
                (TokenType::Select, "SELECT"),
                (TokenType::String, "hello"),
            ]),
        },
        TokenizeTestCase {
            description: "the number 13 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: "13END",
            expected_tokens: Ok(vec![(TokenType::Number, "13"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1.5 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: "1.5END",
            expected_tokens: Ok(vec![(TokenType::Number, "1.5"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number .5 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: ".5END",
            expected_tokens: Ok(vec![(TokenType::Number, ".5"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1e3 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: "1e3END",
            expected_tokens: Ok(vec![(TokenType::Number, "1e3"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1E+3 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: "1E+3END",
            expected_tokens: Ok(vec![(TokenType::Number, "1E+3"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1e-3 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: "1e-3END",
            expected_tokens: Ok(vec![(TokenType::Number, "1e-3"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number .5e2 keeps an adjacent END keyword",
            config: TokenizerConfig::default(),
            sql: ".5e2END",
            expected_tokens: Ok(vec![(TokenType::Number, ".5e2"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 13 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */13END",
            expected_tokens: Ok(vec![(TokenType::Number, "13"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1.5 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */1.5END",
            expected_tokens: Ok(vec![(TokenType::Number, "1.5"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number .5 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */.5END",
            expected_tokens: Ok(vec![(TokenType::Number, ".5"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1e3 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */1e3END",
            expected_tokens: Ok(vec![(TokenType::Number, "1e3"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1E+3 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */1E+3END",
            expected_tokens: Ok(vec![(TokenType::Number, "1E+3"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number 1e-3 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */1e-3END",
            expected_tokens: Ok(vec![(TokenType::Number, "1e-3"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "the number .5e2 keeps an adjacent END keyword after a comment",
            config: TokenizerConfig::default(),
            sql: "/* π */.5e2END",
            expected_tokens: Ok(vec![(TokenType::Number, ".5e2"), (TokenType::End, "END")]),
        },
        TokenizeTestCase {
            description: "1e3 is one number in scientific notation",
            config: TokenizerConfig::default(),
            sql: "1e3",
            expected_tokens: Ok(vec![(TokenType::Number, "1e3")]),
        },
        TokenizeTestCase {
            description: "1E+3 is one number in scientific notation",
            config: TokenizerConfig::default(),
            sql: "1E+3",
            expected_tokens: Ok(vec![(TokenType::Number, "1E+3")]),
        },
        TokenizeTestCase {
            description: "1e-3 is one number in scientific notation",
            config: TokenizerConfig::default(),
            sql: "1e-3",
            expected_tokens: Ok(vec![(TokenType::Number, "1e-3")]),
        },
        TokenizeTestCase {
            description: ".5e2 is one number in scientific notation",
            config: TokenizerConfig::default(),
            sql: ".5e2",
            expected_tokens: Ok(vec![(TokenType::Number, ".5e2")]),
        },
        TokenizeTestCase {
            description: "1.e2 is one number in scientific notation",
            config: TokenizerConfig::default(),
            sql: "1.e2",
            expected_tokens: Ok(vec![(TokenType::Number, "1.e2")]),
        },
        TokenizeTestCase {
            description: "an exponent marker without digits, 1e, is not part of the number",
            config: TokenizerConfig::default(),
            sql: "1e",
            expected_tokens: Ok(vec![(TokenType::Number, "1"), (TokenType::Var, "e")]),
        },
        TokenizeTestCase {
            description: "an exponent marker without digits, 1E+, is not part of the number",
            config: TokenizerConfig::default(),
            sql: "1E+",
            expected_tokens: Ok(vec![
                (TokenType::Number, "1"),
                (TokenType::Var, "E"),
                (TokenType::Plus, "+"),
            ]),
        },
        TokenizeTestCase {
            description: "an exponent marker without digits, .5e, is not part of the number",
            config: TokenizerConfig::default(),
            sql: ".5e",
            expected_tokens: Ok(vec![(TokenType::Number, ".5"), (TokenType::Var, "e")]),
        },
        TokenizeTestCase {
            description: "an exponent marker without digits, .5E-, is not part of the number",
            config: TokenizerConfig::default(),
            sql: ".5E-",
            expected_tokens: Ok(vec![
                (TokenType::Number, ".5"),
                (TokenType::Var, "E"),
                (TokenType::Dash, "-"),
            ]),
        },
        TokenizeTestCase {
            description: "an unknown character is an error",
            config: TokenizerConfig::default(),
            sql: "SELECT \u{2022} FROM t",
            expected_tokens: Err(
                "Tokenization error at line 1, column 9: Unexpected character: '\u{2022}'",
            ),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            typed_texts(test_case.config, test_case.sql),
            test_case
                .expected_tokens
                .map(|tokens| tokens
                    .into_iter()
                    .map(|(token_type, text)| (token_type, text.to_owned()))
                    .collect())
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_tokenizing_comments_then_they_trail_the_previous_token() {
    let test_cases = [CommentTestCase {
        description: "a line comment trails the token before it",
        sql: "SELECT -- comment\n1",
        expected_trailing_comments: vec![vec![" comment"], vec![]],
    }];

    for test_case in test_cases {
        assert_eq!(
            trailing_comments(test_case.sql),
            test_case.expected_trailing_comments,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_tokenizing_through_each_path_then_the_paths_agree() {
    let test_cases = [
        TokenPathTestCase {
            description:
                "the ASCII fast path matches the character path for plain identifiers and lists",
            sql: "SELECT a, b FROM t WHERE id IN (1, 2, 3)",
            agrees: fast_path_matches_character_path,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "the ASCII fast path matches the character path for quotes and comments",
            sql: "SELECT 'it''s', \"quoted\", $1 /* comment */ FROM schema.table",
            agrees: fast_path_matches_character_path,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "the ASCII fast path matches the character path for several statements",
            sql: "INSERT INTO t VALUES (1, 'a'), (2, 'b'); UPDATE t SET value = 'c'",
            agrees: fast_path_matches_character_path,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description:
                "the ASCII fast path matches the character path for dollar strings and numbers",
            sql: "SELECT $$body$$, $tag$content$tag$, 0xFF, 1.25e-2",
            agrees: fast_path_matches_character_path,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "parser tokens match public tokens for plain identifiers and numbers",
            sql: "SELECT alpha, 123, 'plain' FROM schema.table WHERE id = 42",
            agrees: parser_tokens_match_public_tokens,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "parser tokens match public tokens for quotes and dollar strings",
            sql: "SELECT 'it''s', $$body$$, $tag$content$tag$ /* comment */",
            agrees: parser_tokens_match_public_tokens,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "parser tokens match public tokens for a line break",
            sql: "SELECT cafe, 'naive' FROM t\nWHERE value >= 1.25e-2",
            agrees: parser_tokens_match_public_tokens,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "parser tokens match public tokens for non-ASCII text",
            sql: "SELECT cafe, 'caf\u{e9}', \u{3b4}elta FROM donn\u{e9}es",
            agrees: parser_tokens_match_public_tokens,
            expected_agreement: true,
        },
        TokenPathTestCase {
            description: "unchanged ASCII parser tokens borrow the source for plain text",
            sql: "SELECT alpha, 123, 'plain'",
            agrees: parser_tokens_borrow_source,
            expected_agreement: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            (test_case.agrees)(test_case.sql),
            test_case.expected_agreement,
            "{}",
            test_case.description
        );
    }
}
