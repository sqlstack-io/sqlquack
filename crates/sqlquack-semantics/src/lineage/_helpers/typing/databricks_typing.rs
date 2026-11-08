//! Databricks SQL's result types as the pinned warehouse's `typeof` and `DESCRIBE QUERY` report.

use std::rc::Rc;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::{DataType, Expression};

use crate::dialects::constants::DATABRICKS_DEFAULT_DECIMAL;
use crate::dialects::main::canonical_type::canonical_type;
use crate::lineage::constants::{FLOAT_TYPES, INTEGER_TYPES, TWO_ARGUMENTS};
use crate::lineage::models::{Lineage, LineageContext};

/// Databricks' text type, which every string literal, concatenation and sized string cast has.
const STRING_TYPE: &str = "STRING";
/// The most digits a Databricks DECIMAL holds.
const MAX_DECIMAL_PRECISION: usize = 38;

/// A literal's Databricks type: none for kinds whose type is not recorded.
pub(crate) fn databricks_literal_type(literal: &Literal) -> Option<String> {
    match literal {
        Literal::Number(text) => number_type(text, false),
        Literal::String(_) => Some(STRING_TYPE.to_owned()),
        _ => None,
    }
}

/// A numeric literal's type: INT, BIGINT or DECIMAL(n,0) by size, DECIMAL(p,s), or DOUBLE.
fn number_type(text: &str, negative: bool) -> Option<String> {
    if text.contains(['e', 'E']) {
        return Some("DOUBLE".to_owned());
    }
    let (whole, fraction): (&str, Option<&str>) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (text, None),
    };
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(whole) || !fraction.is_none_or(digits) {
        return None;
    }
    let significant: &str = whole.trim_start_matches('0');
    if let Some(fraction) = fraction {
        let precision: usize = (significant.len() + fraction.len()).max(1);
        return (precision <= MAX_DECIMAL_PRECISION)
            .then(|| format!("DECIMAL({precision},{})", fraction.len()));
    }
    if significant.len() > MAX_DECIMAL_PRECISION {
        return None;
    }
    let magnitude: i128 = significant.parse::<i128>().unwrap_or(0);
    let value: i128 = if negative { -magnitude } else { magnitude };
    Some(
        if (i128::from(i32::MIN)..=i128::from(i32::MAX)).contains(&value) {
            "INT".to_owned()
        } else if (i128::from(i64::MIN)..=i128::from(i64::MAX)).contains(&value) {
            "BIGINT".to_owned()
        } else {
            format!("DECIMAL({},0)", significant.len())
        },
    )
}

/// The type a Databricks CAST to `data_type` gives, where it is recorded.
pub(crate) fn databricks_cast_spelling(data_type: &DataType) -> Option<String> {
    let text: &str = match data_type {
        DataType::Boolean => "BOOLEAN",
        DataType::TinyInt { .. } => "TINYINT",
        DataType::SmallInt { .. } => "SMALLINT",
        DataType::Int { .. } => "INT",
        DataType::BigInt { .. } => "BIGINT",
        DataType::Float { .. } => "FLOAT",
        DataType::Double { .. } => "DOUBLE",
        DataType::Decimal { precision, scale } => {
            let (precision, scale): (u32, u32) = match precision {
                Some(precision) => (*precision, scale.unwrap_or(0)),
                None => DATABRICKS_DEFAULT_DECIMAL,
            };
            return Some(format!("DECIMAL({precision},{scale})"));
        }
        DataType::String { .. } | DataType::VarChar { .. } | DataType::Char { .. } => STRING_TYPE,
        DataType::Binary { .. } => "BINARY",
        DataType::Date => "DATE",
        DataType::Timestamp { timezone: true, .. } => "TIMESTAMP",
        DataType::Timestamp {
            timezone: false, ..
        } => "TIMESTAMP_NTZ",
        _ => return None,
    };
    Some(text.to_owned())
}

/// Databricks' type for `expression`: `None` keeps the shared generic answer, `Some(None)` none.
pub(crate) fn databricks_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> Option<Option<Rc<str>>> {
    let typed = |index: usize| {
        parts
            .get(index)
            .and_then(|(_, part)| part.data_type.clone())
    };
    let is_string = |data_type: &Option<Rc<str>>| {
        data_type
            .as_deref()
            .is_some_and(|data_type| context.profile.same_type(data_type, STRING_TYPE))
    };
    match expression {
        Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::Lt(_)
        | Expression::Lte(_)
        | Expression::Gt(_)
        | Expression::Gte(_)
        | Expression::And(_)
        | Expression::Or(_)
        | Expression::Not(_)
        | Expression::In(_)
        | Expression::Between(_)
        | Expression::Like(_)
        | Expression::ILike(_)
        | Expression::Is(_)
        | Expression::Exists(_)
        | Expression::RegexpLike(_)
        | Expression::IsNull(_)
        | Expression::Count(_)
        | Expression::CountIf(_)
        | Expression::Coalesce(_)
        | Expression::Min(_)
        | Expression::Max(_)
        | Expression::Case(_)
        | Expression::IfFunc(_)
        | Expression::WithinGroup(_)
        | Expression::WindowFunction(_)
        | Expression::Pos(_) => None,
        Expression::Neg(negated) => match &negated.this {
            Expression::Literal(literal) => match literal.as_ref() {
                Literal::Number(text) => Some(number_type(text, true).map(Rc::from)),
                _ => Some(None),
            },
            _ => None,
        },
        Expression::Concat(_) | Expression::DPipe(_) => Some(
            (!parts.is_empty() && parts.iter().all(|(_, part)| is_string(&part.data_type)))
                .then(|| Rc::from(STRING_TYPE)),
        ),
        Expression::Add(_) | Expression::Sub(_) | Expression::Mul(_) => {
            let (left, right) = (typed(0), typed(1));
            Some(match (left, right) {
                (Some(left), Some(right))
                    if parts.len() == TWO_ARGUMENTS
                        && context.profile.same_type(&left, &right)
                        && (INTEGER_TYPES.contains(&canonical_type(&left).as_str())
                            || FLOAT_TYPES.contains(&canonical_type(&left).as_str())) =>
                {
                    Some(left)
                }
                _ => None,
            })
        }
        Expression::Function(function) => {
            Some(context.schema.function_type(&function.name).map(Rc::from))
        }
        Expression::AggregateFunction(function) => {
            Some(context.schema.function_type(&function.name).map(Rc::from))
        }
        _ => Some(None),
    }
}
