use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::main::load_policy::load_policy;

use crate::statements::_helpers::constructs::constructs;
use crate::statements::tests::test_types::ConstructsTestCase;

#[test]
fn given_a_statement_snowflake_cannot_parse_when_reading_constructs_then_only_a_declared_compile_error_passes()
 {
    let test_cases = [
        ConstructsTestCase {
            description: "a declared compile error keeps its token spellings",
            compile_error_line: "compile_error = true",
            expected_ok: true,
        },
        ConstructsTestCase {
            description: "a case that does not declare one still fails",
            compile_error_line: "",
            expected_ok: false,
        },
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let policy = load_policy(&root.join(POLICY_FILE_NAME)).expect("the repository policy loads");
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let toml = format!(
            "category = \"relational\"\nconstruct = \"node:select\"\nsession_profile = \"utc-v1\"\n\n\
             [[case]]\nid = \"datetime\"\n{}\ndescription = \"d\"\n\
             columns = [{{ name = \"k\", type = \"NUMBER(38,0)\" }}]\nrows = [[\"1\"]]\n\
             query = \"SELECT DATETIME '2026-01-02 03:04:05' AS v FROM input\"\norder = \"ordered\"\ntags = [\"null\", \"duplicates\", \"empty-input\"]\n",
            test_case.compile_error_line
        );
        std::fs::write(directory.path().join("typed.toml"), toml).expect("the case file writes");
        let cases = load_cases(directory.path(), &policy).expect("the case loads");

        let keys = constructs(&cases[0], &Default::default());

        assert_eq!(
            keys.is_ok(),
            test_case.expected_ok,
            "{}",
            test_case.description
        );
    }
}
