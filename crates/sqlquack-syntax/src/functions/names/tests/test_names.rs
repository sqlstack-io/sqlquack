use crate::functions::names::_helpers::aggregates::is_aggregate_function_name_upper;
use crate::functions::names::constants::AGGREGATE_FUNCTION_NAME_LIST;
use crate::functions::names::constants::DUCKDB_AGGREGATE_FUNCTION_NAME_LIST;
use crate::functions::names::constants::NO_PAREN_FUNCTION_NAME_LIST;
use crate::functions::names::main::is_aggregate_function_name_for_dialect::is_aggregate_function_name_for_dialect;
use crate::functions::names::main::is_no_paren_function_name_upper::is_no_paren_function_name_upper;
use crate::functions::names::tests::helpers::duplicates;
use crate::functions::names::tests::helpers::not_aggregate;
use crate::functions::names::tests::helpers::not_no_paren;
use crate::functions::names::tests::helpers::shared;
use crate::functions::names::tests::test_types::NameListTestCase;
use crate::functions::names::tests::test_types::NameLookupTestCase;
use crate::tokens::models::DialectType;

#[test]
fn given_function_name_lists_when_checking_then_every_name_is_unique_and_described() {
    let test_cases = [
        NameListTestCase {
            description: "no-paren names are unique",
            names: NO_PAREN_FUNCTION_NAME_LIST,
            checked: duplicates,
            expected_failures: vec![],
        },
        NameListTestCase {
            description: "aggregate names are unique",
            names: AGGREGATE_FUNCTION_NAME_LIST,
            checked: duplicates,
            expected_failures: vec![],
        },
        NameListTestCase {
            description: "DuckDB aggregate names are unique",
            names: DUCKDB_AGGREGATE_FUNCTION_NAME_LIST,
            checked: duplicates,
            expected_failures: vec![],
        },
        NameListTestCase {
            description: "DuckDB aggregate names do not repeat shared ones",
            names: DUCKDB_AGGREGATE_FUNCTION_NAME_LIST,
            checked: |names| shared(names, AGGREGATE_FUNCTION_NAME_LIST),
            expected_failures: vec![],
        },
        NameListTestCase {
            description: "every no-paren name has no-paren metadata",
            names: NO_PAREN_FUNCTION_NAME_LIST,
            checked: not_no_paren,
            expected_failures: vec![],
        },
        NameListTestCase {
            description: "every aggregate name has aggregate metadata",
            names: AGGREGATE_FUNCTION_NAME_LIST,
            checked: not_aggregate,
            expected_failures: vec![],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            (test_case.checked)(test_case.names),
            test_case.expected_failures,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_function_name_when_looking_it_up_then_its_kind_is_known() {
    let no_paren = |name: &str, _: Option<DialectType>| is_no_paren_function_name_upper(name);
    let aggregate = |name: &str, _: Option<DialectType>| is_aggregate_function_name_upper(name);
    let test_cases = [
        NameLookupTestCase {
            description: "CURRENT_TIMESTAMP needs no parentheses",
            name: "CURRENT_TIMESTAMP",
            dialect: None,
            lookup: no_paren,
            expected_found: true,
        },
        NameLookupTestCase {
            description: "CURDATE needs no parentheses",
            name: "CURDATE",
            dialect: None,
            lookup: no_paren,
            expected_found: true,
        },
        NameLookupTestCase {
            description: "an unknown name needs parentheses",
            name: "FOO",
            dialect: None,
            lookup: no_paren,
            expected_found: false,
        },
        NameLookupTestCase {
            description: "COUNT is an aggregate",
            name: "COUNT",
            dialect: None,
            lookup: aggregate,
            expected_found: true,
        },
        NameLookupTestCase {
            description: "APPROX_PERCENTILE is an aggregate",
            name: "APPROX_PERCENTILE",
            dialect: None,
            lookup: aggregate,
            expected_found: true,
        },
        NameLookupTestCase {
            description: "COALESCE is not an aggregate",
            name: "COALESCE",
            dialect: None,
            lookup: aggregate,
            expected_found: false,
        },
        NameLookupTestCase {
            description: "arg_max_null is an aggregate in DuckDB",
            name: "arg_max_null",
            dialect: Some(DialectType::DuckDB),
            lookup: is_aggregate_function_name_for_dialect,
            expected_found: true,
        },
        NameLookupTestCase {
            description: "arg_max_null is not a generic aggregate",
            name: "arg_max_null",
            dialect: Some(DialectType::Generic),
            lookup: is_aggregate_function_name_for_dialect,
            expected_found: false,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            (test_case.lookup)(test_case.name, test_case.dialect),
            test_case.expected_found,
            "{}",
            test_case.description
        );
    }
}
