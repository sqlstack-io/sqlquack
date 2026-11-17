use crate::screen::_helpers::mutations::mutations;
use crate::screen::models::MutationKind;
use crate::screen::models::MutationKind::{Backquote, Drop, String, Swap, Unquote};
use crate::screen::tests::test_types::MutationsTestCase;

#[test]
fn given_a_text_when_listing_its_mutations_then_each_token_is_dropped_and_respelled_once() {
    let test_cases = [
        MutationsTestCase {
            description: "a bare name is dropped, backquoted and quoted; a symbol only dropped",
            sql: "RESET v1 ;",
            expected_mutations: vec![
                (Drop, "RESET", "v1 ;"),
                (Backquote, "RESET", "`RESET` v1 ;"),
                (String, "RESET", "'RESET' v1 ;"),
                (Drop, "v1", "RESET ;"),
                (Backquote, "v1", "RESET `v1` ;"),
                (String, "v1", "RESET 'v1' ;"),
                (Drop, ";", "RESET v1"),
            ],
        },
        MutationsTestCase {
            description: "TEMP and TEMPORARY trade places; quoted names are unquoted",
            sql: "TEMP `k` 'v'",
            expected_mutations: vec![
                (Drop, "TEMP", "`k` 'v'"),
                (Swap, "TEMP", "TEMPORARY `k` 'v'"),
                (Backquote, "TEMP", "`TEMP` `k` 'v'"),
                (String, "TEMP", "'TEMP' `k` 'v'"),
                (Drop, "`k`", "TEMP 'v'"),
                (Unquote, "`k`", "TEMP k 'v'"),
                (String, "`k`", "TEMP 'k' 'v'"),
                (Drop, "'v'", "TEMP `k`"),
                (Unquote, "'v'", "TEMP `k` v"),
                (Backquote, "'v'", "TEMP `k` `v`"),
            ],
        },
        MutationsTestCase {
            description: "a variant equal to the text or to an earlier variant is written once",
            sql: "a a",
            expected_mutations: vec![
                (Drop, "a", "a"),
                (Backquote, "a", "`a` a"),
                (String, "a", "'a' a"),
                (Backquote, "a", "a `a`"),
                (String, "a", "a 'a'"),
            ],
        },
    ];
    for test_case in test_cases {
        let written = mutations(test_case.sql);
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
