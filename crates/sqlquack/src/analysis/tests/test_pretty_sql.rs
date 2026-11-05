use crate::analysis::tests::helpers::pretty_layout_mismatches;
use crate::analysis::tests::test_types::PrettyLayoutTestCase;

#[test]
fn given_repository_cases_when_converted_sql_is_laid_out_then_tokens_and_parse_trees_are_kept() {
    let test_cases = [PrettyLayoutTestCase {
        description: "every distinct converted statement is laid out with the same tokens and DuckDB parse tree",
        expected_mismatches: Vec::new(),
    }];

    for test_case in test_cases {
        assert_eq!(
            pretty_layout_mismatches(),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
    }
}
