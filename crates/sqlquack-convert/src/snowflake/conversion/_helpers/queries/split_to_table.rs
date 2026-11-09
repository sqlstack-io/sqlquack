//! LATERAL SPLIT_TO_TABLE of a text: one row per delimited part.

use serde_json::Value;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::blocks::main::string_literal::string_literal;
use sqlquack_blocks::text::main::split_to_rows::split_to_rows;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::scalar::convert_expression;
use crate::snowflake::conversion::_helpers::queries::flatten::Flattened;
use crate::snowflake::conversion::_helpers::support::scope::{Context, Literal, Scope};
use crate::snowflake::conversion::_helpers::support::site::{Placement, Site};
use crate::snowflake::conversion::_helpers::support::tree::{
    Payload, function_construct, integer_literal, list,
};
use crate::snowflake::conversion::constants::{
    CHR_NAME, SPLIT_TO_TABLE_COLUMNS, SPLIT_TO_TABLE_RULE,
};
use sqlquack_semantics::snowflake::typing::types::ColumnSpec;
use sqlquack_semantics::snowflake::typing::types::ScopeRelation;

/// The name of the table function this module converts.
pub(crate) const SPLIT_TO_TABLE_NAME: &str = "SPLIT_TO_TABLE";

/// `LATERAL SPLIT_TO_TABLE(text, delimiter) alias` over the relations before it.
pub(crate) fn lateral_split(
    call: &Payload,
    lateral: &Payload,
    (earlier, outer): (&[ScopeRelation], Option<&Scope<'_>>),
    context: &Context<'_>,
) -> Result<Flattened, Refusal> {
    let function = function_construct(SPLIT_TO_TABLE_NAME);
    let _rule = context.log.rule(SPLIT_TO_TABLE_RULE);
    let alias = lateral
        .get("alias")
        .and_then(Value::as_str)
        .map(str::to_ascii_uppercase)
        .ok_or_else(|| {
            missing(
                function.clone(),
                "a SPLIT_TO_TABLE without an alias".to_owned(),
            )
        })?;
    let [text, delimiter] = list(call, "args") else {
        return Err(outside_conditions(
            function,
            "SPLIT_TO_TABLE takes a text and a delimiter".to_owned(),
        ));
    };
    let scope = Scope::new(earlier.to_vec(), outer);
    let site = Site {
        scope: &scope,
        context,
        placement: Placement::ROW,
    };
    let text = convert_expression(text, site)?;
    if text.data_type != SnowflakeType::Varchar {
        return Err(outside_conditions(
            function,
            format!(
                "SPLIT_TO_TABLE of a {} value, not a VARCHAR",
                text.data_type
            ),
        ));
    }
    let delimiter = delimiter_sql(delimiter, &function, site)?;
    let log = context.log;
    let columns: Vec<ColumnSpec> = SPLIT_TO_TABLE_COLUMNS
        .iter()
        .map(|(column, data_type)| ColumnSpec::new(*column, *data_type))
        .collect();
    Ok(Flattened {
        sql: format!(
            " CROSS JOIN LATERAL ({}) AS {}",
            split_to_rows(&text.sql, &delimiter, log),
            quoted_identifier(&alias, log)
        ),
        relation: ScopeRelation {
            alias,
            columns,
            merged: Vec::new(),
        },
    })
}

/// A non-empty string literal, or CHR of a literal code point.
fn delimiter_sql(node: &Value, function: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    if let Some(character) = character_literal(node) {
        return Ok(string_literal(&character.to_string(), site.context.log));
    }
    match convert_expression(node, site)?.literal {
        Some(Literal::Text(text)) if !text.is_empty() => {
            Ok(string_literal(&text, site.context.log))
        }
        _ => Err(outside_conditions(
            function.clone(),
            "a delimiter other than a non-empty string literal".to_owned(),
        )),
    }
}

/// The character of `CHR(n)` for a literal code point n of a character other than NUL.
fn character_literal(node: &Value) -> Option<char> {
    let call = node.get("function")?;
    if !call.get("name")?.as_str()?.eq_ignore_ascii_case(CHR_NAME) {
        return None;
    }
    let [code] = call.get("args")?.as_array()?.as_slice() else {
        return None;
    };
    let code = u32::try_from(integer_literal(code)?).unwrap_or(u32::MAX);
    char::from_u32(code).filter(|character| *character != '\0')
}
