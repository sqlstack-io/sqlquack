//! DDL clauses with no shared field: keyed `ALTER TABLE` actions, view options and documented drops.

use serde_json::Value;
use sqlquack_syntax::Parsed;

use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ValueTestCase;

const KEY: &str = "/0/alter_table/actions/0/SetProperty/properties/0/0";
const VALUE: &str = "/0/alter_table/actions/0/SetProperty/properties/0/1";

#[test]
fn given_a_ddl_clause_with_no_shared_field_when_parsing_then_its_tree_keeps_it() {
    let test_cases = [
        ValueTestCase {
            description: "an owner change written in lower case with extra spaces",
            sql: "alter table t1 owner   to v1",
            pointer: KEY,
            expected_json: "\"OWNER TO\"",
        },
        ValueTestCase {
            description: "SET before OWNER is its own key",
            sql: "ALTER TABLE t1 SET OWNER TO v1",
            pointer: KEY,
            expected_json: "\"SET OWNER TO\"",
        },
        ValueTestCase {
            description: "a row filter keeps its function",
            sql: "ALTER TABLE t1 SET ROW FILTER f1 ON ( c1 , c2 )",
            pointer: "/0/alter_table/actions/0/SetProperty/properties/0/1/tuple/expressions/0/table/name/name",
            expected_json: "\"f1\"",
        },
        ValueTestCase {
            description: "a row filter keeps the columns passed to it",
            sql: "ALTER TABLE t1 SET ROW FILTER f1 ON ( c1 , c2 )",
            pointer: "/0/alter_table/actions/0/SetProperty/properties/0/1/tuple/expressions/1/tuple/expressions/1/column/name/name",
            expected_json: "\"c2\"",
        },
        ValueTestCase {
            description: "dropping a row filter holds no value",
            sql: "ALTER TABLE t1 DROP ROW FILTER",
            pointer: VALUE,
            expected_json: "{\"null\":null}",
        },
        ValueTestCase {
            description: "TRUNCATE UNIFORM HISTORY is the managed mode",
            sql: "ALTER TABLE t1 SET MANAGED TRUNCATE UNIFORM HISTORY",
            pointer: "/0/alter_table/actions/0/SetProperty/properties/0/1/var/this",
            expected_json: "\"TRUNCATE UNIFORM HISTORY\"",
        },
        ValueTestCase {
            description: "DROP FEATURE keeps TRUNCATE HISTORY",
            sql: "ALTER TABLE t1 DROP FEATURE f1 TRUNCATE HISTORY",
            pointer: KEY,
            expected_json: "\"DROP FEATURE\"",
        },
        ValueTestCase {
            description: "a column position keeps the column it follows",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 AFTER c2",
            pointer: "/0/alter_table/actions/0/SetProperty/properties/0/1/tuple/expressions/2/tuple/expressions/1/identifier/name",
            expected_json: "\"c2\"",
        },
        ValueTestCase {
            description: "a column mask is a masking policy",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 SET MASK m1 USING COLUMNS ( c2 )",
            pointer: "/0/alter_table/actions/0/SetProperty/properties/0/1/tuple/expressions/2/masking_policy_column_constraint/this/table/name/name",
            expected_json: "\"m1\"",
        },
        ValueTestCase {
            description: "an added column keeps its mask among its options",
            sql: "ALTER TABLE t1 ADD COLUMN c1 INT MASK m1",
            pointer: "/0/alter_table/actions/0/AddColumn/column/options/0/masking_policy_column_constraint/this/table/name/name",
            expected_json: "\"m1\"",
        },
        ValueTestCase {
            description: "each column of a comma list keeps its own action",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 FIRST , c2 SYNC IDENTITY",
            pointer: "/0/alter_table/actions/1/SetProperty/properties/0/0",
            expected_json: "\"COLUMN SYNC IDENTITY\"",
        },
        ValueTestCase {
            description: "a view keeps its schema binding mode",
            sql: "CREATE VIEW v1 WITH SCHEMA TYPE EVOLUTION AS SELECT 1",
            pointer: "/0/create_view/options/0/property/value/var/this",
            expected_json: "\"TYPE EVOLUTION\"",
        },
        ValueTestCase {
            description: "a view keeps the columns it is partitioned on",
            sql: "CREATE VIEW v1 PARTITIONED ON ( c1 , c2 ) AS SELECT 1 AS c1 , 2 AS c2",
            pointer: "/0/create_view/options/0/property/value/tuple/expressions/1/identifier/name",
            expected_json: "\"c2\"",
        },
        ValueTestCase {
            description: "a temporary view over a source keeps its provider",
            sql: "CREATE TEMPORARY VIEW v1 ( c1 INT ) USING csv OPTIONS ( 'k' = 'v' )",
            pointer: "/0/utility_statement/kind",
            expected_json: "\"CREATE TEMPORARY VIEW\"",
        },
        ValueTestCase {
            description: "dropping a temporary table is its own kind",
            sql: "DROP TEMP TABLE IF EXISTS t1",
            pointer: "/0/utility_statement/kind",
            expected_json: "\"DROP TEMP TABLE\"",
        },
        ValueTestCase {
            description: "UNDROP by identifier is its own kind",
            sql: "UNDROP MATERIALIZED VIEW WITH ID '0'",
            pointer: "/0/utility_statement/kind",
            expected_json: "\"UNDROP MATERIALIZED VIEW WITH ID\"",
        },
        ValueTestCase {
            description: "ALTER DATABASE keeps DATABASE as its kind",
            sql: "ALTER DATABASE s1 SET OWNER TO u1",
            pointer: "/0/alter_object/kind",
            expected_json: "\"DATABASE\"",
        },
        ValueTestCase {
            description: "a column's reference keeps FOREIGN KEY when written",
            sql: "CREATE TABLE t1 ( c1 INT CONSTRAINT n1 FOREIGN KEY REFERENCES t2 )",
            pointer: "/0/create_table/columns/0/constraints/0/References/has_foreign_key_keywords",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a grant on the unnamed catalog keeps CATALOG as its kind",
            sql: "GRANT SELECT ON CATALOG TO u1",
            pointer: "/0/grant/kind",
            expected_json: "\"CATALOG\"",
        },
        ValueTestCase {
            description: "a revoke on the metastore keeps METASTORE as its kind",
            sql: "REVOKE SELECT ON METASTORE FROM u1",
            pointer: "/0/revoke/kind",
            expected_json: "\"METASTORE\"",
        },
        ValueTestCase {
            description: "an auto CDC flow's star keeps the columns it excepts",
            sql: "CREATE OR REFRESH STREAMING TABLE st1 FLOW AUTO CDC FROM v1 KEYS ( c1 ) \
                  SEQUENCE BY c2 COLUMNS * EXCEPT ( c3 , c4 )",
            pointer: "/0/utility_statement/operands/4/property/value/star/except/1/name",
            expected_json: "\"c4\"",
        },
        ValueTestCase {
            description: "IGNORE NULL UPDATES is a clause of its own before SEQUENCE BY",
            sql: "CREATE OR REFRESH STREAMING TABLE st1 FLOW AUTO CDC FROM v1 KEYS ( c1 ) \
                  IGNORE NULL UPDATES SEQUENCE BY c2",
            pointer: "/0/utility_statement/operands/3/property/this/var/this",
            expected_json: "\"IGNORE NULL UPDATES\"",
        },
        ValueTestCase {
            description: "a snapshot flow's history star keeps the columns it excepts",
            sql: "CREATE OR REFRESH STREAMING TABLE st1 FLOW AUTO CDC FROM SNAPSHOT ( SELECT 1 ) \
                  KEYS ( c1 ) TRACK HISTORY ON * EXCEPT ( c3 )",
            pointer: "/0/utility_statement/operands/3/property/value/star/except/0/name",
            expected_json: "\"c3\"",
        },
        ValueTestCase {
            description: "a policy on the metastore keeps ON METASTORE apart from its row filter",
            sql: "CREATE POLICY p1 ON METASTORE ROW FILTER f1 TO `u1` FOR TABLES",
            pointer: "/0/utility_statement/operands/1/property/this/var/this",
            expected_json: "\"ON METASTORE\"",
        },
        ValueTestCase {
            description: "FOR TABLES is a clause of its own before a mask's column",
            sql: "CREATE POLICY p1 ON SCHEMA s1 COLUMN MASK m1 TO `u1` FOR TABLES ON COLUMN c1",
            pointer: "/0/utility_statement/operands/4/property/this/var/this",
            expected_json: "\"FOR TABLES\"",
        },
        ValueTestCase {
            description: "STORED AS BITEMPORAL ends before an optional TRACK HISTORY ON",
            sql: "CREATE OR REFRESH STREAMING TABLE st1 FLOW AUTO CDC FROM v1 KEYS ( c1 ) \
                  SEQUENCE BY c2 SYSTEM SEQUENCE BY c9 STORED AS BITEMPORAL TRACK HISTORY ON c3",
            pointer: "/0/utility_statement/operands/5/property/this/var/this",
            expected_json: "\"STORED AS BITEMPORAL\"",
        },
    ];
    for test_case in test_cases {
        let parsed: Parsed = parse_strict(test_case.sql).expect(test_case.description);
        assert!(parsed.gaps.is_empty(), "{}", test_case.description);
        let tree: Value = serde_json::to_value(&parsed.statements).expect(test_case.description);
        assert_eq!(
            tree.pointer(test_case.pointer)
                .map(Value::to_string)
                .as_deref(),
            Some(test_case.expected_json),
            "{}",
            test_case.description
        );
    }
}
