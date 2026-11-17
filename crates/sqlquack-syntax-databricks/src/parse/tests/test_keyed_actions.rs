//! `ALTER TABLE` actions with no shared variant, kept as `SetProperty` entries under fixed keys.

use serde_json::Value;
use sqlquack_syntax::Parsed;

use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ValueTestCase;

const KEY: &str = "/0/alter_table/actions/0/SetProperty/properties/0/0";
const VALUE: &str = "/0/alter_table/actions/0/SetProperty/properties/0/1";

#[test]
fn given_an_action_with_no_shared_variant_when_parsing_then_it_keeps_its_canonical_key() {
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
