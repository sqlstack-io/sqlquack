use crate::text::tests::helpers::edit_distance;
use crate::text::tests::test_types::EditDistanceTestCase;

#[test]
fn given_two_texts_when_measuring_edit_distance_then_characters_are_counted_not_bytes() {
    let test_cases = [
        EditDistanceTestCase {
            description: "substitutions and an insertion",
            left: Some("kitten"),
            right: Some("sitting"),
            expected_distance: Some(3),
        },
        EditDistanceTestCase {
            description: "an accented letter is one substitution, not two bytes",
            left: Some("caf\u{e9}"),
            right: Some("cafe"),
            expected_distance: Some(1),
        },
        EditDistanceTestCase {
            description: "a character outside the BMP is one deletion",
            left: Some("\u{1f600}a"),
            right: Some("a"),
            expected_distance: Some(1),
        },
        EditDistanceTestCase {
            description: "a combining mark is a character of its own",
            left: Some("e\u{301}"),
            right: Some("\u{e9}"),
            expected_distance: Some(2),
        },
        EditDistanceTestCase {
            description: "two empty texts",
            left: Some(""),
            right: Some(""),
            expected_distance: Some(0),
        },
        EditDistanceTestCase {
            description: "a transposition is two edits",
            left: Some("ab"),
            right: Some("ba"),
            expected_distance: Some(2),
        },
        EditDistanceTestCase {
            description: "NULL on either side",
            left: None,
            right: Some("x"),
            expected_distance: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            edit_distance(test_case.left, test_case.right),
            test_case.expected_distance,
            "{}",
            test_case.description
        );
    }
}
