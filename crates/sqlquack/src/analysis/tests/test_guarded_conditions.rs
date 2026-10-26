use crate::analysis::tests::helpers::guards_gated;
use crate::analysis::tests::test_types::GuardedConditionTestCase;

#[test]
fn given_a_condition_with_a_guard_when_converting_then_the_guard_is_never_a_bare_conjunct() {
    let test_cases = [
        GuardedConditionTestCase {
            description: "a guarded WHERE conjunct is evaluated only where the others hold",
            sql: "SELECT note FROM customers WHERE customer_id = 1 AND TO_DATE(note) > '2024-01-01'",
            expected_gated: true,
        },
        GuardedConditionTestCase {
            description: "the guarded conjunct is gated wherever it is written",
            sql: "SELECT note FROM customers WHERE (TO_DATE(note) > '2024-01-01' AND note <> '') \
                  AND customer_id = 1",
            expected_gated: true,
        },
        GuardedConditionTestCase {
            description: "a guarded join condition is gated as well",
            sql: "SELECT o.amount FROM orders o JOIN customers c ON o.customer_id = c.customer_id \
                  AND TO_DATE(c.note) > '2024-01-01'",
            expected_gated: true,
        },
        GuardedConditionTestCase {
            description: "a lone guarded condition has nothing to wait for",
            sql: "SELECT note FROM customers WHERE TO_DATE(note) > '2024-01-01'",
            expected_gated: false,
        },
        GuardedConditionTestCase {
            description: "conditions without guards keep their shape",
            sql: "SELECT note FROM customers WHERE customer_id = 1 AND note <> ''",
            expected_gated: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            guards_gated(test_case.sql),
            test_case.expected_gated,
            "{}",
            test_case.description
        );
    }
}
