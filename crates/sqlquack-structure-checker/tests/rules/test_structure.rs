use crate::helpers;
use crate::test_types;

const TEST_CASES: &[test_types::CheckRepoTestCase] = &[
    test_types::CheckRepoTestCase {
        description: "reports no violations for compliant test layout",
        repo_files: &[],
        expected_violation_codes: &[],
    },
    test_types::CheckRepoTestCase {
        description: "reports missing expected field assertion",
        repo_files: &[test_types::RepoFile {
            path: "crates/example/tests/rules/role_files.rs",
            contents: "use crate::helpers;\nuse crate::test_types;\nconst TEST_CASES: &[test_types::CheckRepoTestCase] = &[];\n#[test]\nfn given_input_when_running_then_reports_output() {\n    for test_case in TEST_CASES {\n        assert_eq!(1, 1, \"{}\", test_case.description);\n    }\n}\n",
        }],
        expected_violation_codes: &["SQT014"],
    },
    test_types::CheckRepoTestCase {
        description: "reports if in test body",
        repo_files: &[test_types::RepoFile {
            path: "crates/example/tests/rules/role_files.rs",
            contents: "use crate::helpers;\nuse crate::test_types;\nconst TEST_CASES: &[test_types::CheckRepoTestCase] = &[];\n#[test]\nfn given_input_when_running_then_reports_output() {\n    for test_case in TEST_CASES {\n        if true {\n            assert_eq!(1, test_case.expected_count, \"{}\", test_case.description);\n        }\n    }\n}\n",
        }],
        expected_violation_codes: &["SQT011"],
    },
    test_types::CheckRepoTestCase {
        description: "reports test case without expected field",
        repo_files: &[test_types::RepoFile {
            path: "crates/example/tests/rules/test_types.rs",
            contents: "pub(crate) struct CheckRepoTestCase {\n    pub(crate) description: &'static str,\n}\n",
        }],
        expected_violation_codes: &["SQT017"],
    },
    test_types::CheckRepoTestCase {
        description: "reports undeclared area test file",
        repo_files: &[test_types::RepoFile {
            path: "crates/example/tests/rules/forgotten.rs",
            contents: "use crate::test_types;\nconst TEST_CASES: &[test_types::CheckRepoTestCase] = &[];\n#[test]\nfn given_input_when_running_then_reports_output() {\n    for test_case in TEST_CASES {\n        assert_eq!(1, test_case.expected_count, \"{}\", test_case.description);\n    }\n}\n",
        }],
        expected_violation_codes: &["SQT020"],
    },
    test_types::CheckRepoTestCase {
        description: "reports harness module pointing to missing area file",
        repo_files: &[test_types::RepoFile {
            path: "crates/example/tests/rules.rs",
            contents: "#[path = \"rules/helpers.rs\"]\nmod helpers;\n#[path = \"rules/missing.rs\"]\nmod missing;\n#[path = \"rules/role_files.rs\"]\nmod role_files;\n#[path = \"rules/test_types.rs\"]\nmod test_types;\n",
        }],
        expected_violation_codes: &["SQT019"],
    },
];

#[test]
fn given_test_layout_rules_when_checking_structure_then_reports_expected_violation_codes() {
    for test_case in TEST_CASES {
        let repo_root = helpers::make_temp_repo(test_case);
        let actual_codes = helpers::collect_test_violation_codes(&repo_root);

        assert_eq!(
            actual_codes.as_slice(),
            test_case.expected_violation_codes,
            "{}",
            test_case.description
        );
    }
}
