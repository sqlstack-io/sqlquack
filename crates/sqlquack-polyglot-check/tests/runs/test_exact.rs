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
                    ("malformed_typed_type_text".to_owned(), 2),
                    ("snowflake_bare_values_cte".to_owned(), 1),
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
