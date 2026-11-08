//! Token boundaries and faults of the lexer, as SQL Server 2022 splits its input under PARSEONLY.

use sqlquack_syntax::TokenKind as LosslessKind;
use sqlquack_syntax::tokens::models::TokenType;

use crate::keywords::main::keyword_status::keyword_status;
use crate::scan::main::tokenize::tokenize;
use crate::scan::models::KeywordStatus;
use crate::scan::tests::helpers::{first_uncovered, quoted_values, significant_kinds, texts};
use crate::scan::tests::test_types::{
    BoundaryTestCase, CoverageTestCase, KeywordTestCase, QuotedTestCase, TokenTypeTestCase,
    UnclosedTestCase,
};

#[test]
fn given_sql_when_tokenizing_then_every_byte_is_in_exactly_one_token_in_order() {
    let test_cases = [
        CoverageTestCase {
            description: "delimited names, a variable, a compound assignment and a line comment",
            sql: "SELECT [a]]b] , \"c\"\"d\" FROM #t WHERE @x += N'it''s' -- tail",
            expected_uncovered: None,
        },
        CoverageTestCase {
            description: "a nested block comment, money, a hex literal and a dangling exponent",
            sql: "SELECT 1 /* a /* b */ c */ + $ 12.5 + \u{a3}3 + 0x1F + 1e+ ; GO",
            expected_uncovered: None,
        },
        CoverageTestCase {
            description: "Unicode blanks and zero-width characters between tokens",
            sql: "SELECT\u{a0}1\u{3000}\u{200b}x\u{feff}FROM\u{0b}t\r\n",
            expected_uncovered: None,
        },
        CoverageTestCase {
            description: "an empty input",
            sql: "",
            expected_uncovered: None,
        },
        CoverageTestCase {
            description: "blanks only",
            sql: "  \t\n",
            expected_uncovered: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            first_uncovered(test_case.sql),
            test_case.expected_uncovered,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_ambiguous_runs_when_tokenizing_then_boundaries_are_the_engines() {
    let test_cases = [
        BoundaryTestCase {
            description: "the engine reads `1x` as 1 aliased x",
            sql: "SELECT 1x",
            expected_texts: &["SELECT", "1", "x"],
        },
        BoundaryTestCase {
            description: "the engine reads `1e+` as one float",
            sql: "SELECT 1e+",
            expected_texts: &["SELECT", "1e+"],
        },
        BoundaryTestCase {
            description: "a number takes one exponent",
            sql: "SELECT 1E5E5",
            expected_texts: &["SELECT", "1E5", "E5"],
        },
        BoundaryTestCase {
            description: "a number takes one point, so `1.2.3` fails near `.3`",
            sql: "SELECT 1.2.3",
            expected_texts: &["SELECT", "1.2", ".3"],
        },
        BoundaryTestCase {
            description: "a hex literal may have no digits",
            sql: "SELECT 0xZ",
            expected_texts: &["SELECT", "0x", "Z"],
        },
        BoundaryTestCase {
            description: "money takes blanks before its digits",
            sql: "SELECT $ 5 x",
            expected_texts: &["SELECT", "$ 5", "x"],
        },
        BoundaryTestCase {
            description: "money takes no exponent",
            sql: "SELECT $1e5",
            expected_texts: &["SELECT", "$1", "e5"],
        },
        BoundaryTestCase {
            description: "money takes one sign, even with no digits after it",
            sql: "SELECT $-x, $ -.5",
            expected_texts: &["SELECT", "$-", "x", ",", "$ -.5"],
        },
        BoundaryTestCase {
            description: "money's sign ends before a second minus: `$--'` is `$-`, `-` and a string",
            sql: "SELECT $--'a'",
            expected_texts: &["SELECT", "$-", "-", "'a'"],
        },
        BoundaryTestCase {
            description: "`$` before a letter is a pseudo-column name; `@`, `#` and `$` continue a name",
            sql: "SELECT $action, @x$y, #t@u",
            expected_texts: &["SELECT", "$action", ",", "@x$y", ",", "#t@u"],
        },
        BoundaryTestCase {
            description: "adjacent national strings are two tokens",
            sql: "SELECT N'a'N'b'",
            expected_texts: &["SELECT", "N'a'", "N'b'"],
        },
        BoundaryTestCase {
            description: "`!=` and `!<` are one token each",
            sql: "SELECT a !=1, b!<2",
            expected_texts: &["SELECT", "a", "!=", "1", ",", "b", "!<", "2"],
        },
        BoundaryTestCase {
            description: "`--` runs to a carriage return",
            sql: "SELECT 1 --a\rFROM",
            expected_texts: &["SELECT", "1", "FROM"],
        },
        BoundaryTestCase {
            description: "`--` after a number starts a comment",
            sql: "SELECT 1--1",
            expected_texts: &["SELECT", "1"],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            texts(test_case.sql),
            test_case.expected_texts,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_tokens_when_tokenizing_then_their_shared_types_are_the_agreed_ones() {
    let test_cases = [TokenTypeTestCase {
        description: "names, variables, literals, inequalities and a compound assignment",
        sql: "SELECT [a], \"b\", c, @v, @@ROWCOUNT, #t, 'x', N'y', 0xFF, 1.5, $2 FROM t WHERE a <> 1 AND b != 2 AND c !< 3; SET @v += 1",
        expected_kinds: [
            TokenType::Var,
            TokenType::QuotedIdentifier,
            TokenType::Comma,
            TokenType::QuotedIdentifier,
            TokenType::Comma,
            TokenType::Var,
            TokenType::Comma,
            TokenType::Parameter,
            TokenType::Comma,
            TokenType::Parameter,
            TokenType::Comma,
            TokenType::Var,
            TokenType::Comma,
            TokenType::String,
            TokenType::Comma,
            TokenType::NationalString,
            TokenType::Comma,
            TokenType::HexString,
            TokenType::Comma,
            TokenType::Number,
            TokenType::Comma,
            TokenType::Number,
            TokenType::Var,
            TokenType::Var,
            TokenType::Var,
            TokenType::Var,
            TokenType::Neq,
            TokenType::Number,
            TokenType::Var,
            TokenType::Var,
            TokenType::Neq,
            TokenType::Number,
            TokenType::Var,
            TokenType::Var,
            TokenType::Operator,
            TokenType::Number,
            TokenType::Semicolon,
            TokenType::Var,
            TokenType::Parameter,
            TokenType::Operator,
            TokenType::Number,
        ]
        .map(LosslessKind::Significant)
        .to_vec(),
    }];
    for test_case in test_cases {
        assert_eq!(
            significant_kinds(test_case.sql),
            test_case.expected_kinds,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_quoted_runs_when_scanning_then_values_are_undoubled() {
    let test_cases = [QuotedTestCase {
        description: "a doubled closing character stands for one",
        sql: "SELECT [a]]b], \"c\"\"d\", 'e''f'",
        expected_values: &["a]b", "c\"d", "e'f"],
    }];
    for test_case in test_cases {
        assert_eq!(
            quoted_values(test_case.sql),
            test_case.expected_values,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unclosed_runs_when_tokenizing_then_the_engines_lexical_error_is_raised() {
    let test_cases = [
        UnclosedTestCase {
            description: "an unclosed string",
            sql: "SELECT 'a",
            expected_code: 105,
            expected_message: "Unclosed quotation mark after the character string 'a'.",
        },
        UnclosedTestCase {
            description: "an unclosed bracketed name",
            sql: "SELECT [a",
            expected_code: 105,
            expected_message: "Unclosed quotation mark after the character string 'a'.",
        },
        UnclosedTestCase {
            description: "an unclosed double-quoted name",
            sql: "SELECT \"a",
            expected_code: 105,
            expected_message: "Unclosed quotation mark after the character string 'a'.",
        },
        UnclosedTestCase {
            description: "`]]` is an escaped `]`, so the name never closes",
            sql: "SELECT 1 FROM [a]]",
            expected_code: 105,
            expected_message: "Unclosed quotation mark after the character string 'a]'.",
        },
        UnclosedTestCase {
            description: "an unclosed nested block comment",
            sql: "SELECT 1 /* a /* b */",
            expected_code: 113,
            expected_message: "Missing end comment mark '*/'.",
        },
    ];
    for test_case in test_cases {
        let fault = tokenize(test_case.sql).expect_err(test_case.description);
        assert_eq!(
            (fault.code, fault.message.as_str()),
            (test_case.expected_code, test_case.expected_message),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_words_when_classifying_then_reserved_unreserved_and_names_differ() {
    let test_cases = [
        KeywordTestCase {
            description: "a reserved word in lower case",
            word: "select",
            expected_status: Some(KeywordStatus::Reserved),
        },
        KeywordTestCase {
            description: "a reserved word in mixed case",
            word: "Pivot",
            expected_status: Some(KeywordStatus::Reserved),
        },
        KeywordTestCase {
            description: "a reserved function name",
            word: "TRY_CONVERT",
            expected_status: Some(KeywordStatus::Reserved),
        },
        KeywordTestCase {
            description: "a documented reserved word the engine accepts as a name",
            word: "PRECISION",
            expected_status: Some(KeywordStatus::Unreserved),
        },
        KeywordTestCase {
            description: "a window keyword",
            word: "rows",
            expected_status: Some(KeywordStatus::Unreserved),
        },
        KeywordTestCase {
            description: "a window frame keyword",
            word: "PRECEDING",
            expected_status: Some(KeywordStatus::Unreserved),
        },
        KeywordTestCase {
            description: "a plain name",
            word: "customer_id",
            expected_status: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            keyword_status(test_case.word),
            test_case.expected_status,
            "{}",
            test_case.description
        );
    }
}
