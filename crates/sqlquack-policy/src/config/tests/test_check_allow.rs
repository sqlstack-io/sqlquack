use std::collections::BTreeMap;

use crate::config::main::check_allow::check_allow;
use crate::config::tests::test_types::CheckAllowTestCase;
use crate::policy::main::evidence_policy::evidence_policy;

#[test]
fn given_a_callers_allowances_when_checking_them_then_errors_name_where_they_are_written() {
    let test_cases = [
        CheckAllowTestCase {
            description: "known differences, allowed and refused",
            allow: &[("hash", true), ("random", false)],
            expected_error: None,
        },
        CheckAllowTestCase {
            description: "a misspelt known difference names the one it is near, and where it is",
            allow: &[("hsah", true)],
            expected_error: Some(
                "[testing.local] allow in project.toml names no known difference `hsah`; did you \
                 mean `hash`?",
            ),
        },
        CheckAllowTestCase {
            description: "Python functions take the caller's runtime fix",
            allow: &[("project_udf", true)],
            expected_error: Some(
                "[testing.local] allow in project.toml cannot allow `project_udf`: register them",
            ),
        },
    ];
    for test_case in test_cases {
        let allow: BTreeMap<String, bool> = test_case
            .allow
            .iter()
            .map(|(name, allowed)| ((*name).to_owned(), *allowed))
            .collect();

        let error = check_allow(
            &allow,
            evidence_policy(),
            ("[testing.local] allow in project.toml", "register them"),
        )
        .err();

        assert_eq!(
            error.as_deref(),
            test_case.expected_error,
            "{}",
            test_case.description
        );
    }
}
