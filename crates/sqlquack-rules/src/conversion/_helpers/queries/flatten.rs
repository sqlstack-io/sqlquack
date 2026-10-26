//! LATERAL FLATTEN of a VARIANT: one row per array element or object key.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb::semi_structured::main::json_flatten::json_flatten;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Scope, ScopeRelation};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, function_construct, identifier_name, list, missing, node_construct,
    outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{FLATTEN_COLUMNS, FLATTEN_RULE};
use crate::conversion::models::ColumnSpec;

/// The name of the table function LATERAL converts.
const FLATTEN_NAME: &str = "FLATTEN";

/// A converted LATERAL FLATTEN: its join text and the relation it brings into scope.
pub(crate) struct Flattened {
    pub(crate) sql: String,
    pub(crate) relation: ScopeRelation,
}

/// `LATERAL FLATTEN(input => v[, outer => TRUE])` over the relations before it.
pub(crate) fn lateral_flatten(
    payload: &Value,
    earlier: &[ScopeRelation],
    outer: Option<&Scope<'_>>,
    context: &Context<'_>,
) -> Result<Flattened, Refusal> {
    let construct = node_construct("lateral");
    let lateral = payload_object(payload, &construct)?;
    require_understood(lateral, &["this", "alias"], &construct)?;
    let call = field(lateral, "this", &construct)?
        .get("function")
        .and_then(Value::as_object)
        .ok_or_else(|| missing(construct.clone(), "LATERAL of this source".to_owned()))?;
    let name = call
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let function = function_construct(&name);
    if name != FLATTEN_NAME {
        return Err(missing(
            function,
            format!("no rule converts LATERAL {name}"),
        ));
    }
    let _rule = context.log.rule(FLATTEN_RULE);
    let alias = lateral
        .get("alias")
        .and_then(Value::as_str)
        .map(str::to_ascii_uppercase)
        .ok_or_else(|| missing(function.clone(), "a FLATTEN without an alias".to_owned()))?;
    let (input, keep_empty) = arguments(call, &function)?;
    let scope = Scope::new(earlier.to_vec(), outer);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let input = convert_expression(input, site)?;
    if input.data_type != SnowflakeType::SemiStructured {
        return Err(outside_conditions(
            function,
            format!("FLATTEN of a {} value, not a VARIANT", input.data_type),
        ));
    }
    diverge(context, JSON_FIDELITY, &function)?;
    let log = context.log;
    let flattened = format!(
        "LATERAL ({}) AS {}",
        json_flatten(&input.sql, log),
        quoted_identifier(&alias, log)
    );
    let sql = match keep_empty {
        true => format!(" LEFT JOIN {flattened} ON TRUE"),
        false => format!(" CROSS JOIN {flattened}"),
    };
    let columns: Vec<ColumnSpec> = FLATTEN_COLUMNS
        .iter()
        .map(|(column, data_type)| ColumnSpec {
            name: (*column).to_owned(),
            data_type: *data_type,
        })
        .collect();
    Ok(Flattened {
        sql,
        relation: ScopeRelation { alias, columns },
    })
}

/// The input and OUTER flag; PATH, RECURSIVE and MODE are refused.
fn arguments<'a>(call: &'a Payload, construct: &Construct) -> Result<(&'a Value, bool), Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let mut input: Option<&Value> = None;
    let mut keep_empty = false;
    for (position, argument) in list(call, "args").iter().enumerate() {
        let Some(named) = argument.get("named_argument") else {
            if position > 0 {
                return Err(refuse(
                    "FLATTEN with positional arguments after the input".to_owned(),
                ));
            }
            input = Some(argument);
            continue;
        };
        let name = named
            .get("name")
            .and_then(identifier_name)
            .unwrap_or_default();
        let value = named.get("value").unwrap_or(&Value::Null);
        match name.as_str() {
            "INPUT" => input = Some(value),
            "OUTER" => {
                keep_empty = value
                    .get("boolean")
                    .and_then(|boolean| boolean.get("value"))
                    .and_then(Value::as_bool)
                    .ok_or_else(|| refuse("an OUTER other than a boolean literal".to_owned()))?;
            }
            other => return Err(refuse(format!("FLATTEN with {other}"))),
        }
    }
    input
        .map(|input| (input, keep_empty))
        .ok_or_else(|| refuse("FLATTEN without an input".to_owned()))
}
