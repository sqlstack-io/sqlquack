use std::collections::BTreeSet;

use crate::statements::_helpers::rejections::keep_specific_rejections;
use crate::statements::models::Outcome;
use crate::statements::tests::helpers::statement;
use crate::statements::tests::test_types::RejectionTestCase;

const REJECTION: &str = "translation/functions/f.toml#t009_invalid";

#[test]
fn given_a_matching_rejection_when_its_refusal_is_not_specific_then_it_stays_refused() {
    let test_cases = [
        RejectionTestCase {
            description: "a family refused wholesale under SQ0002, such as ATAN2, stays refused",
            rejection: ("SQ0002", "fn:ATAN2"),
            sibling: (
                "translation/functions/f.toml#t001_valid",
                Outcome::Refused,
                "SQ0002",
                "fn:ATAN2",
            ),
            declared: &[],
            expected_outcome: Outcome::Refused,
        },
        RejectionTestCase {
            description: "an SQ0002 refusal with no valid sibling, as in the HLL-over files, stays refused",
            rejection: ("SQ0002", "fn:HLL_ACCUMULATE"),
            sibling: (
                "translation/functions/f.toml#t002_invalid",
                Outcome::Refused,
                "SQ0002",
                "fn:HLL_ACCUMULATE",
            ),
            declared: &["translation/functions/f.toml#t002_invalid"],
            expected_outcome: Outcome::Refused,
        },
        RejectionTestCase {
            description: "a specific refusal whose valid sibling translates is a matching rejection",
            rejection: ("SQ0003", "node:in"),
            sibling: (
                "translation/functions/f.toml#t001_valid",
                Outcome::Matching,
                "",
                "",
            ),
            declared: &[],
            expected_outcome: Outcome::MatchingRejection,
        },
        RejectionTestCase {
            description: "a valid sibling refused on the same key leaves it refused",
            rejection: ("SQ0003", "fn:AVG"),
            sibling: (
                "translation/functions/f.toml#t001_valid",
                Outcome::Refused,
                "SQ0003",
                "fn:AVG",
            ),
            declared: &[],
            expected_outcome: Outcome::Refused,
        },
        RejectionTestCase {
            description: "a valid case refused on the same key in another case file does not count",
            rejection: ("SQ0003", "node:in"),
            sibling: (
                "relational/other.toml#valid",
                Outcome::Refused,
                "SQ0003",
                "node:in",
            ),
            declared: &[],
            expected_outcome: Outcome::MatchingRejection,
        },
        RejectionTestCase {
            description: "a sibling refused on the same key that declares compile_error is not valid input",
            rejection: ("SQ0003", "node:in"),
            sibling: (
                "translation/functions/f.toml#t002_invalid",
                Outcome::Refused,
                "SQ0003",
                "node:in",
            ),
            declared: &["translation/functions/f.toml#t002_invalid"],
            expected_outcome: Outcome::MatchingRejection,
        },
    ];
    for test_case in test_cases {
        let (code, key) = test_case.rejection;
        let (reference, outcome, sibling_code, sibling_key) = test_case.sibling;
        let statements = vec![
            statement(REJECTION, Outcome::MatchingRejection, code, key),
            statement(reference, outcome, sibling_code, sibling_key),
        ];
        let declared: BTreeSet<&str> = test_case
            .declared
            .iter()
            .copied()
            .chain([REJECTION])
            .collect();

        let statements = keep_specific_rejections(statements, &declared);

        assert_eq!(
            statements[0].outcome, test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
