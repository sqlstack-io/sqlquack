use crate::analysis::tests::helpers::case_rules;
use crate::analysis::tests::test_types::CaseProvenanceTestCase;

#[test]
fn given_a_case_when_converting_then_its_input_fixture_adds_no_rules() {
    let test_cases = [
        CaseProvenanceTestCase {
            description: "an expression case relies on its expression's rules only",
            reference: "numeric/addition.toml#integers",
            expected_rules: vec![
                "rule.alias".to_owned(),
                "rule.column".to_owned(),
                "rule.decimal_arithmetic".to_owned(),
                "rule.from_relation".to_owned(),
                "rule.select".to_owned(),
            ],
        },
        CaseProvenanceTestCase {
            description: "a query's own CTEs and ORDER BY are its rules",
            reference: "relational/common_tables.toml#chained_ctes",
            expected_rules: vec![
                "rule.alias".to_owned(),
                "rule.column".to_owned(),
                "rule.common_table".to_owned(),
                "rule.comparison".to_owned(),
                "rule.decimal_arithmetic".to_owned(),
                "rule.from_relation".to_owned(),
                "rule.number_literal".to_owned(),
                "rule.order_by".to_owned(),
                "rule.select".to_owned(),
                "rule.where".to_owned(),
            ],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            case_rules(test_case.reference),
            test_case.expected_rules,
            "{}",
            test_case.description
        );
    }
}
