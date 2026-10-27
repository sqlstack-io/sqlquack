use crate::parser::tests::helpers::decisions_outside;
use crate::parser::tests::helpers::depth_exhaustion_inputs;
use crate::parser::tests::helpers::depth_scope_trace;
use crate::parser::tests::helpers::if_chains;
use crate::parser::tests::helpers::truncated_prefixes;
use crate::parser::tests::test_types::Decision;
use crate::parser::tests::test_types::DepthScopeTestCase;
use crate::parser::tests::test_types::TerminationTestCase;

#[test]
fn given_hostile_inputs_when_parsing_then_each_decides_within_the_budget() {
    let test_cases = [
        TerminationTestCase {
            description: "every truncated prefix of a valid statement parses or is rejected",
            inputs: truncated_prefixes(),
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
                Decision::Rejected {
                    by_depth_guard: true,
                },
            ],
        },
        TerminationTestCase {
            description: "an unclosed custom type argument list is rejected by every route",
            inputs: vec![
                "SELECT a.:S1(".to_owned(),
                "SELECT CAST(a AS S1(9".to_owned(),
                "CREATE TABLE t (c S1(".to_owned(),
                "ALTER TABLE t ADD COLUMN c S1(".to_owned(),
                "SELECT JSON_VALUE(a, '$.b' RETURNING S1(".to_owned(),
            ],
            expected_decisions: &[Decision::Rejected {
                by_depth_guard: false,
            }],
        },
        TerminationTestCase {
            description: "a malformed 24-link IF chain is rejected",
            inputs: if_chains(24, "I?{"),
            expected_decisions: &[Decision::Rejected {
                by_depth_guard: false,
            }],
        },
        TerminationTestCase {
            description: "exhausting the parser depth is rejected by the depth guard",
            inputs: depth_exhaustion_inputs(),
            expected_decisions: &[Decision::Rejected {
                by_depth_guard: true,
            }],
        },
        TerminationTestCase {
            description: "a parse after depth exhaustion still succeeds",
            inputs: vec!["SELECT 1".to_owned()],
            expected_decisions: &[Decision::Parsed],
        },
        TerminationTestCase {
            description: "accepted IF forms still parse",
            inputs: [
                "SELECT IF(a, 1, 2) FROM t",
                "SELECT IF(a > 1, 'x', 'y') AS c FROM t",
                "SELECT IF a THEN 1 ELSE 2 END FROM t",
                "SELECT IF(IF(a, 1, 2), 3, 4) FROM t",
                "SELECT IF FROM t",
                "SELECT t.if FROM t",
                "SELECT ~IF FROM t",
                "SELECT +IF FROM t",
                "SELECT -IF FROM t",
            ]
            .map(str::to_owned)
            .into_iter()
            .chain([format!("{}1", "IF~".repeat(24))])
            .collect(),
            expected_decisions: &[Decision::Parsed],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            decisions_outside(&test_case.inputs, test_case.expected_decisions),
            Vec::<(String, Decision)>::new(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_nested_parser_depth_scopes_when_entering_and_leaving_then_the_budget_is_shared() {
    let test_cases = [DepthScopeTestCase {
        description: "a child parser shares the budget and every scope gives its levels back",
        max_parser_depth: 3,
        expected_trace: (3, true, 0, true, true, true),
    }];

    for test_case in test_cases {
        assert_eq!(
            depth_scope_trace(test_case.max_parser_depth),
            test_case.expected_trace,
            "{}",
            test_case.description
        );
    }
}
