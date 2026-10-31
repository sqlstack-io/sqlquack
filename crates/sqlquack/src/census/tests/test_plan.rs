use crate::census::tests::helpers::{CHAIN, owned, plan_summary};
use crate::census::tests::test_types::{PlanSummary, PlanTestCase};

#[test]
fn given_assumed_constructs_when_planning_then_upper_bounds_follow_the_model_graph() {
    let test_cases = [
        PlanTestCase {
            description: "nothing assumed: the bounds are the census's counts, and the plan \
                          supports the two roots that together clear the chain",
            assumed: &[],
            expected_summary: PlanSummary {
                convertible: [1, 1],
                upper_bounds: [1, 1],
                next: vec![
                    ("fn:SOUNDEX".to_owned(), 1, 1),
                    ("fn:STRTOK".to_owned(), 1, 1),
                    (
                        "relation:__SQLQUACK_RELATION__SHOP__RETURNS".to_owned(),
                        1,
                        1,
                    ),
                ],
                steps: vec![(owned(&["fn:SOUNDEX", "fn:STRTOK"]), [6, 6])],
            },
        },
        PlanTestCase {
            description: "one root assumed: its model and the readers typed by it convert, a \
                          reader also needing the other root does not",
            assumed: &["fn:STRTOK"],
            expected_summary: PlanSummary {
                convertible: [1, 1],
                upper_bounds: [4, 4],
                next: vec![
                    ("fn:SOUNDEX".to_owned(), 1, 1),
                    (
                        "relation:__SQLQUACK_RELATION__SHOP__RETURNS".to_owned(),
                        1,
                        1,
                    ),
                ],
                steps: vec![(owned(&["fn:SOUNDEX"]), [6, 6])],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            plan_summary(CHAIN, test_case.assumed),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}
