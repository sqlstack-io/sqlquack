use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::ConversionOptions;
use sqlquack_semantics::types::models::{
    ColumnSpec, FunctionLanguage, FunctionReturns, FunctionSpec, FunctionTraits, Schema,
};
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::analysis::main::analyze_function::analyze_function;
use crate::analysis::tests::test_types::AnalyzeFunctionTestCase;

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
            SourceDialect::Snowflake,
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
