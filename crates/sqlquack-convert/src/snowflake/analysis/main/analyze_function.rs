//! `analyze_function`: a SQL project function's body as a DuckDB macro body, or its refusals.

use crate::snowflake::conversion::main::_convert_function_body::convert_function_body;
use crate::snowflake::conversion::main::quoted_exclusions::quoted_exclusions;
use crate::snowflake::conversion::models::{ConversionOptions, ConvertedFunction};
use sqlquack_diagnostics::classification::main::classify::classify;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_diagnostics::refusals::types::ConstructPolicy;
use sqlquack_diagnostics::snowflake::classification::main::cast_type_name::cast_type_name;
use sqlquack_parse::parsing::main::parse_source::parse_source;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;
use sqlquack_semantics::snowflake::typing::types::{FunctionReturns, FunctionSpec, Schema};
use sqlquack_syntax::Dialect;
use sqlquack_syntax::tokens::models::SourceLines;

use crate::analysis::main::placed::placed;
use crate::snowflake::analysis::_helpers::declared_refusals::declared_refusals;
use crate::snowflake::analysis::constants::SCALAR_RESULT_COLUMN;

/// Converts `body` as `spec` declares it; a NUMBER scalar's body must be of the declared type.
pub fn analyze_function(
    (body, spec): (&str, &FunctionSpec),
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
    let statements = parse_source(&sql, Dialect::Snowflake)
        .map_err(|failure| vec![refuse_unparsed(&failure, &lines)])?;
    let [statement] = statements.as_slice() else {
        return Err(vec![Refusal {
            construct: None,
            location: Some(lines.whole()),
            category: RefusalCategory::MissingRule,
            reason: format!("a function body of {} statements", statements.len()),
        }]);
    };
    let occurrences = classify(statement, &lines, cast_type_name);
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
