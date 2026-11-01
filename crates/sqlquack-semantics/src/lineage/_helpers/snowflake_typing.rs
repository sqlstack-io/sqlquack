//! Snowflake result types from the conversion crate's recorded typing rules; no type unrecorded.

use std::rc::Rc;

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::main::average::average;
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_snowflake::typing::main::count::count;
use sqlquack_snowflake::typing::main::number_literal::number_literal;
use sqlquack_snowflake::typing::main::numeric_add_subtract::numeric_add_subtract;
use sqlquack_snowflake::typing::main::numeric_divide::numeric_divide;
use sqlquack_snowflake::typing::main::numeric_modulo::numeric_modulo;
use sqlquack_snowflake::typing::main::numeric_multiply::numeric_multiply;
use sqlquack_snowflake::typing::main::ranking::ranking;
use sqlquack_snowflake::typing::main::sum::sum;
use sqlquack_snowflake::typing::main::unify::unify;
use sqlquack_snowflake::typing::models::SnowflakeType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;

use crate::lineage::constants::{SNOWFLAKE_COUNT_FUNCTIONS, SNOWFLAKE_RANKING_FUNCTIONS};
use crate::lineage::models::Lineage;

/// What a rule answers: `None` when no rule owns the node, else its spelling, which may be unknown.
type Owned = Option<Option<Rc<str>>>;

/// A numeric literal: NUMBER(p,s) from its digits, as recorded; untyped with an exponent.
pub(crate) fn snowflake_literal_type(literal: &Literal) -> Owned {
    let Literal::Number(text) = literal else {
        return None;
    };
    if text.contains(['e', 'E']) {
        return Some(None);
    }
    Some(spelled(number_literal(text, &ProvenanceLog::default())))
}

/// The recorded Snowflake type of `expression`, when a rule owns it; its children in `parts`.
pub(crate) fn snowflake_type(expression: &Expression, parts: &[(&Expression, Lineage)]) -> Owned {
    let log = ProvenanceLog::default();
    let binary = |rule: fn(
        SnowflakeType,
        SnowflakeType,
        &ProvenanceLog,
    ) -> Result<SnowflakeType, String>| {
        let [(_, left), (_, right)] = parts else {
            return None;
        };
        let (left, right) = (numeric(left)?, numeric(right)?);
        Some(spelled(rule(left, right, &log)))
    };
    let first = || parts.first().and_then(|(_, part)| known(part));
    match expression {
        Expression::Add(_) | Expression::Sub(_) => binary(numeric_add_subtract),
        Expression::Mul(_) => binary(numeric_multiply),
        Expression::Div(_) => binary(numeric_divide),
        Expression::Mod(_) => binary(numeric_modulo),
        Expression::Count(_) => Some(Some(Rc::from(count(&log).to_string()))),
        Expression::RowNumber(_) | Expression::Rank(_) | Expression::DenseRank(_) => {
            Some(Some(Rc::from(ranking(&log).to_string())))
        }
        Expression::Sum(_) => Some(first().and_then(|operand| spelled(sum(operand, &log)))),
        Expression::Avg(_) => Some(first().and_then(|operand| spelled(average(operand, &log)))),
        Expression::Function(function) if !function.quoted => {
            named_type(&function.name, parts, &log)
        }
        Expression::AggregateFunction(function) => named_type(&function.name, parts, &log),
        _ => None,
    }
}

fn named_type(name: &str, parts: &[(&Expression, Lineage)], log: &ProvenanceLog) -> Owned {
    let upper: String = name.to_ascii_uppercase();
    let first = || parts.first().and_then(|(_, part)| known(part));
    if SNOWFLAKE_COUNT_FUNCTIONS.contains(&upper.as_str()) {
        return Some(Some(Rc::from(count(log).to_string())));
    }
    if SNOWFLAKE_RANKING_FUNCTIONS.contains(&upper.as_str()) {
        return Some(Some(Rc::from(ranking(log).to_string())));
    }
    match upper.as_str() {
        "SUM" => Some(first().and_then(|operand| spelled(sum(operand, log)))),
        "AVG" => Some(first().and_then(|operand| spelled(average(operand, log)))),
        _ => None,
    }
}

/// The common type of value operands, by the recorded unification; `None` when one is unknown.
pub(crate) fn snowflake_unified(operands: &[&Lineage]) -> Option<Rc<str>> {
    let mut types: Vec<SnowflakeType> = Vec::with_capacity(operands.len());
    for operand in operands {
        types.push(known(operand)?);
    }
    spelled(unify(&types, &ProvenanceLog::default()))
}

/// The type a value's spelling denotes, when the recorded cast targets know it.
fn known(part: &Lineage) -> Option<SnowflakeType> {
    match cast_target(part.data_type.as_deref()?, &ProvenanceLog::default()) {
        Ok(target) => Some(target.data_type),
        Err(_unknown) => None,
    }
}

/// A NUMBER or FLOAT operand; anything else leaves arithmetic to other rules.
fn numeric(part: &Lineage) -> Option<SnowflakeType> {
    known(part)
        .filter(|operand| matches!(operand, SnowflakeType::Number { .. } | SnowflakeType::Float))
}

fn spelled(typed: Result<SnowflakeType, String>) -> Option<Rc<str>> {
    match typed {
        Ok(data_type) => Some(Rc::from(data_type.to_string())),
        Err(_unrecorded) => None,
    }
}
