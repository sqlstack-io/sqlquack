//! Snowflake result types from the conversion crate's recorded typing rules; no type unrecorded.

use std::rc::Rc;

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::types::models::IntervalUnit;
use sqlquack_syntax::{DataType, Expression};

use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::Lineage;
use crate::snowflake::profile::_helpers::spelled_targets::spelled_target;
use crate::snowflake::profile::constants::{
    SNOWFLAKE_COUNT_FUNCTIONS, SNOWFLAKE_DATE_ADD_FUNCTIONS, SNOWFLAKE_DATE_DIFF_FUNCTIONS,
    SNOWFLAKE_DATE_UNITS, SNOWFLAKE_MICROSECOND_UNIT, SNOWFLAKE_RANKING_FUNCTIONS,
    SNOWFLAKE_TIME_UNITS,
};
use crate::snowflake::typing::constants::{
    DATEDIFF_PRECISIONS, TIME_DIFF_MICROSECONDS_TYPE, TIME_DIFF_TYPE, UNIT_ALIASES,
};
use crate::snowflake::typing::main::average::average;
use crate::snowflake::typing::main::count::count;
use crate::snowflake::typing::main::number_literal::number_literal;
use crate::snowflake::typing::main::numeric_add_subtract::numeric_add_subtract;
use crate::snowflake::typing::main::numeric_divide::numeric_divide;
use crate::snowflake::typing::main::numeric_modulo::numeric_modulo;
use crate::snowflake::typing::main::numeric_multiply::numeric_multiply;
use crate::snowflake::typing::main::ranking::ranking;
use crate::snowflake::typing::main::sum::sum;
use crate::snowflake::typing::main::unify::unify;
use crate::snowflake::typing::models::SnowflakeType;

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
        Expression::DateDiff(function) => {
            let unit: &str = function.unit.as_ref().map(interval_unit_name)?;
            Some(date_diff_type(
                unit,
                (
                    part_type(parts, &function.expression),
                    part_type(parts, &function.this),
                ),
            ))
        }
        Expression::Function(function) if !function.quoted => {
            temporal_call_type(&function.name, &function.args, parts)
                .or_else(|| named_type(&function.name, parts, &log))
        }
        Expression::AggregateFunction(function) => named_type(&function.name, parts, &log),
        _ => None,
    }
}

/// DATEADD and DATEDIFF written as calls: `(unit, amount, value)` and `(unit, start, end)`.
fn temporal_call_type(
    name: &str,
    arguments: &[Expression],
    parts: &[(&Expression, Lineage)],
) -> Owned {
    let upper: String = name.to_ascii_uppercase();
    let [unit, second, third] = arguments else {
        return None;
    };
    if SNOWFLAKE_DATE_ADD_FUNCTIONS.contains(&upper.as_str()) {
        return Some(date_add_type(
            unit_text(unit).as_deref(),
            part_type(parts, third),
        ));
    }
    if SNOWFLAKE_DATE_DIFF_FUNCTIONS.contains(&upper.as_str()) {
        let Some(unit) = unit_text(unit) else {
            return Some(None);
        };
        return Some(date_diff_type(
            &unit,
            (part_type(parts, second), part_type(parts, third)),
        ));
    }
    None
}

/// DATEADD as recorded: a timestamp's kind at precision 9, TIME to TIME(9), DATE for a date part.
fn date_add_type(unit: Option<&str>, value: Option<SnowflakeType>) -> Option<Rc<str>> {
    let unit: &str = unit?;
    let kept: SnowflakeType = match value? {
        SnowflakeType::TimestampNtz { .. } => SnowflakeType::TimestampNtz { precision: 9 },
        SnowflakeType::TimestampLtz { .. } => SnowflakeType::TimestampLtz { precision: 9 },
        SnowflakeType::TimestampTz { .. } => SnowflakeType::TimestampTz { precision: 9 },
        SnowflakeType::Time { .. } if SNOWFLAKE_TIME_UNITS.contains(&unit) => {
            SnowflakeType::Time { precision: 9 }
        }
        SnowflakeType::Date if SNOWFLAKE_DATE_UNITS.contains(&unit) => SnowflakeType::Date,
        _ => return None,
    };
    Some(Rc::from(kept.to_string()))
}

/// DATEDIFF's recorded NUMBER(p,0) for its unit, or the recorded TIME difference types.
fn date_diff_type(
    unit: &str,
    operands: (Option<SnowflakeType>, Option<SnowflakeType>),
) -> Option<Rc<str>> {
    let typed: SnowflakeType = match operands {
        (Some(SnowflakeType::Time { .. }), Some(SnowflakeType::Time { .. })) => {
            if unit == SNOWFLAKE_MICROSECOND_UNIT {
                TIME_DIFF_MICROSECONDS_TYPE
            } else {
                TIME_DIFF_TYPE
            }
        }
        (Some(start), Some(end)) if dated(start) && dated(end) => {
            let (_, precision) = DATEDIFF_PRECISIONS.iter().find(|(name, _)| *name == unit)?;
            SnowflakeType::Number {
                precision: *precision,
                scale: 0,
            }
        }
        _ => return None,
    };
    Some(Rc::from(typed.to_string()))
}

/// A DATE or timestamp of any kind.
fn dated(data_type: SnowflakeType) -> bool {
    matches!(
        data_type,
        SnowflakeType::Date
            | SnowflakeType::TimestampNtz { .. }
            | SnowflakeType::TimestampLtz { .. }
            | SnowflakeType::TimestampTz { .. }
    )
}

/// The canonical name of a unit written as a keyword, a bare name or a string.
fn unit_text(unit: &Expression) -> Option<String> {
    let written: &str = match unit {
        Expression::Var(var) => &var.this,
        Expression::Column(column) if column.table.is_none() => &column.name.name,
        Expression::Identifier(identifier) => &identifier.name,
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::String(text) => text,
            _ => return None,
        },
        _ => return None,
    };
    let upper: String = written.to_ascii_uppercase();
    Some(
        UNIT_ALIASES
            .iter()
            .find(|(alias, _)| *alias == upper)
            .map_or(upper, |(_, name)| (*name).to_owned()),
    )
}

fn interval_unit_name(unit: &IntervalUnit) -> &'static str {
    match unit {
        IntervalUnit::Year => "YEAR",
        IntervalUnit::Quarter => "QUARTER",
        IntervalUnit::Month => "MONTH",
        IntervalUnit::Week => "WEEK",
        IntervalUnit::Day => "DAY",
        IntervalUnit::Hour => "HOUR",
        IntervalUnit::Minute => "MINUTE",
        IntervalUnit::Second => "SECOND",
        IntervalUnit::Millisecond => "MILLISECOND",
        IntervalUnit::Microsecond => "MICROSECOND",
        IntervalUnit::Nanosecond => "NANOSECOND",
    }
}

/// The recorded type of `child`, a child of the node being typed.
fn part_type(parts: &[(&Expression, Lineage)], child: &Expression) -> Option<SnowflakeType> {
    parts
        .iter()
        .find(|(part, _)| std::ptr::eq(*part, child))
        .and_then(|(_, lineage)| known(lineage))
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
    spelled_target(part.data_type.as_deref()?).map(|target| target.data_type)
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

/// A Snowflake cast target's spelling; TIME keeps the fractional-second precision it states.
pub(crate) fn snowflake_cast_spelling(data_type: &DataType) -> Option<String> {
    match data_type {
        DataType::Time {
            precision: Some(precision),
            timezone: false,
        } => Some(format!("TIME({precision})")),
        other => rendered_type(other),
    }
}
