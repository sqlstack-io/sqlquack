//! The exact check on every corpus committed with SQLQuack: zero differences, lossless tokens.

use std::collections::BTreeMap;

use sqlquack_polyglot_check::corpora::main::case_corpus::case_corpus;
use sqlquack_polyglot_check::corpora::main::load_corpus::load_corpus;

use crate::helpers::{exact_outcome, repository};
use crate::test_types::{CorpusTestCase, ExactOutcome};

#[test]
fn given_committed_corpora_when_parsed_by_both_parsers_then_results_are_identical() {
    let root = repository();
    let corpora = root.join("corpora/polyglot-check");
    let test_cases = [
        CorpusTestCase {
            description: "the SQL inputs of polyglot's tests",
            entries: load_corpus(&corpora.join("polyglot_tests.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::from([
                    ("malformed_typed_type_text".to_owned(), 1),
                    ("snowflake_bare_values_cte".to_owned(), 1),
                    ("snowflake_cte_without_as_prose".to_owned(), 1),
                    ("snowflake_reserved_type_name".to_owned(), 1),
                    ("snowflake_by_name_on_except".to_owned(), 1),
                    ("snowflake_by_name_on_intersect".to_owned(), 1),
                    ("snowflake_empty_select_list".to_owned(), 1),
                    ("snowflake_empty_select_list_before_from".to_owned(), 1),
                    (
                        "snowflake_empty_select_list_keyword_qualifier".to_owned(),
                        1,
                    ),
                    ("snowflake_empty_select_list_lower_case".to_owned(), 1),
                    ("snowflake_empty_select_list_stage_text".to_owned(), 1),
                    (
                        "snowflake_empty_select_list_trailing_order_by".to_owned(),
                        1,
                    ),
                    ("snowflake_empty_select_list_trailing_space".to_owned(), 1),
                    ("snowflake_reserved_distinct_alias".to_owned(), 1),
                    ("snowflake_reserved_is_alias".to_owned(), 1),
                    ("snowflake_set_operation_join_side".to_owned(), 1),
                ]),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqlglot's fixtures",
            entries: load_corpus(&corpora.join("sqlglot.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::from([
                    ("snowflake_cte_without_as".to_owned(), 1),
                    ("snowflake_empty_call_argument".to_owned(), 1),
                    ("snowflake_reserved_cte_name".to_owned(), 1),
                    ("snowflake_reserved_truncate_target".to_owned(), 1),
                    (
                        "snowflake_empty_select_list_trailing_order_by".to_owned(),
                        1,
                    ),
                    ("snowflake_reserved_window_alias".to_owned(), 1),
                ]),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqlglot's PostgreSQL tests",
            entries: load_corpus(&corpora.join("sqlglot_postgres.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "PostgreSQL's regression statements",
            entries: load_corpus(&corpora.join("postgres_regress.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqlglot's SQLite tests",
            entries: load_corpus(&corpora.join("sqlglot_sqlite.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "sqllogictest's statements",
            entries: load_corpus(&corpora.join("sqllogictest.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "every Snowflake type spelling in each cast form",
            entries: load_corpus(&corpora.join("snowflake_types.json")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
        CorpusTestCase {
            description: "SQLQuack's cases, source and converted",
            entries: case_corpus(&root.join("cases"), &root.join("sqlquack.toml")),
            expected_outcome: ExactOutcome {
                loaded: true,
                rejected: BTreeMap::new(),
                differences: Vec::new(),
                tokens_lost: Vec::new(),
                type_names: Vec::new(),
                unlisted: Vec::new(),
                unexplained: Vec::new(),
                located: Vec::new(),
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            exact_outcome(&test_case.entries),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
