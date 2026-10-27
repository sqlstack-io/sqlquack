use crate::census::tests::helpers::{
    CHAIN, CYCLE, MANY_PLACED, PLACED, QUOTED_LOWER_CASE, UNRESOLVED_COLUMN_AND_RELATION,
    blocker_summary, owned, placed_refusals,
};
use crate::census::tests::test_types::{BlockerSummary, BlockerTestCase, PlacedTestCase};

#[test]
fn given_chained_models_when_tracing_refusals_then_each_root_counts_the_files_it_blocks() {
    let test_cases = [
        BlockerTestCase {
            description: "two refused roots, their downstream models, and an unknown relation",
            fixtures: CHAIN,
            names: true,
            expected_summary: BlockerSummary {
                roots: vec![
                    ("fn:JAROWINKLER_SIMILARITY".to_owned(), [1, 3, 4, 3]),
                    ("fn:INITCAP".to_owned(), [1, 1, 2, 1]),
                    (
                        "relation:__SQLQUACK_RELATION__SHOP__RETURNS".to_owned(),
                        [1, 0, 1, 1],
                    ),
                ],
                single_root_files: 5,
                files: vec![
                    ("a".to_owned(), owned(&[]), 0),
                    ("b".to_owned(), owned(&["a"]), 0),
                    ("c".to_owned(), owned(&["a"]), 0),
                    ("d".to_owned(), owned(&["c", "e"]), 0),
                    ("e".to_owned(), owned(&[]), 0),
                    ("f".to_owned(), owned(&[]), 1),
                ],
            },
        },
        BlockerTestCase {
            description: "without names, files are numbered and relation roots folded",
            fixtures: CHAIN,
            names: false,
            expected_summary: BlockerSummary {
                roots: vec![
                    ("fn:JAROWINKLER_SIMILARITY".to_owned(), [1, 3, 4, 3]),
                    ("fn:INITCAP".to_owned(), [1, 1, 2, 1]),
                    ("unknown relation".to_owned(), [1, 0, 1, 1]),
                ],
                single_root_files: 5,
                files: vec![
                    ("#1".to_owned(), owned(&[]), 0),
                    ("#2".to_owned(), owned(&["#1"]), 0),
                    ("#3".to_owned(), owned(&["#1"]), 0),
                    ("#4".to_owned(), owned(&["#3", "#5"]), 0),
                    ("#5".to_owned(), owned(&[]), 0),
                    ("#6".to_owned(), owned(&[]), 0),
                ],
            },
        },
        BlockerTestCase {
            description: "an unresolved column keeps its root next to an unknown relation's",
            fixtures: UNRESOLVED_COLUMN_AND_RELATION,
            names: true,
            expected_summary: BlockerSummary {
                roots: vec![
                    (
                        "relation:__SQLQUACK_RELATION__SHOP__RETURNS".to_owned(),
                        [1, 0, 1, 0],
                    ),
                    ("statement".to_owned(), [1, 0, 1, 0]),
                ],
                single_root_files: 0,
                files: vec![("a".to_owned(), owned(&[]), 1)],
            },
        },
        BlockerTestCase {
            description: "a quoted lower-case name finds its model regardless of case",
            fixtures: QUOTED_LOWER_CASE,
            names: true,
            expected_summary: BlockerSummary {
                roots: vec![("fn:INITCAP".to_owned(), [1, 1, 2, 2])],
                single_root_files: 2,
                files: vec![
                    ("a".to_owned(), owned(&[]), 0),
                    ("b".to_owned(), owned(&["a"]), 0),
                ],
            },
        },
        BlockerTestCase {
            description: "models refused only by each other are a cycle root that readers inherit",
            fixtures: CYCLE,
            names: true,
            expected_summary: BlockerSummary {
                roots: vec![("cycle".to_owned(), [2, 1, 3, 3])],
                single_root_files: 3,
                files: vec![
                    ("a".to_owned(), owned(&["b"]), 0),
                    ("b".to_owned(), owned(&["a"]), 0),
                    ("c".to_owned(), owned(&["a"]), 0),
                ],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            blocker_summary(test_case.fixtures, test_case.names),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_refused_files_when_reporting_then_each_refusal_has_its_line_and_column() {
    let test_cases = [
        PlacedTestCase {
            description: "a refusal after a reference call is placed in the file as written",
            fixtures: PLACED,
            names: true,
            expected_placed: (
                vec![(
                    "a".to_owned(),
                    vec!["2:42 missing rule: fn:INITCAP".to_owned()],
                )],
                vec!["  a  2:42  missing rule: fn:INITCAP".to_owned()],
            ),
        },
        PlacedTestCase {
            description: "without names, files are numbered and keep their positions",
            fixtures: PLACED,
            names: false,
            expected_placed: (
                vec![(
                    "#1".to_owned(),
                    vec!["2:42 missing rule: fn:INITCAP".to_owned()],
                )],
                vec!["  #1  2:42  missing rule: fn:INITCAP".to_owned()],
            ),
        },
        PlacedTestCase {
            description: "the text report places a file's first refusals and counts the rest",
            fixtures: MANY_PLACED,
            names: true,
            expected_placed: (
                vec![(
                    "a".to_owned(),
                    vec![
                        "1:8 missing rule: fn:INITCAP".to_owned(),
                        "2:8 missing rule: fn:INITCAP".to_owned(),
                        "3:8 missing rule: fn:INITCAP".to_owned(),
                        "4:8 missing rule: fn:INITCAP".to_owned(),
                        "5:8 missing rule: fn:INITCAP".to_owned(),
                        "6:8 missing rule: fn:INITCAP".to_owned(),
                        "7:8 missing rule: fn:INITCAP".to_owned(),
                    ],
                )],
                vec![
                    "  a  1:8  missing rule: fn:INITCAP".to_owned(),
                    "  a  2:8  missing rule: fn:INITCAP".to_owned(),
                    "  a  3:8  missing rule: fn:INITCAP".to_owned(),
                    "  a  4:8  missing rule: fn:INITCAP".to_owned(),
                    "  a  5:8  missing rule: fn:INITCAP".to_owned(),
                    "  a  +2 more".to_owned(),
                ],
            ),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            placed_refusals(test_case.fixtures, test_case.names),
            test_case.expected_placed,
            "{}",
            test_case.description
        );
    }
}
