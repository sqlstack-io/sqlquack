//! `analyze_function`: a SQL project function's body as a DuckDB macro body, or its refusals.

use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::{SourceDialect, SourceLines};
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_core::refusals::types::ConstructPolicy;
use sqlquack_rules::conversion::main::convert_function_body::convert_function_body;
use sqlquack_rules::conversion::main::quoted_exclusions::quoted_exclusions;
use sqlquack_rules::conversion::models::{ConversionOptions, ConvertedFunction};
use sqlquack_semantics::types::models::{FunctionReturns, FunctionSpec, Schema};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::analysis::_helpers::declared_refusals::declared_refusals;
use crate::analysis::constants::SCALAR_RESULT_COLUMN;
use crate::analysis::main::placed::placed;

/// Converts `body` as `spec` declares it; a NUMBER scalar's body must be of the declared type.
pub fn analyze_function(
    (body, spec): (&str, &FunctionSpec),
    dialect: SourceDialect,
    policy: &dyn ConstructPolicy,
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<ConvertedFunction, Vec<Refusal>> {
    let (prefix, suffix) = match spec.returns {
        FunctionReturns::Scalar(SnowflakeType::Number { .. }) => (
            "SELECT (".to_owned(),
            format!(") AS \"{SCALAR_RESULT_COLUMN}\""),
        ),
        FunctionReturns::Scalar(data_type) => (
            "SELECT CAST((".to_owned(),
            format!(") AS {data_type}) AS \"{SCALAR_RESULT_COLUMN}\""),
        ),
        FunctionReturns::Table(_) => (String::new(), String::new()),
    };
    let sql = format!("{prefix}{body}{suffix}");
    let options = &ConversionOptions {
        quoted_exclusions: quoted_exclusions(body),
        ..options.clone()
    };
    let lines = SourceLines::shifted(body, prefix.chars().count());
    let statements =
        parse_source(&sql, dialect).map_err(|failure| vec![refuse_unparsed(&failure, &lines)])?;
    let [statement] = statements.as_slice() else {
        return Err(vec![Refusal {
            construct: None,
            location: Some(lines.whole()),
            category: RefusalCategory::MissingRule,
            reason: format!("a function body of {} statements", statements.len()),
        }]);
    };
    let occurrences = classify(statement, &lines, dialect);
    let declared: Vec<Refusal> = declared_refusals(statement, &occurrences, policy, (body, &lines));
    if !declared.is_empty() {
        return Err(declared);
    }
    convert_function_body(
        statement,
        spec,
        (schema, options, &lines),
        ProvenanceLog::default(),
    )
    .map_err(|refusal| vec![placed(refusal, statement, &lines)])
}
