//! Columns, literals, parentheses, negation and scalar subqueries.

use serde_json::Value;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_snowflake::typing::main::literal_digits::literal_digits;
use sqlquack_snowflake::typing::main::number_literal::number_literal;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::collecting::{
    collecting, guessed, opaque_in_scope, placeholder,
};
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    LITERAL_NUMBER, LITERAL_STRING, field, identifier_name, missing, node_construct,
    outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    COLUMN, COMMENT, CONSTANT_LITERAL, NEGATE, NUMBER_LITERAL, PARENTHESES, SCALAR_SUBQUERY,
};

/// A column reference.
pub(crate) fn column(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("column");
    let _rule = site.context.log.rule(COLUMN);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["name", "table"], &construct)?;
    let name = identifier_name(field(payload, "name", &construct)?)
        .ok_or_else(|| missing(construct.clone(), "a column has no name".to_owned()))?;
    let qualifier = payload.get("table").and_then(identifier_name);
    match site
        .scope
        .resolve(qualifier.as_deref(), &name, site.context.log)
    {
        Ok(emitted) => {
            if emitted.data_type == SnowflakeType::Null && collecting() {
                guessed();
            }
            Ok(emitted)
        }
        Err(_) if collecting() && opaque_in_scope(site.scope, qualifier.as_deref()) => {
            Ok(placeholder(None))
        }
        Err(refusal) => Err(refusal),
    }
}

/// A numeric or string literal.
pub(crate) fn literal(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("literal");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["literal_type", "value"], &construct)?;
    let text = payload
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let log = site.context.log;
    match payload.get("literal_type").and_then(Value::as_str) {
        Some(LITERAL_NUMBER) => {
            let _rule = log.rule(NUMBER_LITERAL);
            let data_type = number_literal(text, log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))?;
            let digits = literal_digits(text)
                .map_err(|reason| outside_conditions(construct.clone(), reason))?;
            Ok(Emitted {
                sql: cast(&digits, data_type, log)
                    .map_err(|reason| outside_conditions(construct, reason))?,
                data_type,
                literal: Some(Literal::Number(text.to_owned())),
                utc_offset: false,
            })
        }
        Some(LITERAL_STRING) => {
            let _rule = log.rule(CONSTANT_LITERAL);
            Ok(Emitted {
                sql: string_literal(text, log),
                data_type: SnowflakeType::Varchar,
                literal: Some(Literal::Text(text.to_owned())),
                utc_offset: false,
            })
        }
        other => Err(missing(
            construct,
            format!("no rule converts {} literals", other.unwrap_or("unknown")),
        )),
    }
}

/// TRUE or FALSE.
pub(crate) fn boolean_literal(payload: &Value, site: Site<'_>) -> Emitted {
    let _rule = site.context.log.rule(CONSTANT_LITERAL);
    let sql = match payload.get("value").and_then(Value::as_bool) {
        Some(true) => "TRUE",
        _ => "FALSE",
    };
    Emitted::new(sql.to_owned(), SnowflakeType::Boolean)
}

/// NULL, which takes the type of its context.
pub(crate) fn null_literal(site: Site<'_>) -> Emitted {
    let _rule = site.context.log.rule(CONSTANT_LITERAL);
    Emitted::new("NULL".to_owned(), SnowflakeType::Null)
}

/// A parenthesised expression.
pub(crate) fn parentheses(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("paren");
    let _rule = site.context.log.rule(PARENTHESES);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this"], &construct)?;
    let inner = convert_expression(field(payload, "this", &construct)?, site)?;
    Ok(Emitted {
        sql: format!("({})", inner.sql),
        ..inner
    })
}

/// An expression with trailing comments: the expression alone.
pub(crate) fn commented(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("annotated");
    let _rule = site.context.log.rule(COMMENT);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "trailing_comments"], &construct)?;
    convert_expression(field(payload, "this", &construct)?, site)
}

/// Unary minus, which keeps the operand's type.
pub(crate) fn negate(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("neg");
    let _rule = site.context.log.rule(NEGATE);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this"], &construct)?;
    let inner = convert_expression(field(payload, "this", &construct)?, site)?;
    match inner.data_type {
        SnowflakeType::Number { .. } | SnowflakeType::Float => {
            Ok(Emitted::new(format!("(-{})", inner.sql), inner.data_type))
        }
        other => Err(outside_conditions(construct, format!("negating {other}"))),
    }
}

/// A subquery that returns one column, used as a value.
pub(crate) fn scalar_subquery(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("subquery");
    let _rule = site.context.log.rule(SCALAR_SUBQUERY);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this"], &construct)?;
    let (sql, columns) = convert_query(
        field(payload, "this", &construct)?,
        Some(site.scope),
        site.context,
        false,
    )?;
    match columns.as_slice() {
        [column] => Ok(Emitted::new(format!("({sql})"), column.data_type)),
        _ => Err(outside_conditions(
            construct,
            "a scalar subquery must return exactly one column".to_owned(),
        )),
    }
}
