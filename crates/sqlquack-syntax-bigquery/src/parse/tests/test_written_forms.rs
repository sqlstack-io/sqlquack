//! BigQuery literals written raw, as adjacent pieces or plain: each form keeps its own tree.

use crate::parse::main::parse::parse;
use crate::parse::tests::test_types::DistinctTreesTestCase;

#[test]
fn given_a_literal_written_in_different_ways_when_parsing_then_each_tree_keeps_what_was_written() {
    let test_cases = [
        DistinctTreesTestCase {
            description: "bytes as one piece, as adjacent pieces and raw",
            texts: &["SELECT b'ab'", "SELECT b'a' b'b'", "SELECT rb'ab'"],
            expected_distinct: 3,
        },
        DistinctTreesTestCase {
            description: "a DATE of a plain, a raw and an adjacent string",
            texts: &[
                "SELECT DATE '2020-01-01'",
                "SELECT DATE r'2020-01-01'",
                "SELECT DATE '2020-' '01-01'",
            ],
            expected_distinct: 3,
        },
        DistinctTreesTestCase {
            description: "a TIMESTAMP of a plain, a raw and an adjacent string",
            texts: &[
                "SELECT TIMESTAMP '2020-01-01 00:00:00'",
                "SELECT TIMESTAMP r'2020-01-01 00:00:00'",
                "SELECT TIMESTAMP '2020-01-01' ' 00:00:00'",
            ],
            expected_distinct: 3,
        },
        DistinctTreesTestCase {
            description: "a JSON and a NUMERIC of a plain, a raw and an adjacent string",
            texts: &[
                "SELECT JSON '{}'",
                "SELECT JSON r'{}'",
                "SELECT JSON '{' '}'",
                "SELECT NUMERIC '1'",
                "SELECT NUMERIC r'1'",
                "SELECT NUMERIC '1' '0'",
            ],
            expected_distinct: 6,
        },
        DistinctTreesTestCase {
            description: "a grantee as a plain, a raw and an adjacent string",
            texts: &[
                "GRANT `roles/viewer` ON TABLE d.t TO 'user:a@example.com'",
                "GRANT `roles/viewer` ON TABLE d.t TO r'user:a@example.com'",
                "GRANT `roles/viewer` ON TABLE d.t TO 'user:a@' 'example.com'",
            ],
            expected_distinct: 3,
        },
    ];
    for test_case in test_cases {
        let trees: Vec<_> = test_case
            .texts
            .iter()
            .map(|text| parse(text).map(|parsed| parsed.statements))
            .collect();
        assert!(
            trees.iter().all(Result::is_ok),
            "{}: every form parses",
            test_case.description
        );
        let distinct: usize = trees
            .iter()
            .enumerate()
            .filter(|(index, current)| !trees[..*index].contains(current))
            .count();
        assert_eq!(
            distinct, test_case.expected_distinct,
            "{}",
            test_case.description
        );
    }
}
