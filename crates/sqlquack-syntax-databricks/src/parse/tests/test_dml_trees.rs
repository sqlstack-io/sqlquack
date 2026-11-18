//! DML clauses kept in shared fields: Hive's directory of an `INSERT OVERWRITE` and `COPY INTO`'s columns.

use serde_json::Value;
use sqlquack_syntax::Parsed;

use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ValueTestCase;

const DIRECTORY: &str = "INSERT OVERWRITE LOCAL DIRECTORY '/tmp/x' ROW FORMAT DELIMITED \
     FIELDS TERMINATED BY ',' NULL DEFINED AS 'N' STORED AS ORC SELECT 1";

#[test]
fn given_a_dml_clause_with_a_shared_field_when_parsing_then_its_tree_keeps_it() {
    let test_cases = [
        ValueTestCase {
            description: "a directory keeps its path",
            sql: DIRECTORY,
            pointer: "/0/insert/directory/path",
            expected_json: "\"/tmp/x\"",
        },
        ValueTestCase {
            description: "a directory keeps LOCAL",
            sql: DIRECTORY,
            pointer: "/0/insert/directory/local",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a directory keeps its field delimiter",
            sql: DIRECTORY,
            pointer: "/0/insert/directory/row_format/fields_terminated_by",
            expected_json: "\",\"",
        },
        ValueTestCase {
            description: "a directory keeps its null marker",
            sql: DIRECTORY,
            pointer: "/0/insert/directory/row_format/null_defined_as",
            expected_json: "\"N\"",
        },
        ValueTestCase {
            description: "a directory keeps its storage format",
            sql: DIRECTORY,
            pointer: "/0/insert/directory/stored_as",
            expected_json: "\"ORC\"",
        },
        ValueTestCase {
            description: "a directory keeps the query written to it",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' SELECT c1 FROM t1",
            pointer: "/0/insert/query/select/from/expressions/0/table/name/name",
            expected_json: "\"t1\"",
        },
        ValueTestCase {
            description: "COPY INTO keeps its target's columns",
            sql: "COPY INTO t1 ( c1 , c2 ) FROM '/tmp/x' FILEFORMAT = CSV",
            pointer: "/0/copy/columns/1/name",
            expected_json: "\"c2\"",
        },
        ValueTestCase {
            description: "COPY INTO keeps its target with its columns",
            sql: "COPY INTO t1 ( c1 ) FROM '/tmp/x' FILEFORMAT = CSV",
            pointer: "/0/copy/this/table/name/name",
            expected_json: "\"t1\"",
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
