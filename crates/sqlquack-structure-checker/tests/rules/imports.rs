use crate::helpers;
use crate::test_types;

const TEST_CASES: &[test_types::CheckRepoTestCase] = &[
    test_types::CheckRepoTestCase {
        description: "reports pub use outside lib rs",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/mod.rs",
            contents: "pub use models::AnalysisResult;\n",
        }],
        expected_violation_codes: &["SQS022"],
    },
    test_types::CheckRepoTestCase {
        description: "reports pub crate use reexport",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/mod.rs",
            contents: "pub(crate) use models::AnalysisResult;\n",
        }],
        expected_violation_codes: &["SQS023"],
    },
    test_types::CheckRepoTestCase {
        description: "reports cross module helper import",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/ast/models.rs",
            contents: "use crate::analysis::helpers;\npub struct Query;\n",
        }],
        expected_violation_codes: &["SQS005", "SQS011"],
    },
];

#[test]
fn given_import_rules_when_checking_structure_then_reports_expected_violation_codes() {
    for test_case in TEST_CASES {
        let repo_root = helpers::make_temp_repo(test_case);
        let actual_codes = helpers::collect_violation_codes(&repo_root);

        assert_eq!(
            actual_codes.as_slice(),
            test_case.expected_violation_codes,
            "{}",
            test_case.description
        );
    }
}
