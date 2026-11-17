//! Each form of `TREED_FORMS` keeps the verdicts the parser gave while it was a gap, inside a compound included.

use crate::parse::_helpers::tree::reading::{Reading, parse_as};
use crate::parse::constants::TREED_FORMS;
use crate::parse::tests::helpers::verdict;
use crate::parse::tests::test_types::{ExpectedVerdict, TreedFormTestCase};

/// A statement accepted only after a gap earlier in its statement: refused on its own.
const RELAXED: &str = "ALTER TABLE t1 ALTER COLUMN c1";

#[test]
fn given_each_treed_form_before_a_relaxed_statement_when_parsing_then_the_verdict_is_the_gap_verdict()
 {
    let test_cases = [
        TreedFormTestCase {
            description: "add",
            construct: "add",
            sql: "ADD JAR",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "list",
            construct: "list",
            sql: "LIST JAR",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "restore",
            construct: "restore",
            sql: "RESTORE t1 VERSION AS OF 0",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "deny",
            construct: "deny",
            sql: "DENY SELECT ON TABLE t1 TO v1",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "cache_select",
            construct: "cache_select",
            sql: "CACHE SELECT c1 FROM t1",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "refresh",
            construct: "refresh",
            sql: "REFRESH t1",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "describe_column",
            construct: "describe_column",
            sql: "DESC t1 c1",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "show_partition",
            construct: "show_partition",
            sql: "SHOW PARTITIONS t1 PARTITION ( c1 = 1 )",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "row_format",
            construct: "row_format",
            sql: "CREATE TABLE t3 ROW FORMAT DELIMITED",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "stored_as",
            construct: "stored_as",
            sql: "CREATE TABLE t3 STORED AS TEXTFILE",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "constraint_validation",
            construct: "constraint_validation",
            sql: "ALTER TABLE t1 ADD PRIMARY KEY ( c1 ) ENABLE",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "column_mask",
            construct: "column_mask",
            sql: "CREATE TABLE t3 ( c1 INT MASK v1 )",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "row_filter",
            construct: "row_filter",
            sql: "ALTER TABLE t1 DROP ROW FILTER",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "owner",
            construct: "owner",
            sql: "ALTER TABLE t1 OWNER TO `u1`",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "managed",
            construct: "managed",
            sql: "ALTER TABLE t1 SET MANAGED",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "serde",
            construct: "serde",
            sql: "ALTER TABLE t1 SET SERDE 'org.example.Serde'",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "table_collation",
            construct: "table_collation",
            sql: "ALTER TABLE t1 DEFAULT COLLATION UTF8_BINARY",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "recover_partitions",
            construct: "recover_partitions",
            sql: "ALTER TABLE t1 RECOVER PARTITIONS",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "replace_partitioning",
            construct: "replace_partitioning",
            sql: "ALTER TABLE t1 REPLACE PARTITIONED BY WITH CLUSTER BY",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "predictive_optimization",
            construct: "predictive_optimization",
            sql: "ALTER TABLE t1 ENABLE PREDICTIVE OPTIMIZATION",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "rename_partition",
            construct: "rename_partition",
            sql: "ALTER TABLE t1 PARTITION ( c1 = 1 ) RENAME TO PARTITION ( c1 = 2 )",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "drop_primary_key",
            construct: "drop_primary_key",
            sql: "ALTER TABLE t1 DROP PRIMARY KEY",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "drop_foreign_key",
            construct: "drop_foreign_key",
            sql: "ALTER TABLE t1 DROP FOREIGN KEY ( c1 )",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "drop_feature",
            construct: "drop_feature",
            sql: "ALTER TABLE t1 DROP FEATURE 'v1'",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "alter_column_position",
            construct: "alter_column_position",
            sql: "ALTER TABLE t1 ALTER c1 FIRST",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "identity_sync",
            construct: "identity_sync",
            sql: "ALTER TABLE t1 ALTER c1 SYNC IDENTITY",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
        TreedFormTestCase {
            description: "column_tags",
            construct: "column_tags",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 UNSET TAGS ( 'k' )",
            expected_in_compound: ExpectedVerdict::Accepted { opaque: true },
            expected_as_script: ExpectedVerdict::Rejected { syntax: true },
        },
    ];
    let covered: Vec<&str> = test_cases
        .iter()
        .map(|test_case| test_case.construct)
        .collect();
    assert_eq!(
        covered,
        TREED_FORMS.to_vec(),
        "every form of TREED_FORMS has a case"
    );
    for test_case in test_cases {
        let reviewed: bool = parse_as(test_case.sql, Reading::Reviewed)
            .expect(test_case.description)
            .gaps
            .iter()
            .any(|gap| gap.construct == test_case.construct);
        assert!(reviewed, "{} is read as a tree", test_case.description);
        assert_eq!(
            verdict(&format!("BEGIN {} ; {RELAXED} ; END", test_case.sql)),
            test_case.expected_in_compound,
            "{} in a compound",
            test_case.description
        );
        assert_eq!(
            verdict(&format!("{} ; {RELAXED}", test_case.sql)),
            test_case.expected_as_script,
            "{} as a script",
            test_case.description
        );
    }
}
