use crate::parser::tests::helpers::child_paths;
use crate::parser::tests::helpers::generic_statement;
use crate::parser::tests::test_types::ChildPathTestCase;
use sqlquack_syntax::traversal::models::ChildPathSegment;

#[test]
fn given_a_statement_when_visiting_children_then_paths_follow_fields_and_indices() {
    let test_cases = [ChildPathTestCase {
        description: "projections are indexed and the FROM table nests under its field",
        sql: "SELECT a, b FROM t",
        expected_paths: vec![
            vec![
                ChildPathSegment::Field("expressions"),
                ChildPathSegment::Index(0),
            ],
            vec![
                ChildPathSegment::Field("expressions"),
                ChildPathSegment::Index(1),
            ],
            vec![
                ChildPathSegment::Field("from"),
                ChildPathSegment::Field("expressions"),
                ChildPathSegment::Index(0),
            ],
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            child_paths(&generic_statement(test_case.sql)),
            test_case.expected_paths,
            "{}",
            test_case.description
        );
    }
}
