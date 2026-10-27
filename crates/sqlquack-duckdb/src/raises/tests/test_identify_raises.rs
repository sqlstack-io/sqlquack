use crate::raises::main::identify_raises::identify_raises;
use crate::raises::main::raised::raised;
use crate::raises::tests::helpers::{guarded, read_back, same_ids};
use crate::raises::tests::test_types::{ReadBackTestCase, SameIdTestCase};

#[test]
fn given_guarded_sql_when_identifying_raises_then_ids_follow_the_expression_content() {
    let test_cases = [
        SameIdTestCase {
            description: "the same expression gets the same ID",
            left: guarded("b"),
            right: guarded("b"),
            expected_same: true,
        },
        SameIdTestCase {
            description: "another expression gets another ID",
            left: guarded("b"),
            right: guarded("c"),
            expected_same: false,
        },
        SameIdTestCase {
            description: "another tag in the same expression gets another ID",
            left: guarded("b"),
            right: guarded("b").replace("sqlquack:100051", "sqlquack:100046"),
            expected_same: false,
        },
        SameIdTestCase {
            description: "an inner raise identified already does not change the outer ID",
            left: format!("COALESCE({}, {})", guarded("b"), raised("numeric_range: x")),
            right: format!(
                "COALESCE({}, {})",
                identify_raises(&guarded("b")).0,
                raised("numeric_range: x")
            ),
            expected_same: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            same_ids(&test_case.left, &test_case.right),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_identified_raise_when_duckdb_reports_it_then_its_id_reads_back() {
    let test_cases = [
        ReadBackTestCase {
            description: "a Snowflake error's ID reads back",
            sql: guarded("b"),
            expected_read: Some(0),
        },
        ReadBackTestCase {
            description: "a divergence guard's ID reads back",
            sql: raised("numeric_range: the exact quotient needs more than 128 bits"),
            expected_read: Some(0),
        },
        ReadBackTestCase {
            description: "a message without a tag gets no ID",
            sql: raised("Division by zero"),
            expected_read: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            read_back(&test_case.sql),
            test_case.expected_read,
            "{}",
            test_case.description
        );
    }
}
