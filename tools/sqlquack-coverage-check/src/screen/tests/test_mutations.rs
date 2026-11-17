use crate::screen::_helpers::mutations::mutations;
use crate::screen::models::MutationKind;
use crate::screen::models::MutationKind::{
    Backquote, DoubleQuote, Drop, InsertAs, InsertEquals, Requote, String, Swap, Unquote,
};
use crate::screen::tests::helpers::{databricks_tokens, no_tokens};
use crate::screen::tests::test_types::{GluedTokenTestCase, MutationsTestCase};

#[test]
fn given_a_text_when_listing_its_mutations_then_each_token_is_dropped_and_respelled_once() {
    let test_cases = [
        MutationsTestCase {
            description: "a bare name is dropped, quoted three ways and preceded by AS and =; a symbol only dropped",
            sql: "RESET v1 ;",
            expected_mutations: vec![
                (Drop, "RESET", "v1 ;"),
                (Backquote, "RESET", "`RESET` v1 ;"),
                (String, "RESET", "'RESET' v1 ;"),
                (DoubleQuote, "RESET", "\"RESET\" v1 ;"),
                (InsertAs, "RESET", "AS RESET v1 ;"),
                (InsertEquals, "RESET", "= RESET v1 ;"),
                (Drop, "v1", "RESET ;"),
                (Backquote, "v1", "RESET `v1` ;"),
                (String, "v1", "RESET 'v1' ;"),
                (DoubleQuote, "v1", "RESET \"v1\" ;"),
                (InsertAs, "v1", "RESET AS v1 ;"),
                (InsertEquals, "v1", "RESET = v1 ;"),
                (Drop, ";", "RESET v1"),
                (InsertAs, ";", "RESET v1 AS ;"),
                (InsertEquals, ";", "RESET v1 = ;"),
                (InsertAs, "", "RESET v1 ; AS"),
                (InsertEquals, "", "RESET v1 ; ="),
            ],
        },
        MutationsTestCase {
            description: "TEMP and TEMPORARY trade places; quoted names are unquoted; a string is requoted",
            sql: "TEMP `k` 'v'",
            expected_mutations: vec![
                (Drop, "TEMP", "`k` 'v'"),
                (Swap, "TEMP", "TEMPORARY `k` 'v'"),
                (Backquote, "TEMP", "`TEMP` `k` 'v'"),
                (String, "TEMP", "'TEMP' `k` 'v'"),
                (DoubleQuote, "TEMP", "\"TEMP\" `k` 'v'"),
                (InsertAs, "TEMP", "AS TEMP `k` 'v'"),
                (InsertEquals, "TEMP", "= TEMP `k` 'v'"),
                (Drop, "`k`", "TEMP 'v'"),
                (Unquote, "`k`", "TEMP k 'v'"),
                (String, "`k`", "TEMP 'k' 'v'"),
                (DoubleQuote, "`k`", "TEMP \"k\" 'v'"),
                (InsertAs, "`k`", "TEMP AS `k` 'v'"),
                (InsertEquals, "`k`", "TEMP = `k` 'v'"),
                (Drop, "'v'", "TEMP `k`"),
                (Unquote, "'v'", "TEMP `k` v"),
                (Backquote, "'v'", "TEMP `k` `v`"),
                (Requote, "'v'", "TEMP `k` \"v\""),
                (InsertAs, "'v'", "TEMP `k` AS 'v'"),
                (InsertEquals, "'v'", "TEMP `k` = 'v'"),
                (InsertAs, "", "TEMP `k` 'v' AS"),
                (InsertEquals, "", "TEMP `k` 'v' ="),
            ],
        },
        MutationsTestCase {
            description: "a keyword with two synonyms is swapped for each",
            sql: "DEC",
            expected_mutations: vec![
                (Swap, "DEC", "DECIMAL"),
                (Swap, "DEC", "NUMERIC"),
                (Backquote, "DEC", "`DEC`"),
                (String, "DEC", "'DEC'"),
                (DoubleQuote, "DEC", "\"DEC\""),
                (InsertAs, "DEC", "AS DEC"),
                (InsertEquals, "DEC", "= DEC"),
                (InsertAs, "", "DEC AS"),
                (InsertEquals, "", "DEC ="),
            ],
        },
        MutationsTestCase {
            description: "a variant equal to the text or to an earlier variant is written once",
            sql: "a a",
            expected_mutations: vec![
                (Drop, "a", "a"),
                (Backquote, "a", "`a` a"),
                (String, "a", "'a' a"),
                (DoubleQuote, "a", "\"a\" a"),
                (InsertAs, "a", "AS a a"),
                (InsertEquals, "a", "= a a"),
                (Backquote, "a", "a `a`"),
                (String, "a", "a 'a'"),
                (DoubleQuote, "a", "a \"a\""),
                (InsertAs, "a", "a AS a"),
                (InsertEquals, "a", "a = a"),
                (InsertAs, "", "a a AS"),
                (InsertEquals, "", "a a ="),
            ],
        },
    ];
    for test_case in test_cases {
        let written = mutations(test_case.sql, &[]);
        let found: Vec<(MutationKind, &str, &str)> = written
            .iter()
            .map(|mutation| {
                (
                    mutation.kind,
                    mutation.token.as_str(),
                    mutation.sql.as_str(),
                )
            })
            .collect();
        assert_eq!(
            found, test_case.expected_mutations,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_glued_token_when_listing_its_mutations_then_the_lexer_changes_it_alone() {
    let test_cases = [
        GluedTokenTestCase {
            description: "a STRUCT field's colon glued to its type is dropped alone",
            sql: "SELECT CAST ( NULL AS STRUCT < c1 :STRING > )",
            variant: "SELECT CAST ( NULL AS STRUCT < c1 STRING > )",
            lexed: databricks_tokens,
            expected_written: true,
        },
        GluedTokenTestCase {
            description: "the space splitter alone never drops the glued colon",
            sql: "SELECT CAST ( NULL AS STRUCT < c1 :STRING > )",
            variant: "SELECT CAST ( NULL AS STRUCT < c1 STRING > )",
            lexed: no_tokens,
            expected_written: false,
        },
        GluedTokenTestCase {
            description: "a token glued on both sides is dropped with its neighbours kept apart",
            sql: "SELECT a1:b1",
            variant: "SELECT a1 b1",
            lexed: databricks_tokens,
            expected_written: true,
        },
        GluedTokenTestCase {
            description: "a token glued on its left only is dropped with the space after it kept",
            sql: "SELECT f(x) AS y",
            variant: "SELECT f(x AS y",
            lexed: databricks_tokens,
            expected_written: true,
        },
        GluedTokenTestCase {
            description: "dropping a left-glued token never glues its neighbours together",
            sql: "SELECT f(x) AS y",
            variant: "SELECT f(xAS y",
            lexed: databricks_tokens,
            expected_written: false,
        },
        GluedTokenTestCase {
            description: "text inside a comment is still changed by the space splitter",
            sql: "SELECT /* m */ 1",
            variant: "SELECT /* */ 1",
            lexed: databricks_tokens,
            expected_written: true,
        },
    ];
    for test_case in test_cases {
        let lexed: Vec<(usize, usize)> = (test_case.lexed)(test_case.sql);
        let written: bool = mutations(test_case.sql, &lexed)
            .iter()
            .any(|mutation| mutation.sql == test_case.variant);
        assert_eq!(
            written, test_case.expected_written,
            "{}",
            test_case.description
        );
    }
}
