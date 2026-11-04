use crate::raises::main::identify_raises::identify_raises;
use crate::raises::main::raised::raised;
use crate::raises::tests::helpers::{guarded, read_back, same_ids};
use crate::raises::tests::test_types::{ManyRaisesTestCase, ReadBackTestCase, SameIdTestCase};

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

#[test]
fn given_an_expression_with_many_raises_when_identifying_them_then_time_grows_linearly() {
    let test_cases = [
        ManyRaisesTestCase {
            description: "20,000 raises of one tag in one expression",
            sql: vec![guarded("b"); 20_000].join(" + "),
            expected_ids: 1,
            expected_budget_seconds: 2,
        },
        ManyRaisesTestCase {
            description: "20,000 raises of two tags in one expression",
            sql: format!(
                "{} + {}",
                vec![guarded("b"); 10_000].join(" + "),
                vec![raised("numeric_range: x"); 10_000].join(" + ")
            ),
            expected_ids: 2,
            expected_budget_seconds: 2,
        },
    ];
    for test_case in test_cases {
        let started = std::time::Instant::now();
        let (_, ids) = identify_raises(&test_case.sql);
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(test_case.expected_budget_seconds),
            "{}: took {elapsed:?}",
            test_case.description
        );
        assert_eq!(
            ids.len(),
            test_case.expected_ids,
            "{}",
            test_case.description
        );
    }
}
