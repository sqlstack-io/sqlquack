use crate::snowflake::catalogue::main::builtin_catalogue::builtin_catalogue;
use crate::snowflake::conversion::models::ConversionOptions;
use sqlquack_diagnostics::refusals::models::RefusalCategory;
use sqlquack_semantics::schemas::models::{FunctionLanguage, FunctionTraits};
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;
use sqlquack_semantics::snowflake::typing::types::{
    ColumnSpec, FunctionReturns, FunctionSpec, Schema,
};

use std::collections::BTreeMap;

use crate::snowflake::analysis::main::analyze_function::analyze_function;
use crate::snowflake::analysis::main::convert_function::convert_function;
use crate::snowflake::analysis::models::ConversionChoices;
use crate::snowflake::analysis::tests::test_types::{
    AnalyzeFunctionTestCase, ConvertFunctionTestCase,
};

#[test]
fn given_a_sql_function_body_when_converted_without_the_engine_then_it_is_a_macro_body_or_refused()
{
    let amount = SnowflakeType::Number {
        precision: 10,
        scale: 2,
    };
    let spec = FunctionSpec {
        language: FunctionLanguage::Sql,
        arguments: vec![ColumnSpec::new("AMOUNT", amount)],
        returns: FunctionReturns::Scalar(amount),
        traits: FunctionTraits::default(),
    };
    let test_cases = [
        AnalyzeFunctionTestCase {
            description: "a NUMBER scalar body converts, its parameter cast to the declared type",
            body: "CAST(amount * 2 AS NUMBER(10,2))",
            expected_parameters: &["p1"],
            expected_body_has: Some("SELECT CAST(p1 AS DECIMAL(10,2)) AS \"AMOUNT\""),
            expected_refusals: &[],
        },
        AnalyzeFunctionTestCase {
            description: "a body naming no parameter or column is refused as unresolved",
            body: "missing + 1",
            expected_parameters: &[],
            expected_body_has: None,
            expected_refusals: &[RefusalCategory::UnresolvedName],
        },
    ];
    for test_case in test_cases {
        let outcome = analyze_function(
            (test_case.body, &spec),
            &builtin_catalogue(),
            (&Schema::default(), &ConversionOptions::default()),
        );
        let parameters = outcome
            .as_ref()
            .map(|converted| converted.parameters.clone())
            .unwrap_or_default();
        let body_has = test_case.expected_body_has.filter(|fragment| {
            outcome
                .as_ref()
                .is_ok_and(|converted| converted.body.contains(fragment))
        });
        let refusals: Vec<RefusalCategory> = outcome
            .err()
            .unwrap_or_default()
            .iter()
            .map(|refusal| refusal.category)
            .collect();
        assert_eq!(
            (parameters, body_has, refusals.as_slice()),
            (
                test_case
                    .expected_parameters
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                test_case.expected_body_has,
                test_case.expected_refusals
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_sql_function_body_when_converted_as_sqlquack_function_does_then_refusals_are_diagnostics_in_the_body()
 {
    let amount = SnowflakeType::Number {
        precision: 10,
        scale: 2,
    };
    let spec = FunctionSpec {
        language: FunctionLanguage::Sql,
        arguments: vec![ColumnSpec::new("AMOUNT", amount)],
        returns: FunctionReturns::Scalar(amount),
        traits: FunctionTraits::default(),
    };
    let test_cases = [
        ConvertFunctionTestCase {
            description: "a body that converts under the built-in policy is a macro body",
            body: "CAST(amount * 2 AS NUMBER(10,2))",
            parameters: &[],
            expected_outcome: Ok(&["p1"]),
        },
        ConvertFunctionTestCase {
            description: "an unresolved name is an SQ0004 diagnostic placed on its text in the body",
            body: "amount + missing",
            parameters: &[],
            expected_outcome: Err(&[("SQ0004", Some("missing"))]),
        },
        ConvertFunctionTestCase {
            description: "a session parameter the built-in policy does not know is an unplaced SQ0007",
            body: "CAST(amount * 2 AS NUMBER(10,2))",
            parameters: &[("NO_SUCH_PARAMETER", "1")],
            expected_outcome: Err(&[("SQ0007", None)]),
        },
    ];
    for test_case in test_cases {
        let parameters: BTreeMap<String, String> = test_case
            .parameters
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        let outcome = convert_function(
            (test_case.body, &spec),
            (&ConversionChoices::default(), &parameters),
            &Schema::default(),
        );
        let text = |start: usize, end: usize| -> String {
            test_case
                .body
                .chars()
                .skip(start)
                .take(end - start)
                .collect()
        };
        let actual: Result<Vec<String>, Vec<(&str, Option<String>)>> = outcome
            .map(|converted| converted.parameters)
            .map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| {
                        (
                            diagnostic.code,
                            diagnostic.span.map(|span| text(span.start, span.end)),
                        )
                    })
                    .collect()
            });
        assert_eq!(
            actual,
            test_case
                .expected_outcome
                .map(|names| names.iter().map(|name| (*name).to_owned()).collect())
                .map_err(|diagnostics| diagnostics
                    .iter()
                    .map(|(code, covered)| (*code, covered.map(str::to_owned)))
                    .collect()),
            "{}",
            test_case.description
        );
    }
}
