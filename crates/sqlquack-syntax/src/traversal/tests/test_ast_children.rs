use crate::traversal::tests::helpers::select_from_where;
use crate::traversal::tests::helpers::tracked_and_untracked_children;
use crate::traversal::tests::test_types::ChildIdentityTestCase;

#[test]
fn given_a_statement_when_visiting_children_then_tracked_and_untracked_visits_agree() {
    let test_cases = [ChildIdentityTestCase {
        description: "both visitors see the same children in the same order",
        expression: select_from_where(),
        expected_child_count: 4,
    }];

    for test_case in test_cases {
        let (tracked, untracked, paths) = tracked_and_untracked_children(&test_case.expression);
        assert_eq!(
            (tracked.len(), tracked == untracked, paths),
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
