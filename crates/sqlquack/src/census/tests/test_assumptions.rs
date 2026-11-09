use crate::census::tests::helpers::{ASSUMING, assumption_counts};
use crate::census::tests::test_types::AssumptionTestCase;

#[test]
fn given_a_model_relying_on_an_assumption_when_counting_then_it_and_its_readers_drop_out() {
    let test_cases = [AssumptionTestCase {
        description: "the model reading a naive value as zoned relies on session_timezone_utc: it \
                      leaves both no-assumption counts, and its reader leaves the strict one",
        expected_counts: [3, 3, 2, 1],
        expected_relying: vec![("session_timezone_utc".to_owned(), 1)],
    }];
    for test_case in test_cases {
        let (counts, relying) = assumption_counts(ASSUMING);
        assert_eq!(
            (counts, relying.into_iter().collect::<Vec<_>>()),
            (test_case.expected_counts, test_case.expected_relying),
            "{}",
            test_case.description
        );
    }
}
