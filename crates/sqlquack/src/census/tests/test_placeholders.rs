use crate::census::_helpers::placeholders::rewrite_with_references;
use crate::census::tests::test_types::RewriteTestCase;

#[test]
fn given_sqlbuild_compiled_sql_when_rewriting_then_references_become_parseable_sql() {
    let test_cases = [
        RewriteTestCase {
            description: "model, source and seed references become relations named after them",
            sql: "SELECT * FROM __ref(\"orders\") o JOIN __source(\"shop\", \"customers\") c \
                  ON TRUE JOIN __seed( \"regions\" ) r ON TRUE",
            expected_sql: "SELECT * FROM __sqlquack_relation__orders o JOIN \
                           __sqlquack_relation__shop__customers c \
                       ON TRUE JOIN __sqlquack_relation__regions r ON TRUE",
        },
        RewriteTestCase {
            description: "project functions are named after their reference and keep their arguments",
            sql: "SELECT __udf(\"is_gift\")(o.note) FROM t",
            expected_sql: "SELECT __sqlquack_udf__is_gift(o.note) FROM t",
        },
        RewriteTestCase {
            description: "table functions are named after their reference and keep their arguments",
            sql: "SELECT * FROM TABLE(__table_fn(\"open_orders\")(1))",
            expected_sql: "SELECT * FROM TABLE(__sqlquack_table_fn__open_orders(1))",
        },
        RewriteTestCase {
            description: "cursor bounds become a column",
            sql: "SELECT * FROM t WHERE updated_at >= __cursor_start()",
            expected_sql: "SELECT * FROM t WHERE updated_at >= __sqlquack_cursor",
        },
        RewriteTestCase {
            description: "references in strings and comments stay",
            sql: "SELECT '__ref(\"x\")' -- __ref(\"y\")\nFROM t /* __seed(\"z\") */",
            expected_sql: "SELECT '__ref(\"x\")' -- __ref(\"y\")\nFROM t /* __seed(\"z\") */",
        },
        RewriteTestCase {
            description: "other double-underscore names stay",
            sql: "SELECT __internal_flag, col__ref(1) FROM t",
            expected_sql: "SELECT __internal_flag, col__ref(1) FROM t",
        },
        RewriteTestCase {
            description: "a parenthesis inside a reference string does not end the call",
            sql: "SELECT * FROM __ref(\"odd)name\")",
            expected_sql: "SELECT * FROM __sqlquack_relation__odd_name",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            rewrite_with_references(test_case.sql).sql,
            test_case.expected_sql,
            "{}",
            test_case.description
        );
    }
}
