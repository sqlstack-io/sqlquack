use std::process::Output;

use crate::helpers::{approved_root, syntax_coverage};
use crate::test_types::ValidateTestCase;

#[test]
fn given_an_approved_form_when_validating_then_the_tree_screen_gates_it() {
    let test_cases = [
        ValidateTestCase {
            description: "control: a case with no same-tree variant passes",
            sql: "SELECT 1",
            allowlist: "",
            expected_success: true,
            expected_stderr_mention: "screening the 1 approved forms",
        },
        ValidateTestCase {
            description: "a comment's text dropped parses to the same tree and fails validate",
            sql: "SELECT /* m */ 1",
            allowlist: "",
            expected_success: false,
            expected_stderr_mention: "FAILED, the tree screen flags an approved form",
        },
        ValidateTestCase {
            description: "an entry for a form the catalogue does not hold fails validate",
            sql: "SELECT 1",
            allowlist: "[[allowed]]\nform = \"db.query.nowhere\"\nmutation = \"drop\"\n\
                        token = \";\"\nreason = \"r\"\n",
            expected_success: false,
            expected_stderr_mention: "db.query.nowhere Drop: the form is not catalogued",
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        approved_root(directory.path(), test_case.sql, test_case.allowlist);
        let root: &str = directory.path().to_str().expect("root path");
        let output: Output = syntax_coverage(&["validate", "--root", root]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.success(),
            test_case.expected_success,
            "{}: {stderr}",
            test_case.description
        );
        assert!(
            stderr.contains(test_case.expected_stderr_mention),
            "{}: {stderr}",
            test_case.description
        );
    }
}
