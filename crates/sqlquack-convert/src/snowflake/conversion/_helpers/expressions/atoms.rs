//! Columns, literals, parentheses, negation and scalar subqueries.

use serde_json::{Value, json};
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::blocks::main::string_literal::string_literal;
use sqlquack_blocks::semi_structured::constants::VARIANT_CHAIN_TYPE;
use sqlquack_diagnostics::classification::constants::{CURSOR_CONSTRUCT, CURSOR_PLACEHOLDER};
use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::main::literal_digits::literal_digits;
use sqlquack_semantics::snowflake::typing::main::number_literal::number_literal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::casts::cast_to;
use crate::snowflake::conversion::_helpers::expressions::scalar::{
    convert_expression, convert_held,
};
use crate::snowflake::conversion::_helpers::queries::query::convert_query;
use crate::snowflake::conversion::_helpers::support::collecting::{
    collecting, guessed, opaque_in_scope, placeholder,
};
use crate::snowflake::conversion::_helpers::support::divergence::diverge;
use crate::snowflake::conversion::_helpers::support::scope::{Emitted, Literal, Scope, resolve};
use crate::snowflake::conversion::_helpers::support::site::Site;
use crate::snowflake::conversion::_helpers::support::tree::{
    LITERAL_NUMBER, LITERAL_STRING, Payload, field, identifier_name, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::snowflake::conversion::constants::{
    COLUMN, COMMENT, CONSTANT_LITERAL, DOLLAR_PARAMETER, DOLLAR_STRING_KIND, NEGATE,
    NUMBER_LITERAL, PARENTHESES, POSITIONAL_COLUMN, SCALAR_SUBQUERY, SEQUENCE_COLUMN,
    SEQUENCE_PEERS, TYPED_LITERAL_KINDS, TYPED_LITERAL_PAIR, UNARY_PLUS,
};
use sqlquack_semantics::snowflake::scopes::models::ScopeRelation;
use sqlquack_semantics::snowflake::types::models::Holding;
use sqlquack_semantics::snowflake::typing::constants::MAX_PRECISION;

/// The integer digits a unary plus gives a NUMBER at least, within 38: + 1 is NUMBER(2,0).
const PLUS_INTEGER_DIGITS: u8 = 2;

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
        Err(_) if qualifier.is_none() && name.eq_ignore_ascii_case(CURSOR_PLACEHOLDER) => {
            Err(missing(
                Construct::new(ConstructFamily::Function, CURSOR_CONSTRUCT),
                "a microbatch cursor bound, __cursor_start() or __cursor_end(), which converts \
                 only where its type is known (census --cursor-type)"
                    .to_owned(),
            ))
        }
        Err(_) if sequence_column(site.scope, qualifier.as_deref(), &name) => {
            Err(outside_conditions(
                construct,
                "SEQ of FLATTEN or SPLIT_TO_TABLE, which no rule converts".to_owned(),
            ))
        }
        Err(refusal) => Err(refusal),
    }
}

/// Whether `qualifier.name` is SEQ of a FLATTEN or SPLIT_TO_TABLE source, which neither scopes.
fn sequence_column(scope: &Scope<'_>, qualifier: Option<&str>, name: &str) -> bool {
    let Some(qualifier) = qualifier else {
        return false;
    };
    let table_function = |relation: &ScopeRelation| {
        relation.alias.eq_ignore_ascii_case(qualifier)
            && SEQUENCE_PEERS.iter().all(|peer| has_column(relation, peer))
    };
    let mut level = Some(scope);
    while let Some(current) = level {
        if current.relations.iter().any(table_function) {
            return name.eq_ignore_ascii_case(SEQUENCE_COLUMN);
        }
        level = current.outer;
    }
    false
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
        Some(LITERAL_STRING | DOLLAR_STRING_KIND) => {
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
    let inner = convert_expression(unsigned_operand(payload, &construct)?, site)?;
    match inner.data_type {
        SnowflakeType::Number { .. } | SnowflakeType::Float => {
            Ok(Emitted::new(format!("(-{})", inner.sql), inner.data_type))
        }
        other => Err(outside_conditions(construct, format!("negating {other}"))),
    }
}

/// Unary plus: a NUMBER given two integer digits at least, a FLOAT unchanged, text read as FLOAT.
pub(crate) fn unary_plus(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("pos");
    let _rule = site.context.log.rule(UNARY_PLUS);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this"], &construct)?;
    let inner = convert_expression(unsigned_operand(payload, &construct)?, site)?;
    match inner.data_type {
        SnowflakeType::Number { precision, scale } => {
            let widened = (scale + precision.saturating_sub(scale).max(PLUS_INTEGER_DIGITS))
                .min(MAX_PRECISION);
            let data_type = SnowflakeType::Number {
                precision: widened,
                scale,
            };
            match widened == precision {
                true => Ok(Emitted::new(inner.sql, data_type)),
                false => Ok(Emitted::new(
                    cast(&inner.sql, data_type, site.context.log)
                        .map_err(|reason| outside_conditions(construct.clone(), reason))?,
                    data_type,
                )),
            }
        }
        SnowflakeType::Float => Ok(Emitted::new(inner.sql, inner.data_type)),
        SnowflakeType::Varchar => Ok(Emitted::new(
            cast_to(&inner, "FLOAT", false, (site.context, &construct))
                .map_err(|reason| outside_conditions(construct.clone(), reason))?,
            SnowflakeType::Float,
        )),
        other => Err(outside_conditions(
            construct,
            format!("unary plus of {other}"),
        )),
    }
}

/// A sign's operand, refused when it is itself a sign, as Snowflake rejects `++1` and `- -x` (1005).
fn unsigned_operand<'a>(payload: &'a Payload, construct: &Construct) -> Result<&'a Value, Refusal> {
    let operand = field(payload, "this", construct)?;
    match node_kind(operand) {
        Some(("pos" | "neg", _)) => Err(outside_conditions(
            construct.clone(),
            "two adjacent signs, which Snowflake rejects as an invalid function (1005)".to_owned(),
        )),
        _ => Ok(operand),
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
        &site.context.projecting(false),
        false,
    )?;
    let sql = format!("({sql})");
    match columns.as_slice() {
        [column] => Ok(Emitted {
            held: column.holding,
            untainted: column.untainted,
            sourced: column.sourced,
            utc_offset: column.utc_offset,
            declared: column.declared_offsets.clone(),
            ..Emitted::new(sql, column.data_type)
        }),
        _ => Err(outside_conditions(
            construct,
            "a scalar subquery must return exactly one column".to_owned(),
        )),
    }
}

/// Whether `relation` has a column named `name`, ignoring case.
fn has_column(relation: &ScopeRelation, name: &str) -> bool {
    relation
        .columns
        .iter()
        .any(|column| column.name.eq_ignore_ascii_case(name))
}
