use crate::helpers;
use crate::test_types;

const TEST_CASES: &[test_types::CheckRepoTestCase] = &[
    test_types::CheckRepoTestCase {
        description: "reports no violations for compliant source role files",
        repo_files: &[],
        expected_violation_codes: &[],
    },
    test_types::CheckRepoTestCase {
        description: "reports implementation in mod rs",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/mod.rs",
            contents: "pub(crate) fn analyze() {}\n",
        }],
        expected_violation_codes: &["SQS012"],
    },
    test_types::CheckRepoTestCase {
        description: "reports free function in models rs",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/models.rs",
            contents: "fn build_result() {}\n",
        }],
        expected_violation_codes: &["SQS013"],
    },
    test_types::CheckRepoTestCase {
        description: "reports public method in models rs",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/models.rs",
            contents: "pub struct AnalysisResult;\nimpl AnalysisResult { pub fn new() -> Self { Self } }\n",
        }],
        expected_violation_codes: &["SQS014"],
    },
    test_types::CheckRepoTestCase {
        description: "reports enum in types rs",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/types.rs",
            contents: "pub enum AnalysisKind { Basic }\n",
        }],
        expected_violation_codes: &["SQS015"],
    },
    test_types::CheckRepoTestCase {
        description: "reports function in constants rs",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/constants.rs",
            contents: "fn default_depth() -> usize { 0 }\n",
        }],
        expected_violation_codes: &["SQS016"],
    },
    test_types::CheckRepoTestCase {
        description: "reports missing visible main entry",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/main/analyze_query.rs",
            contents: "fn helper() {}\n",
        }],
        expected_violation_codes: &["SQS018"],
    },
    test_types::CheckRepoTestCase {
        description: "reports too many private main helpers",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/main/analyze_query.rs",
            contents: "pub(crate) fn analyze_query() {}\nfn one() {}\nfn two() {}\nfn three() {}\n",
        }],
        expected_violation_codes: &["SQS019"],
    },
    test_types::CheckRepoTestCase {
        description: "reports public helper api",
        repo_files: &[test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/helpers/scope_builder.rs",
            contents: "pub fn build_scope() {}\n",
        }],
        expected_violation_codes: &["SQS021"],
    },
];

#[test]
fn given_source_role_files_when_checking_structure_then_reports_expected_violation_codes() {
    for test_case in TEST_CASES {
        let repo_root = helpers::make_temp_repo(test_case);
        let actual_codes = helpers::collect_source_violation_codes(&repo_root);

        assert_eq!(
            actual_codes.as_slice(),
            test_case.expected_violation_codes,
            "{}",
            test_case.description
        );
    }
}
