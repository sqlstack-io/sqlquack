//! `python_signature`: a Python function as SQLQuack's runtime registers it in DuckDB.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_blocks::blocks::main::column_type::column_type;
use sqlquack_semantics::types::models::{FunctionReturns, FunctionSpec};
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::analysis::constants::{SCALAR_ABI, VARIANT_ABI, VARIANT_TEXT_TYPE};
use crate::analysis::models::PythonSignature;

/// `name`'s DuckDB signature and ABI, or why it has none.
pub(crate) fn python_signature(name: &str, spec: &FunctionSpec) -> Result<PythonSignature, String> {
    let log = ProvenanceLog::default();
    let FunctionReturns::Scalar(returns) = spec.returns else {
        return Err(format!("function {name}: a Python table function"));
    };
    let (abi, returns) = match returns {
        SnowflakeType::SemiStructured => (VARIANT_ABI, VARIANT_TEXT_TYPE.to_owned()),
        other => (SCALAR_ABI, column_type(other, &log)?),
    };
    Ok(PythonSignature {
        name: name.to_owned(),
        abi: abi.to_owned(),
        arguments: spec
            .arguments
            .iter()
            .map(|argument| column_type(argument.data_type, &log))
            .collect::<Result<Vec<String>, String>>()?,
        returns,
        volatility: spec.traits.volatility.name().to_owned(),
        null_input: spec.traits.null_input.name().to_owned(),
    })
}
