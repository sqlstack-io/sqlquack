use crate::census::_helpers::coverage::convertible_at;
use crate::census::tests::helpers::greedy;
use crate::census::tests::test_types::{ConvertibleTestCase, GreedyTestCase};

#[test]
fn given_rule_order_when_counting_then_files_convert_once_all_their_constructs_have_rules() {
    let test_cases = [
        ConvertibleTestCase {
            description: "files need every construct they use; unparsed files never convert",
            files: vec![Some(vec![0]), Some(vec![0, 1]), Some(vec![0, 2]), None],
            order: vec![0, 1, 2],
            points: vec![0, 1, 2, 3],
            expected_convertible: vec![0, 1, 2, 3],
        },
        ConvertibleTestCase {
            description: "a file using only late constructs waits for them",
            files: vec![Some(vec![2]), Some(vec![0, 1])],
            order: vec![0, 1, 2],
            points: vec![1, 2, 3],
            expected_convertible: vec![0, 1, 2],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            convertible_at(&test_case.files, &test_case.order, 3, &test_case.points),
            test_case.expected_convertible,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_construct_usage_when_ordering_greedily_then_rules_completing_most_files_come_first() {
    let test_cases = [GreedyTestCase {
        description: "a rare construct completing two files beats a common one completing none",
        files: vec![
            Some(vec![1]),
            Some(vec![1]),
            Some(vec![0, 2]),
            Some(vec![0, 3]),
            Some(vec![0, 4]),
        ],
        frequency_order: vec![0, 1, 2, 3, 4],
        expected_order: vec![1, 0, 2, 3, 4],
    }];

    for test_case in test_cases {
        assert_eq!(
            greedy(&test_case.files, &test_case.frequency_order),
            test_case.expected_order,
            "{}",
            test_case.description
        );
    }
}
