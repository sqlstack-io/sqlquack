use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_policy::policy::constants::EVIDENCE_FILE;
use sqlquack_policy::policy::main::load_policy::load_policy;

use crate::statements::_helpers::constructs::constructs;
use crate::statements::tests::test_types::{ConstructsTestCase, DatabricksConstructsTestCase};

/// Keys of a Databricks query and of the input fixture around it: clauses, a Databricks-only operator, the fixture's WITH and VALUES.
const PROBED: &[&str] = &[
    "clause:order_by",
    "clause:with",
    "fn:DIV",
    "node:select",
    "node:values",
];

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
    let policy = load_policy(&root.join(EVIDENCE_FILE)).expect("the repository policy loads");
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

#[test]
fn given_a_databricks_case_when_reading_constructs_then_the_databricks_parser_reads_it() {
    let test_cases = [
        DatabricksConstructsTestCase {
            description: "Databricks-only syntax keys its own query, not the VALUES fixture every case shares",
            query: "SELECT x1 DIV x1 AS r FROM input ORDER BY ordinal",
            probed: PROBED,
            expected_keys: &["clause:order_by", "fn:DIV", "node:select"],
        },
        DatabricksConstructsTestCase {
            description: "a statement the parser cannot read holds no tree construct, so it links to no form",
            query: "SELECT ( x1 AS r FROM input",
            probed: PROBED,
            expected_keys: &[],
        },
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let policy = load_policy(&root.join("crates/sqlquack-policy/policy-databricks.toml"))
        .expect("the Databricks policy loads");
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let toml = format!(
            "category = \"translation\"\nconstruct = \"db.functions.div\"\n\
             session_profile = \"dbx-default-v1\"\n\n[[case]]\nid = \"t001\"\ndescription = \"d\"\n\
             columns = [{{ name = \"x1\", type = \"INT\" }}]\nrows = [[\"7\"]]\n\
             query = \"{}\"\norder = \"ordered\"\ntags = [\"type-combination\"]\n",
            test_case.query
        );
        std::fs::write(directory.path().join("div.toml"), toml).expect("the case file writes");
        let cases = load_cases(directory.path(), &policy).expect("the case loads");

        let keys = constructs(&cases[0], &Default::default()).expect("constructs are read");

        let found: Vec<&str> = test_case
            .probed
            .iter()
            .copied()
            .filter(|key| keys.iter().any(|held| held == key))
            .collect();
        assert_eq!(found, test_case.expected_keys, "{}", test_case.description);
    }
}
