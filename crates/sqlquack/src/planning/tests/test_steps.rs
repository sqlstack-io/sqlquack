use crate::planning::tests::helpers::steps_of;
use crate::planning::tests::test_types::StepsTestCase;

#[test]
fn given_refused_files_when_planning_then_steps_support_only_rule_resolvable_roots() {
    let test_cases = [StepsTestCase {
        description: "a not-expressible root is never a step; each step carries the unit-testable \
                      files that may be incomplete",
        files: &[
            (&[("not expressible in DuckDB", "fn:SOUNDEX")], false),
            (&[("missing rule", "fn:JAROWINKLER_SIMILARITY")], false),
            (&[("outside a rule's type conditions", "node:cast")], true),
        ],
        expected_steps: vec![
            (vec!["fn:JAROWINKLER_SIMILARITY".to_owned()], [1, 1, 0]),
            (vec!["node:cast".to_owned()], [2, 2, 1]),
        ],
    }];
    for test_case in test_cases {
        assert_eq!(
            steps_of(test_case.files),
            test_case.expected_steps,
            "{}",
            test_case.description
        );
    }
}
