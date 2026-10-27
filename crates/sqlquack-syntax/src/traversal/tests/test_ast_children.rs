use crate::traversal::models::ChildPathSegment;
use crate::traversal::tests::helpers::child_paths;
use crate::traversal::tests::helpers::statement;
use crate::traversal::tests::helpers::tracked_and_untracked_children;
use crate::traversal::tests::test_types::ChildIdentityTestCase;
use crate::traversal::tests::test_types::ChildPathTestCase;

#[test]
fn given_a_statement_when_visiting_children_then_tracked_and_untracked_visits_agree() {
    let test_cases = [ChildIdentityTestCase {
        description: "both visitors see the same children in the same order",
        sql: "SELECT a, CASE WHEN b > 1 THEN c ELSE d END FROM t WHERE e IN (1, 2)",
        expected_child_count: 4,
    }];

    for test_case in test_cases {
        let expression = statement(test_case.sql);
        let (tracked, untracked) = tracked_and_untracked_children(&expression);
        assert_eq!(
            (
                tracked.len(),
                tracked == untracked,
                child_paths(&expression).len()
            ),
            (
                test_case.expected_child_count,
                true,
                test_case.expected_child_count
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_statement_when_visiting_children_then_paths_follow_fields_and_indices() {
    let test_cases = [ChildPathTestCase {
        description: "projections are indexed and the FROM table nests under its field",
        sql: "SELECT a, b FROM t",
        expected_paths: vec![
            vec![
                ChildPathSegment::Field("expressions"),
                ChildPathSegment::Index(0),
            ],
            vec![
                ChildPathSegment::Field("expressions"),
                ChildPathSegment::Index(1),
            ],
            vec![
                ChildPathSegment::Field("from"),
                ChildPathSegment::Field("expressions"),
                ChildPathSegment::Index(0),
            ],
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            child_paths(&statement(test_case.sql)),
            test_case.expected_paths,
            "{}",
            test_case.description
        );
    }
}
