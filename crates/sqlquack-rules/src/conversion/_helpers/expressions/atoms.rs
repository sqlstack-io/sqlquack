//! Columns, literals, parentheses, negation and scalar subqueries.

use serde_json::{Value, json};
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::semi_structured::constants::VARIANT_CHAIN_TYPE;
use sqlquack_snowflake::typing::main::literal_digits::literal_digits;
use sqlquack_snowflake::typing::main::number_literal::number_literal;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::{convert_expression, convert_held};
use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::collecting::{
    collecting, guessed, opaque_in_scope, placeholder,
};
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal, resolve};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    LITERAL_NUMBER, LITERAL_STRING, field, identifier_name, node_construct, payload_object,
    require_understood,
};
use crate::conversion::constants::{
    COLUMN, COMMENT, CONSTANT_LITERAL, DOLLAR_PARAMETER, NEGATE, NUMBER_LITERAL, PARENTHESES,
    POSITIONAL_COLUMN, SCALAR_SUBQUERY, TYPED_LITERAL_KINDS, TYPED_LITERAL_PAIR,
};
use sqlquack_semantics::types::models::Holding;

/// A column reference.
pub(crate) fn column(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("column");
    let _rule = site.context.log.rule(COLUMN);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["name", "table"], &construct)?;
    let name = identifier_name(field(payload, "name", &construct)?)
        .ok_or_else(|| missing(construct.clone(), "a column has no name".to_owned()))?;
    let qualifier = payload.get("table").and_then(identifier_name);
    match resolve(site.scope, qualifier.as_deref(), &name, site.context.log) {
        Ok(emitted) => {
            site.context
                .typed
                .read((emitted.untainted, emitted.sourced));
            if emitted.data_type == SnowflakeType::Null && collecting() {
                guessed();
            }
            if emitted.held != Holding::Chained {
                return Ok(emitted);
            }
            diverge(site.context, VARIANT_CHAIN_TYPE, &construct)?;
            Ok(Emitted {
                held: Holding::Json,
                ..emitted
            })
        }
        Err(_) if collecting() && opaque_in_scope(site.scope, qualifier.as_deref()) => {
            Ok(placeholder(None))
        }
        Err(refusal) => Err(refusal),
    }
}

/// `$n`: the n-th column of the query level's only FROM source, read as that column.
pub(crate) fn positional_column(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("parameter");
    let _rule = site.context.log.rule(POSITIONAL_COLUMN);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["index", "style"], &construct)?;
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    if payload.get("style").and_then(Value::as_str) != Some(DOLLAR_PARAMETER) {
        return Err(refuse("a parameter other than a $n column position"));
    }
    let index = match payload
        .get("index")
        .and_then(Value::as_u64)
        .map(usize::try_from)
    {
        Some(Ok(index)) if index > 0 => index,
        _ => {
            return Err(refuse(
                "a column position that is not a positive whole number",
            ));
        }
    };
    let [relation] = site.scope.relations.as_slice() else {
        return Err(refuse(
            "a $n column position over other than one FROM source",
        ));
    };
    let name = relation
        .columns
        .get(index - 1)
        .map(|column| column.name.clone())
        .ok_or_else(|| refuse("a $n column position beyond the source's columns"))?;
    let alias = relation.alias.clone();
    column(
        &serde_json::json!({ "name": { "name": name }, "table": { "name": alias } }),
        site,
    )
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
            let sql = cast(&digits, data_type, log)
                .map_err(|reason| outside_conditions(construct, reason))?;
            Ok(Emitted {
                literal: Some(Literal::Number(text.to_owned())),
                ..Emitted::new(sql, data_type)
            })
        }
        Some(LITERAL_STRING) => {
            let _rule = log.rule(CONSTANT_LITERAL);
            Ok(Emitted {
                literal: Some(Literal::Text(text.to_owned())),
                ..Emitted::new(string_literal(text, log), SnowflakeType::Varchar)
            })
        }
        Some(kind) if TYPED_LITERAL_KINDS.contains(&kind) => {
            log.equivalence(TYPED_LITERAL_PAIR);
            let cast = json!({ "cast": {
                "this": { "literal": { "literal_type": LITERAL_STRING, "value": text } },
                "to": { "data_type": kind },
            }});
            convert_expression(&cast, site)
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
    let inner = convert_held(field(payload, "this", &construct)?, site)?;
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
    convert_held(field(payload, "this", &construct)?, site)
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
        [column] => Ok(Emitted {
            held: column.holding,
            untainted: column.untainted,
            sourced: column.sourced,
            utc_offset: column.utc_offset,
            declared: column.declared_offsets.clone(),
            ..Emitted::new(format!("({sql})"), column.data_type)
        }),
        _ => Err(outside_conditions(
            construct,
            "a scalar subquery must return exactly one column".to_owned(),
        )),
    }
}
