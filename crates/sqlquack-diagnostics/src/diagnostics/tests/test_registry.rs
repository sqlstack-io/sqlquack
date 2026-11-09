use crate::diagnostics::tests::helpers::{registry_mismatches, unregistered_guard_names};
use crate::diagnostics::tests::test_types::{GuardNameTestCase, RegistryTestCase};

#[test]
fn given_the_registry_when_checked_against_a_lock_then_dropped_renamed_or_malformed_codes_show() {
    let test_cases = [
        RegistryTestCase {
            description: "the registry matches its append-only lock",
            lock: include_str!("../codes.lock"),
            whole_registry: true,
            expected_mismatches: Vec::new(),
        },
        RegistryTestCase {
            description: "a code issued once and missing from the registry is reported",
            lock: "SQ0001 parse_fidelity\nSQ9999 retired_code\n",
            whole_registry: false,
            expected_mismatches: vec!["SQ9999 retired_code".to_owned()],
        },
        RegistryTestCase {
            description: "a renamed code is reported under its issued name",
            lock: "SQ0001 parse_failure\n",
            whole_registry: false,
            expected_mismatches: vec!["SQ0001 parse_failure".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            registry_mismatches(test_case.lock, test_case.whole_registry),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_guard_names_in_the_syntax_crate_when_looked_up_then_each_has_a_registered_code() {
    let test_cases = [GuardNameTestCase {
        description: "every E_GUARD_* name the syntax crate raises maps to an SQ code",
        roots: &[
            "../sqlquack-syntax/src/guard",
            "../sqlquack-syntax-snowflake/src/parser",
        ],
        expected_unregistered: Vec::new(),
    }];
    for test_case in test_cases {
        assert_eq!(
            unregistered_guard_names(test_case.roots),
            test_case.expected_unregistered,
            "{}",
            test_case.description
        );
    }
}
