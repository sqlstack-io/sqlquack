use std::collections::BTreeSet;

use crate::formats::_helpers::retired::retire;
use crate::formats::tests::helpers::superseding_receipt as receipt;
use crate::formats::tests::test_types::RetireTestCase;

/// Line hashes are given, not computed: a real cycle needs each line to hold the other's hash.
#[test]
fn given_supersession_chains_when_retiring_then_only_a_linear_chain_passes() {
    let a: &str = &"a".repeat(64);
    let b: &str = &"b".repeat(64);
    let c: &str = &"c".repeat(64);
    let test_cases = [
        RetireTestCase {
            description: "a linear chain keeps only its last receipt",
            receipts: vec![
                (receipt(None), a.to_owned()),
                (receipt(Some(a)), b.to_owned()),
                (receipt(Some(b)), c.to_owned()),
            ],
            expected_kept: 1,
            expected_problem: None,
        },
        RetireTestCase {
            description: "a cycle of two",
            receipts: vec![
                (receipt(Some(b)), a.to_owned()),
                (receipt(Some(a)), b.to_owned()),
            ],
            expected_kept: 0,
            expected_problem: Some("is in a supersession cycle"),
        },
        RetireTestCase {
            description: "a broken chain naming a line that was never published",
            receipts: vec![(receipt(Some(c)), a.to_owned())],
            expected_kept: 1,
            expected_problem: Some("which is no published receipt of its case"),
        },
    ];
    for test_case in test_cases {
        let (kept, _, problems) = retire(test_case.receipts, &BTreeSet::new());
        assert_eq!(
            kept.len(),
            test_case.expected_kept,
            "{}",
            test_case.description
        );
        let problem_found: bool = test_case
            .expected_problem
            .map_or(problems.is_empty(), |problem| {
                problems.iter().any(|found| found.contains(problem))
            });
        assert!(problem_found, "{}: {problems:?}", test_case.description);
    }
}
