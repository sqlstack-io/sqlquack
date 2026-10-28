use crate::text::tests::helpers::split_rows;
use crate::text::tests::test_types::SplitRowsTestCase;

#[test]
fn given_texts_when_splitting_to_rows_then_every_part_is_numbered_from_one() {
    let test_cases = [
        SplitRowsTestCase {
            description: "empty parts at the ends and in the middle are kept",
            text: Some(",a,,b,"),
            delimiter: ",",
            expected_rows: &[(1, ""), (2, "a"), (3, ""), (4, "b"), (5, "")],
        },
        SplitRowsTestCase {
            description: "empty text is one empty part",
            text: Some(""),
            delimiter: ",",
            expected_rows: &[(1, "")],
        },
        SplitRowsTestCase {
            description: "NULL text gives no rows",
            text: None,
            delimiter: ",",
            expected_rows: &[],
        },
        SplitRowsTestCase {
            description: "a multi-character delimiter is matched literally",
            text: Some("a / b/c"),
            delimiter: " / ",
            expected_rows: &[(1, "a"), (2, "b/c")],
        },
    ];
    for test_case in test_cases {
        let rows = split_rows(test_case.text, test_case.delimiter);
        let actual: Vec<(i64, &str)> = rows
            .iter()
            .map(|(index, value)| (*index, value.as_str()))
            .collect();
        assert_eq!(
            actual.as_slice(),
            test_case.expected_rows,
            "{}",
            test_case.description
        );
    }
}
