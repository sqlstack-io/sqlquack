//! The schema file as written, and its columns and functions typed.

use std::collections::BTreeMap;

use serde::Deserialize;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::types::constants::DECLARABLE_OFFSETS;
use crate::types::models::{
    ColumnSpec, FunctionLanguage, FunctionReturns, FunctionSpec, FunctionTraits, NullInput,
    Volatility,
};

/// Columns and parameters as the schema file writes them.
pub(crate) type RawColumns = Vec<RawColumn>;

/// A name and Snowflake type pair, or an object that may also mark a VARIANT's typed leaves.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RawColumn {
    Pair(String, String),
    Described(RawDescribedColumn),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDescribedColumn {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(default)]
    typed_leaves: bool,
    /// The offset every value of a TIMESTAMP_TZ source column holds: `+00:00` or `UTC`.
    #[serde(default)]
    offset: Option<String>,
}

/// The schema file: relations and functions, or relations alone as the top-level map.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RawSchema {
    Sections(RawSections),
    Relations(BTreeMap<String, RawColumns>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSections {
    #[serde(default)]
    pub(crate) relations: BTreeMap<String, RawColumns>,
    #[serde(default)]
    pub(crate) functions: BTreeMap<String, RawFunction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawFunction {
    language: RawLanguage,
    #[serde(default)]
    arguments: RawColumns,
    returns: RawReturns,
    #[serde(default)]
    volatility: Volatility,
    #[serde(default)]
    null_input: NullInput,
    #[serde(default)]
    vectorized: bool,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RawLanguage {
    Sql,
    Python,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RawReturns {
    Scalar(String),
    Table { table: RawColumns },
}

pub(crate) fn typed_function(
    function: RawFunction,
    log: &ProvenanceLog,
) -> Result<Option<FunctionSpec>, String> {
    let returns = match function.returns {
        RawReturns::Scalar(type_name) => match cast_target(&type_name, log) {
            Ok(target) => FunctionReturns::Scalar(target.data_type),
            Err(_) => return Ok(None),
        },
        RawReturns::Table { table } => match typed_columns(table, None, log)? {
            Some(columns) => FunctionReturns::Table(columns),
            None => return Ok(None),
        },
    };
    let Some(arguments) = typed_columns(function.arguments, None, log)? else {
        return Ok(None);
    };
    Ok(Some(FunctionSpec {
        language: match function.language {
            RawLanguage::Sql => FunctionLanguage::Sql,
            RawLanguage::Python => FunctionLanguage::Python,
        },
        arguments,
        returns,
        traits: FunctionTraits {
            volatility: function.volatility,
            null_input: function.null_input,
            vectorized: function.vectorized,
        },
    }))
}

/// The columns of `relation`, or of a function when None; None when a type is unknown.
pub(crate) fn typed_columns(
    columns: RawColumns,
    relation: Option<&str>,
    log: &ProvenanceLog,
) -> Result<Option<Vec<ColumnSpec>>, String> {
    let mut typed: Vec<ColumnSpec> = Vec::with_capacity(columns.len());
    for column in columns {
        let (name, type_name, typed_leaves, offset) = match column {
            RawColumn::Pair(name, type_name) => (name, type_name, false, None),
            RawColumn::Described(column) => (
                column.name,
                column.type_name,
                column.typed_leaves,
                column.offset,
            ),
        };
        let Ok(target) = cast_target(&type_name, log) else {
            return Ok(None);
        };
        let declared = match offset {
            None => Vec::new(),
            Some(offset) => vec![declared_offset(
                (relation, &name),
                target.data_type,
                &offset,
            )?],
        };
        typed.push(ColumnSpec {
            typed_leaves,
            utc_offset: !declared.is_empty(),
            declared_offsets: declared,
            ..ColumnSpec::new(name, target.data_type)
        });
    }
    Ok(Some(typed))
}

/// The label of a relation column declared to hold the UTC offset; any other declaration fails.
fn declared_offset(
    (relation, name): (Option<&str>, &str),
    data_type: SnowflakeType,
    offset: &str,
) -> Result<String, String> {
    let Some(relation) = relation else {
        return Err(format!(
            "{name}: an offset may be declared only for a relation's column"
        ));
    };
    if !matches!(data_type, SnowflakeType::TimestampTz { .. }) {
        return Err(format!(
            "{relation}.{name}: an offset may be declared only for a TIMESTAMP_TZ column, not {data_type}"
        ));
    }
    if !DECLARABLE_OFFSETS
        .iter()
        .any(|declarable| declarable.eq_ignore_ascii_case(offset))
    {
        return Err(format!(
            "{relation}.{name}: the declared offset `{offset}` is not supported; only +00:00 (UTC) is"
        ));
    }
    Ok(format!("{relation}.{name}"))
}
