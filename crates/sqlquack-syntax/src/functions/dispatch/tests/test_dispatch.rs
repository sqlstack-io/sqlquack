use std::collections::HashSet;

use crate::functions::dispatch::constants::PARSER_DISPATCH_SPECS;
use crate::functions::dispatch::main::parser_dispatch_behavior_by_name_upper::parser_dispatch_behavior_by_name_upper;
use crate::functions::dispatch::models::ParserDispatchBehavior;
use crate::functions::dispatch::tests::test_types::DispatchLookupTestCase;
use crate::functions::dispatch::tests::test_types::DispatchSpecsTestCase;

#[test]
fn given_dispatch_specs_when_collecting_names_then_each_name_appears_once() {
    let test_cases = [DispatchSpecsTestCase {
        description: "parser dispatch names are unique",
        names: PARSER_DISPATCH_SPECS.iter().map(|spec| spec.name).collect(),
        expected_unique: PARSER_DISPATCH_SPECS.len(),
    }];

    for test_case in test_cases {
        assert_eq!(
            test_case.names.iter().collect::<HashSet<_>>().len(),
            test_case.expected_unique,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_function_name_when_looking_up_dispatch_then_its_behavior_is_returned() {
    let test_cases = [
        DispatchLookupTestCase {
            description: "TRY_TO_DATE dispatches by its registered behavior",
            name: "TRY_TO_DATE",
            expected_behavior: Some(ParserDispatchBehavior::ExprListFunction),
        },
        DispatchLookupTestCase {
            description: "GROUPING_ID dispatches by its registered behavior",
            name: "GROUPING_ID",
            expected_behavior: Some(ParserDispatchBehavior::OptionalExprListFunction),
        },
        DispatchLookupTestCase {
            description: "SEARCH dispatches by its registered behavior",
            name: "SEARCH",
            expected_behavior: Some(ParserDispatchBehavior::FunctionArgumentsFunction),
        },
        DispatchLookupTestCase {
            description: "CURRENT_ACCOUNT dispatches by its registered behavior",
            name: "CURRENT_ACCOUNT",
            expected_behavior: Some(ParserDispatchBehavior::ZeroArgFunction),
        },
        DispatchLookupTestCase {
            description: "HASH_AGG dispatches by its registered behavior",
            name: "HASH_AGG",
            expected_behavior: Some(ParserDispatchBehavior::HashLike),
        },
        DispatchLookupTestCase {
            description: "HLL dispatches by its registered behavior",
            name: "HLL",
            expected_behavior: Some(ParserDispatchBehavior::HllAggregate),
        },
        DispatchLookupTestCase {
            description: "PERCENTILE dispatches by its registered behavior",
            name: "PERCENTILE",
            expected_behavior: Some(ParserDispatchBehavior::PercentileAggregate),
        },
        DispatchLookupTestCase {
            description: "KURTOSIS dispatches by its registered behavior",
            name: "KURTOSIS",
            expected_behavior: Some(ParserDispatchBehavior::UnaryAggregate),
        },
        DispatchLookupTestCase {
            description: "TRANSLATE dispatches by its registered behavior",
            name: "TRANSLATE",
            expected_behavior: Some(ParserDispatchBehavior::TranslateNonTeradata),
        },
        DispatchLookupTestCase {
            description: "an unknown name has no dispatch behavior",
            name: "NOT_A_KNOWN_FN",
            expected_behavior: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            parser_dispatch_behavior_by_name_upper(test_case.name),
            test_case.expected_behavior,
            "{}",
            test_case.description
        );
    }
}
