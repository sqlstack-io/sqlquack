use serde_json::json;

use crate::helpers::statement_at;
use crate::test_types::ProjectionTestCase;

#[test]
fn given_a_stage_in_from_when_parsing_then_its_written_values_are_kept() {
    let test_cases = [
        ProjectionTestCase {
            description: "a PATTERN that is a name is kept",
            sql: "SELECT * FROM @s1 ( PATTERN => m1 ) ;",
            pointer: "/0/select/from/expressions/0/stage_reference/pattern/column/name/name",
            expected_value: json!("m1"),
        },
        ProjectionTestCase {
            description: "a string PATTERN is a string literal",
            sql: "SELECT * FROM @s1 ( FILE_FORMAT => f1 , PATTERN => '.*' ) ;",
            pointer: "/0/select/from/expressions/0/stage_reference/pattern/literal/value",
            expected_value: json!(".*"),
        },
        ProjectionTestCase {
            description: "a quoted stage's PATTERN is kept",
            sql: "SELECT * FROM '@s1' ( PATTERN => m1 ) ;",
            pointer: "/0/select/from/expressions/0/stage_reference/pattern/column/name/name",
            expected_value: json!("m1"),
        },
        ProjectionTestCase {
            description: "DIRECTORY's argument is a stage, not a column",
            sql: "SELECT 1 FROM DIRECTORY ( @\"SELECT\" ) ;",
            pointer: "/0/select/from/expressions/0/function/args/0/stage_reference/stage_name/parts/0/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "DIRECTORY ( @ ) names no stage",
            sql: "SELECT 1 FROM DIRECTORY ( @) ;",
            pointer: "/0/select/from/expressions/0/function/args/0/stage_reference/stage_name",
            expected_value: json!(null),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            statement_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_double_slash_after_a_stage_when_parsing_then_it_is_the_stage_path() {
    let test_cases = [
        ProjectionTestCase {
            description: "// after the COPY FILES target is its path",
            sql: "COPY FILES INTO @s1// FROM @s1 ;",
            pointer: "/0/copy/this/stage_reference/path",
            expected_value: json!("//"),
        },
        ProjectionTestCase {
            description: "the source after a target ending in // is kept",
            sql: "COPY FILES INTO @s1// FROM @s2 ;",
            pointer: "/0/copy/files/0/stage_reference/stage_name/parts/0/name",
            expected_value: json!("s2"),
        },
        ProjectionTestCase {
            description: "a query source after a target ending in // is kept",
            sql: "COPY FILES INTO @s1// FROM ( SELECT v1 FROM v1 ) ;",
            pointer: "/0/copy/files/0/subquery/this/select/from/expressions/0/table/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "// after the COPY FILES source is its path",
            sql: "COPY FILES INTO @s1 FROM @s1// ;",
            pointer: "/0/copy/files/0/stage_reference/path",
            expected_value: json!("//"),
        },
        ProjectionTestCase {
            description: "// after a space is still a comment",
            sql: "LIST @s1 // a comment\n;",
            pointer: "/0/stage_command/stage/stage_reference/path",
            expected_value: json!(null),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            statement_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
