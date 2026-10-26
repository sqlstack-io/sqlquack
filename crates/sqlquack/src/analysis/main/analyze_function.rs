//! `analyze_function`: a SQL project function's body as a DuckDB macro body, or its refusals.

use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_core::refusals::types::ConstructPolicy;
use sqlquack_rules::conversion::main::convert_function_body::convert_function_body;
use sqlquack_rules::conversion::models::{
    ConversionOptions, ConvertedFunction, FunctionReturns, FunctionSpec, Schema,
};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::analysis::constants::SCALAR_RESULT_COLUMN;

/// Converts `body` as `spec` declares it; a NUMBER scalar's body must be of the declared type.
pub fn analyze_function(
    (body, spec): (&str, &FunctionSpec),
    dialect: SourceDialect,
    policy: &dyn ConstructPolicy,
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<ConvertedFunction, Vec<Refusal>> {
    let sql = match spec.returns {
        FunctionReturns::Scalar(SnowflakeType::Number { .. }) => {
            format!("SELECT ({body}) AS \"{SCALAR_RESULT_COLUMN}\"")
        }
        FunctionReturns::Scalar(data_type) => {
            format!("SELECT CAST(({body}) AS {data_type}) AS \"{SCALAR_RESULT_COLUMN}\"")
        }
        FunctionReturns::Table(_) => body.to_owned(),
    };
    let statements =
        parse_source(&sql, dialect).map_err(|failure| vec![refuse_unparsed(&failure)])?;
    let [statement] = statements.as_slice() else {
        return Err(vec![Refusal {
            construct: None,
            location: None,
            category: RefusalCategory::MissingRule,
            reason: format!("a function body of {} statements", statements.len()),
        }]);
    };
    let occurrences = classify(statement, &sql, dialect);
    let declared: Vec<Refusal> = refuse_constructs(&occurrences, policy)
        .into_iter()
        .filter(|refusal| refusal.category == RefusalCategory::NotExpressible)
        .collect();
    if !declared.is_empty() {
        return Err(declared);
    }
    convert_function_body(statement, spec, (schema, options), ProvenanceLog::default())
        .map_err(|refusal| vec![refusal])
}
