//! LATERAL FLATTEN of a VARIANT: one row per array element or object key.

use serde_json::Value;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_blocks::semi_structured::main::json_flatten::json_flatten;
use sqlquack_blocks::semi_structured::main::json_flatten_recursive::json_flatten_recursive;
use sqlquack_blocks::semi_structured::models::FlattenMode;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::held::read_json;
use crate::snowflake::conversion::_helpers::expressions::scalar::convert_held;
use crate::snowflake::conversion::_helpers::queries::split_to_table::{
    SPLIT_TO_TABLE_NAME, lateral_split,
};
use crate::snowflake::conversion::_helpers::support::divergence::diverge;
use crate::snowflake::conversion::_helpers::support::scope::{Context, Scope};
use crate::snowflake::conversion::_helpers::support::site::{Placement, Site};
use crate::snowflake::conversion::_helpers::support::tree::{
    Payload, field, function_construct, identifier_name, list, node_construct, payload_object,
    require_understood,
};
use crate::snowflake::conversion::constants::{
    FLATTEN_COLUMNS, FLATTEN_ORDERED_COLUMNS, FLATTEN_RULE,
};
use sqlquack_semantics::snowflake::scopes::models::ScopeRelation;
use sqlquack_semantics::snowflake::types::models::{ColumnSpec, Holding};

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
    if name == SPLIT_TO_TABLE_NAME {
        return lateral_split(call, lateral, (earlier, outer), context);
    }
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
    let Arguments {
        input,
        keep_empty,
        recursive,
        mode,
        path,
    } = arguments(call, &function)?;
    let scope = Scope::new(earlier.to_vec(), outer);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement {
            order_free: true,
            ..Placement::ROW
        },
    };
    let mark = context.order.mark();
    let input = convert_held(input, site)?;
    context.order.absorb(mark);
    if input.data_type != SnowflakeType::SemiStructured {
        return Err(outside_conditions(
            function,
            format!("FLATTEN of a {} value, not a VARIANT", input.data_type),
        ));
    }
    diverge(context, JSON_FIDELITY, &function)?;
    let log = context.log;
    let json = read_json(&input, context);
    let json = match path.is_empty() {
        true => json,
        false => format!(
            "json_extract({json}, '${}')",
            path.iter()
                .map(|key| format!(".\"{key}\""))
                .collect::<String>()
        ),
    };
    let rows = match (recursive, mode) {
        (true, mode) => json_flatten_recursive(&json, mode, log),
        (false, mode) => json_flatten(&json, (mode, keep_empty), log),
    };
    let rows = match path.join(".") {
        prefix if prefix.is_empty() => rows,
        prefix => format!(
            "SELECT \"INDEX\", \"KEY\", CASE WHEN starts_with(\"PATH\", '[') \
             THEN '{prefix}' || \"PATH\" ELSE '{prefix}.' || \"PATH\" END AS \"PATH\", \
             \"VALUE\", \"THIS\" FROM ({rows})"
        ),
    };
    let flattened = format!("LATERAL ({rows}) AS {}", quoted_identifier(&alias, log));
    let sql = format!(" CROSS JOIN {flattened}");
    let columns: Vec<ColumnSpec> = FLATTEN_COLUMNS
        .iter()
        .map(|(column, data_type)| ColumnSpec {
            untainted: input.untainted,
            sourced: input.sourced,
            holding: match (*data_type, input.is_variant_held()) {
                (SnowflakeType::SemiStructured, true) => Holding::Variant,
                _ => Holding::Json,
            },
            unordered: input.unordered && FLATTEN_ORDERED_COLUMNS.contains(column),
            ..ColumnSpec::new(*column, *data_type)
        })
        .collect();
    Ok(Flattened {
        sql,
        relation: ScopeRelation {
            alias,
            columns,
            merged: Vec::new(),
        },
    })
}

/// A FLATTEN call's input, OUTER and RECURSIVE flags, MODE and PATH keys.
struct Arguments<'a> {
    input: &'a Value,
    keep_empty: bool,
    recursive: bool,
    mode: FlattenMode,
    path: Vec<String>,
}

/// The input, OUTER, RECURSIVE, MODE and a PATH of dotted keys; OUTER with RECURSIVE or PATH is refused.
fn arguments<'a>(call: &'a Payload, construct: &Construct) -> Result<Arguments<'a>, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let mut input: Option<&Value> = None;
    let mut keep_empty = false;
    let mut recursive = false;
    let mut mode: Option<FlattenMode> = None;
    let mut path: Vec<String> = Vec::new();
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
                keep_empty = boolean_literal(value)
                    .ok_or_else(|| refuse("an OUTER other than a boolean literal".to_owned()))?;
            }
            "RECURSIVE" => {
                recursive = boolean_literal(value)
                    .ok_or_else(|| refuse("a RECURSIVE other than a boolean literal".to_owned()))?;
            }
            "MODE" => {
                let written = value
                    .get("literal")
                    .and_then(|literal| literal.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                mode = Some(match written.to_ascii_uppercase().as_str() {
                    "BOTH" => FlattenMode::Both,
                    "OBJECT" => FlattenMode::Object,
                    "ARRAY" => FlattenMode::Array,
                    _ => return Err(refuse("a MODE other than a literal mode".to_owned())),
                });
            }
            "PATH" => {
                let written = value
                    .get("literal")
                    .and_then(|literal| literal.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                path = written.split('.').map(str::to_owned).collect();
                if !path.iter().all(|key| plain_key(key)) {
                    return Err(refuse(
                        "a PATH other than literal dot-separated keys, which no recording shows"
                            .to_owned(),
                    ));
                }
            }
            other => return Err(refuse(format!("FLATTEN with {other}"))),
        }
    }
    if recursive && keep_empty {
        return Err(refuse("FLATTEN with RECURSIVE and OUTER".to_owned()));
    }
    if !path.is_empty() && (recursive || keep_empty) {
        return Err(refuse(
            "FLATTEN with PATH and RECURSIVE or OUTER, which no recording shows".to_owned(),
        ));
    }
    let input = input.ok_or_else(|| refuse("FLATTEN without an input".to_owned()))?;
    Ok(Arguments {
        input,
        keep_empty,
        recursive,
        mode: mode.unwrap_or(FlattenMode::Both),
        path,
    })
}

/// Whether a PATH step is a key needing no quotes: a letter or `_`, then letters, digits or `_`.
fn plain_key(key: &str) -> bool {
    let mut characters = key.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|next| next.is_ascii_alphanumeric() || next == '_')
}

/// The value of a boolean literal.
fn boolean_literal(value: &Value) -> Option<bool> {
    value
        .get("boolean")
        .and_then(|boolean| boolean.get("value"))
        .and_then(Value::as_bool)
}
