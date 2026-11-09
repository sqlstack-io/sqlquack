//! SQL Server's documented result types where they differ from the generic rules.

use std::rc::Rc;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::{DataType, Expression};

use crate::lineage::models::Lineage;

const NVARCHAR: &str = "NVARCHAR";
const VARCHAR: &str = "VARCHAR";
const INT: &str = "INT";
const BIGINT: &str = "BIGINT";
const FLOAT: &str = "FLOAT";
const MONEY: &str = "MONEY";
const REAL: &str = "REAL";
/// The largest integer literal SQL Server types as int; past it a literal is numeric(digits, 0).
const INT_MAX: u128 = 2_147_483_647;
/// The largest `FLOAT(n)` SQL Server stores as real.
const REAL_FLOAT_PRECISION: u32 = 24;
/// Functions returning a character argument's varying type, and varchar for any other type.
const SAME_STRING_FUNCTIONS: [&str; 5] = ["LOWER", "LTRIM", "REVERSE", "RTRIM", "UPPER"];
/// SQL Server's Unicode character types.
const UNICODE_TYPES: [&str; 3] = ["NCHAR", "NTEXT", "NVARCHAR"];
/// The most digits an int literal has.
const INT_DIGITS: usize = 10;
/// The length of a CAST target CHAR written without one.
const CAST_CHAR_LENGTH: u32 = 30;
/// String functions whose result is nvarchar when any argument is a Unicode string.
const UNICODE_STRING_FUNCTIONS: [&str; 7] = [
    "CONCAT",
    "CONCAT_WS",
    "LEFT",
    "REPLACE",
    "RIGHT",
    "STRING_AGG",
    "SUBSTRING",
];

/// The type SQL Server gives a literal, when it differs from the generic rules.
pub(crate) fn sqlserver_literal_type(literal: &Literal) -> Option<String> {
    match literal {
        Literal::NationalString(_) => Some(NVARCHAR.to_owned()),
        Literal::Number(text) if text.contains(['e', 'E']) => Some(FLOAT.to_owned()),
        Literal::Number(text) => past_int(text).map(|digits| format!("DECIMAL({digits},0)")),
        _ => None,
    }
}

/// A character type's varying form, `CHAR(5)` as `VARCHAR(5)`; `None` for other types.
fn varying_string(spelled: &str) -> Option<Rc<str>> {
    let upper: String = spelled.trim().to_ascii_uppercase();
    let base: &str = upper.split('(').next().unwrap_or_default().trim();
    let size: &str = &upper[base.len()..];
    match base {
        "VARCHAR" | "NVARCHAR" => Some(Rc::from(upper.as_str())),
        "CHAR" | "CHARACTER" => Some(Rc::from(format!("{VARCHAR}{size}"))),
        "NCHAR" => Some(Rc::from(format!("{NVARCHAR}{size}"))),
        _ => None,
    }
}

/// A type spelling's name without its modifiers, upper case.
fn base_type(spelled: &str) -> String {
    spelled
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_uppercase()
}

/// The significant digits of an integer literal past the int range.
fn past_int(text: &str) -> Option<usize> {
    let digits: &str = text.trim_start_matches('0');
    let integer: bool = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let past: bool =
        digits.len() > INT_DIGITS || digits.parse::<u128>().is_ok_and(|value| value > INT_MAX);
    (integer && past).then_some(digits.len())
}

/// The type of a CAST target as SQL Server stores it, where it differs from the generic spelling.
pub(crate) fn sqlserver_cast_spelling(data_type: &DataType) -> Option<Option<String>> {
    match data_type {
        DataType::Float { .. } => Some(Some(REAL.to_owned())),
        DataType::Double {
            precision: Some(precision),
            ..
        } if *precision <= REAL_FLOAT_PRECISION => Some(Some(REAL.to_owned())),
        DataType::Double { .. } => Some(Some(FLOAT.to_owned())),
        DataType::Char {
            length: Some(length),
        } => Some(Some(format!("CHAR({length})"))),
        DataType::Char { length: None } => Some(Some(format!("CHAR({CAST_CHAR_LENGTH})"))),
        _ => None,
    }
}

/// The result types SQL Server documents for counts, string functions, ROUND, SUM and -2147483648.
pub(crate) fn sqlserver_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
) -> Option<Option<Rc<str>>> {
    if let Expression::Neg(negated) = expression
        && let Expression::Literal(literal) = &negated.this
        && let Literal::Number(text) = literal.as_ref()
        && text.trim_start_matches('0').parse::<u128>() == Ok(INT_MAX + 1)
    {
        return Some(Some(Rc::from(INT)));
    }
    if let Expression::Trim(trim) = expression {
        let trimmed: Option<Rc<str>> = parts
            .iter()
            .find(|(part, _)| std::ptr::eq(*part, &trim.this))
            .and_then(|(_, lineage)| lineage.data_type.clone());
        return Some(trimmed.and_then(|spelled| varying_string(&spelled)));
    }
    let called: Option<(&str, &[Expression])> = match expression {
        Expression::Function(function) if !function.quoted => {
            Some((function.name.as_str(), function.args.as_slice()))
        }
        Expression::AggregateFunction(function) => {
            Some((function.name.as_str(), function.args.as_slice()))
        }
        _ => None,
    };
    if let Some((name, arguments)) = called {
        let upper: String = name.to_ascii_uppercase();
        let typed = |argument: &Expression| {
            parts
                .iter()
                .find(|(part, _)| std::ptr::eq(*part, argument))
                .and_then(|(_, lineage)| lineage.data_type.clone())
        };
        if SAME_STRING_FUNCTIONS.contains(&upper.as_str()) {
            return Some(
                arguments
                    .first()
                    .and_then(typed)
                    .map(|spelled| varying_string(&spelled).unwrap_or_else(|| Rc::from(VARCHAR))),
            );
        }
        let unicode: bool = arguments
            .iter()
            .filter_map(typed)
            .any(|spelled| UNICODE_TYPES.contains(&base_type(&spelled).as_str()));
        if unicode && UNICODE_STRING_FUNCTIONS.contains(&upper.as_str()) {
            return Some(Some(Rc::from(NVARCHAR)));
        }
    }
    let (name, first): (&str, &Expression) = match expression {
        Expression::Function(function) if !function.quoted => {
            (function.name.as_str(), function.args.first()?)
        }
        Expression::AggregateFunction(function) => {
            match function.name.to_ascii_uppercase().as_str() {
                "COUNT" => return Some(Some(Rc::from(INT))),
                "COUNT_BIG" => return Some(Some(Rc::from(BIGINT))),
                _ => (function.name.as_str(), function.args.first()?),
            }
        }
        _ => return None,
    };
    let spelled: Rc<str> = parts
        .iter()
        .find(|(part, _)| std::ptr::eq(*part, first))
        .and_then(|(_, lineage)| lineage.data_type.clone())?;
    let base: String = spelled
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_uppercase();
    let widened: Option<&str> = match base.as_str() {
        "TINYINT" | "SMALLINT" | "INT" | "INTEGER" => Some(INT),
        "BIGINT" => Some(BIGINT),
        "FLOAT" | "REAL" | "DOUBLE" | "DOUBLE PRECISION" => Some(FLOAT),
        "MONEY" | "SMALLMONEY" => Some(MONEY),
        _ => None,
    };
    let decimal: bool = matches!(base.as_str(), "DECIMAL" | "NUMERIC");
    match name.to_ascii_uppercase().as_str() {
        "ROUND" if decimal => Some(Some(spelled)),
        "ROUND" | "SUM" => widened.map(|data_type| Some(Rc::from(data_type))),
        _ => None,
    }
}
