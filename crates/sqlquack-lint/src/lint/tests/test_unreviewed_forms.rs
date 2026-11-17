//! Databricks forms read as trees whose layout is not reviewed yet stay refused as opaque.

use crate::lint::tests::helpers;
use crate::lint::tests::test_types;

/// The refusal lint gives a statement the native parser keeps as opaque.
const OPAQUE_REFUSAL: &str = "unsupported syntax: native parser retained an opaque SQL node";

#[test]
fn given_a_databricks_form_read_as_a_tree_but_not_reviewed_when_linting_then_it_is_refused_as_opaque()
 {
    let test_cases = [
        test_types::LintRefusalTestCase {
            description: "ADD of resources",
            sql: "ADD FILE f1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "LIST of resources",
            sql: "LIST JARS",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "RESTORE",
            sql: "RESTORE t1 VERSION AS OF 1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "DENY",
            sql: "DENY SELECT ON TABLE t1 TO u1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "CACHE SELECT as documented",
            sql: "CACHE SELECT c1 FROM t1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "REFRESH of a resource path",
            sql: "REFRESH 'p1'",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "DESCRIBE of a column",
            sql: "DESCRIBE TABLE t1 c1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "SHOW of a partition",
            sql: "SHOW PARTITIONS t1 PARTITION ( c1 = 1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a Hive row format",
            sql: "CREATE TABLE t1 ( c1 INT ) ROW FORMAT DELIMITED",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a Hive storage format",
            sql: "CREATE TABLE t1 ( c1 INT ) STORED AS PARQUET",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a constraint's validation words",
            sql: "ALTER TABLE t1 ADD CONSTRAINT k1 CHECK ( c1 > 0 ) ENABLE",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
    ];
    for test_case in test_cases {
        let refusal: Result<Vec<serde_json::Value>, String> =
            helpers::diagnostics_for_dialect(test_case.sql, test_case.dialect, &["SQLCLEAN001"]);
        assert_eq!(
            refusal.err().as_deref(),
            Some(test_case.expected_refusal),
            "{}",
            test_case.description
        );
    }
}
