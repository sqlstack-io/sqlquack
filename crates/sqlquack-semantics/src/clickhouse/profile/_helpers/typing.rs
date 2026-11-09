//! ClickHouse 26.1's value types, as `toTypeName` spells them; values the rules miss stay untyped.

use std::num::ParseIntError;
use std::rc::Rc;

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;

use crate::clickhouse::profile::_helpers::type_names::clickhouse_type as canonical;
use crate::clickhouse::profile::constants::{
    CLICKHOUSE_BOOLEAN_RESULT, CLICKHOUSE_COUNT_TYPE, CLICKHOUSE_DATETIME_TYPE,
    CLICKHOUSE_DECIMAL_PREFIX, CLICKHOUSE_FIXED_RESULTS, CLICKHOUSE_FIXED_STRING_PREFIX,
    CLICKHOUSE_HEX_PREFIX, CLICKHOUSE_NULLABLE_WRAPPERS, CLICKHOUSE_STRING_TYPE,
};
use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::{Lineage, LineageContext};

/// ClickHouse's numeric types up to 64 bits, by their width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Number {
    Unsigned(u32),
    Signed(u32),
    Float(u32),
}

impl Number {
    fn of(spelling: &str) -> Option<Self> {
        let (kind, bits): (fn(u32) -> Self, &str) = match spelling {
            text if text.starts_with("UInt") => (Self::Unsigned, &text[4..]),
            text if text.starts_with("Int") => (Self::Signed, &text[3..]),
            text if text.starts_with("Float") => (Self::Float, &text[5..]),
            _ => return None,
        };
        let bits: u32 = match bits.parse() {
            Ok(bits) => bits,
            Err(_not_a_width) => return None,
        };
        [8, 16, 32, 64].contains(&bits).then(|| kind(bits))
    }

    fn spelling(self) -> String {
        match self {
            Self::Unsigned(bits) => format!("UInt{bits}"),
            Self::Signed(bits) => format!("Int{bits}"),
            Self::Float(bits) => format!("Float{bits}"),
        }
    }

    fn bits(self) -> u32 {
        match self {
            Self::Unsigned(bits) | Self::Signed(bits) | Self::Float(bits) => bits,
        }
    }
}

/// The widest integer ClickHouse widens arithmetic to.
const WIDEST: u32 = 64;
/// ClickHouse's floating type for arithmetic and averages.
const FLOAT64: Number = Number::Float(WIDEST);

/// A spelling's ClickHouse type with `Nullable` and `LowCardinality` taken off.
fn inner(spelling: &str) -> Option<String> {
    let mut text: String = canonical(spelling)?;
    while let Some(wrapper) = CLICKHOUSE_NULLABLE_WRAPPERS
        .iter()
        .find(|wrapper| text.starts_with(&format!("{wrapper}(")))
    {
        text = text[wrapper.len() + 1..text.len() - 1].to_owned();
    }
    Some(text)
}

/// The smallest type ClickHouse gives an unsigned integer literal of `value`.
fn unsigned_literal(value: u128) -> Number {
    match value {
        0..=0xFF => Number::Unsigned(8),
        0x100..=0xFFFF => Number::Unsigned(16),
        0x1_0000..=0xFFFF_FFFF => Number::Unsigned(32),
        0x1_0000_0000..=0xFFFF_FFFF_FFFF_FFFF => Number::Unsigned(WIDEST),
        _ => FLOAT64,
    }
}

/// The smallest type ClickHouse gives the literal `-value`.
fn negative_literal(value: u128) -> Number {
    match value {
        0..=0x80 => Number::Signed(8),
        0x81..=0x8000 => Number::Signed(16),
        0x8001..=0x8000_0000 => Number::Signed(32),
        0x8000_0001..=0x8000_0000_0000_0000 => Number::Signed(WIDEST),
        _ => FLOAT64,
    }
}

/// A number literal's integer value, or `None` for a float or a value too large to read.
fn integer_value(text: &str) -> Option<Option<u128>> {
    let digits: String = text.chars().filter(|character| *character != '_').collect();
    let read: Result<u128, ParseIntError> = match digits
        .strip_prefix(CLICKHOUSE_HEX_PREFIX)
        .or_else(|| digits.strip_prefix("0X"))
    {
        Some(hex) => u128::from_str_radix(hex, 16),
        None if digits.chars().any(|c| !c.is_ascii_digit()) => return None,
        None => digits.parse::<u128>(),
    };
    match read {
        Ok(value) => Some(Some(value)),
        Err(_too_large) => Some(None),
    }
}

/// The type of a number literal written `text`, negated when `negative`.
fn number_literal(text: &str, negative: bool) -> Option<Number> {
    match (integer_value(text), negative) {
        (None, _) => text
            .chars()
            .all(|c| c.is_ascii_digit() || ".eE+-_".contains(c))
            .then_some(FLOAT64),
        (Some(None), _) => Some(FLOAT64),
        (Some(Some(value)), false) => Some(unsigned_literal(value)),
        (Some(Some(value)), true) => Some(negative_literal(value)),
    }
}

/// A literal's ClickHouse type; NULL stays untyped, as a set operation's NULL branch must.
pub(crate) fn clickhouse_literal_type(expression: &Expression) -> Option<String> {
    match expression {
        Expression::Boolean(_) => Some("Bool".to_owned()),
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(text) => number_literal(text, false).map(Number::spelling),
            Literal::String(_) => Some(CLICKHOUSE_STRING_TYPE.to_owned()),
            _ => None,
        },
        _ => None,
    }
}

/// The ClickHouse type of `child` as its lineage gives it, without `Nullable`.
fn operand(child: &Expression, parts: &[(&Expression, Lineage)]) -> Option<String> {
    let spelled: Rc<str> = parts
        .iter()
        .find(|(candidate, _)| std::ptr::eq(*candidate, child))
        .and_then(|(_, part)| part.data_type.clone())?;
    inner(&spelled)
}

/// `toStartOfDay` and `toStartOfHour` of a date or time: a DateTime in the argument's time zone.
fn start_of_day(argument: &str) -> Option<String> {
    let arguments: &str = match argument {
        "Date" | "Date32" | CLICKHOUSE_DATETIME_TYPE => {
            return Some(CLICKHOUSE_DATETIME_TYPE.to_owned());
        }
        zoned if zoned.starts_with("DateTime(") => return Some(zoned.to_owned()),
        other => other.strip_prefix("DateTime64(")?.strip_suffix(')')?,
    };
    match arguments.split_once(", ") {
        Some((_, zone)) => Some(format!("{CLICKHOUSE_DATETIME_TYPE}({zone})")),
        None => Some(CLICKHOUSE_DATETIME_TYPE.to_owned()),
    }
}

/// Whether one operand is a float and the other a Decimal, which ClickHouse computes in Float64.
fn float_with_decimal(left: &str, right: &str) -> bool {
    let float = |spelling: &str| matches!(Number::of(spelling), Some(Number::Float(_)));
    let decimal = |spelling: &str| spelling.starts_with(CLICKHOUSE_DECIMAL_PREFIX);
    (float(left) && decimal(right)) || (decimal(left) && float(right))
}

/// `+`, `-` and `*` of two numbers: Float64 with a float, else the next wider integer.
fn arithmetic(left: Number, right: Number, subtract: bool) -> Number {
    match (left, right) {
        (Number::Float(_), _) | (_, Number::Float(_)) => FLOAT64,
        _ => {
            let bits: u32 = (left.bits().max(right.bits()) * 2).min(WIDEST);
            let signed: bool =
                subtract || matches!(left, Number::Signed(_)) || matches!(right, Number::Signed(_));
            match signed {
                true => Number::Signed(bits),
                false => Number::Unsigned(bits),
            }
        }
    }
}

/// The type ClickHouse combines values of `types` into: one type as it is, else a numeric supertype.
pub(crate) fn clickhouse_common_type(spellings: &[String]) -> Option<String> {
    let types: Vec<String> = spellings
        .iter()
        .map(|spelling| inner(spelling))
        .collect::<Option<_>>()?;
    let first: &String = types.first()?;
    if types.iter().all(|spelling| spelling == first) {
        return Some(first.clone());
    }
    let numbers: Vec<Number> = types
        .iter()
        .map(|spelling| Number::of(spelling))
        .collect::<Option<_>>()?;
    let widest = |pick: fn(Number) -> Option<u32>| numbers.iter().filter_map(|n| pick(*n)).max();
    let unsigned: Option<u32> = widest(|n| matches!(n, Number::Unsigned(_)).then(|| n.bits()));
    let signed: Option<u32> = widest(|n| matches!(n, Number::Signed(_)).then(|| n.bits()));
    let float: Option<u32> = widest(|n| matches!(n, Number::Float(_)).then(|| n.bits()));
    let number: Number = match (unsigned, signed, float) {
        (_, _, Some(WIDEST)) => FLOAT64,
        (None, None, Some(bits)) => Number::Float(bits),
        (_, _, Some(_)) => return None,
        (Some(bits), None, None) => Number::Unsigned(bits),
        (None, Some(bits), None) => Number::Signed(bits),
        (Some(unsigned), Some(signed), None) if unsigned < signed => Number::Signed(signed),
        (Some(unsigned), Some(_), None) if unsigned < WIDEST => Number::Signed(unsigned * 2),
        _ => return None,
    };
    Some(number.spelling())
}

/// The common type of `children` as their lineage gives them; untyped when any is untyped.
fn common(children: &[&Expression], parts: &[(&Expression, Lineage)]) -> Option<String> {
    let types: Vec<String> = children
        .iter()
        .map(|child| operand(child, parts))
        .collect::<Option<_>>()?;
    clickhouse_common_type(&types)
}

/// The type of computed `expression` from its children's lineage `parts`; `None` leaves it untyped.
pub(crate) fn clickhouse_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
    context: LineageContext<'_>,
) -> Option<Rc<str>> {
    let number = |child: &Expression| Number::of(&operand(child, parts)?);
    let spelling: String = match expression {
        Expression::Function(function) => {
            if let Some(declared) = context.schema.function_type(&function.name) {
                return inner(declared).map(Rc::from);
            }
            let arguments: Vec<&Expression> = function.args.iter().collect();
            function_type(&function.name, &arguments, parts)?
        }
        Expression::AggregateFunction(function) => {
            let arguments: Vec<&Expression> = function.args.iter().collect();
            function_type(&function.name, &arguments, parts)?
        }
        Expression::WindowFunction(window) => operand(&window.this, parts)?,
        Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::NullSafeEq(_)
        | Expression::NullSafeNeq(_)
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
        | Expression::IsNull(_)
        | Expression::Exists(_) => CLICKHOUSE_BOOLEAN_RESULT.to_owned(),
        Expression::Neg(unary) => match &unary.this {
            Expression::Literal(literal) => match literal.as_ref() {
                Literal::Number(text) => number_literal(text, true)?.spelling(),
                _ => return None,
            },
            child => match number(child)? {
                Number::Unsigned(bits) => Number::Signed((bits * 2).min(WIDEST)).spelling(),
                other => other.spelling(),
            },
        },
        Expression::Add(operation)
        | Expression::Sub(operation)
        | Expression::Mul(operation)
        | Expression::Div(operation)
            if float_with_decimal(
                &operand(&operation.left, parts)?,
                &operand(&operation.right, parts)?,
            ) =>
        {
            FLOAT64.spelling()
        }
        Expression::Add(operation) | Expression::Mul(operation) => {
            arithmetic(number(&operation.left)?, number(&operation.right)?, false).spelling()
        }
        Expression::Sub(operation) => {
            arithmetic(number(&operation.left)?, number(&operation.right)?, true).spelling()
        }
        Expression::Div(operation) => {
            number(&operation.left)?;
            number(&operation.right)?;
            FLOAT64.spelling()
        }
        Expression::Concat(_) => CLICKHOUSE_STRING_TYPE.to_owned(),
        Expression::Paren(paren) => operand(&paren.this, parts)?,
        Expression::Cast(cast) => inner(&rendered_type(&cast.to)?)?,
        Expression::Case(case) => {
            let mut values: Vec<&Expression> = case.whens.iter().map(|(_, value)| value).collect();
            values.extend(case.else_.iter());
            common(&values, parts)?
        }
        _ => return None,
    };
    Some(Rc::from(spelling))
}

/// `coalesce` and `ifNull`: the arguments' one type, or the first's when the rest are number literals.
fn coalesced(arguments: &[&Expression], parts: &[(&Expression, Lineage)]) -> Option<String> {
    let (first, rest): (&&Expression, &[&Expression]) = arguments.split_first()?;
    let first_type: String = operand(first, parts)?;
    let literal_rest: bool = rest.iter().all(|argument| {
        matches!(argument, Expression::Literal(literal) if matches!(literal.as_ref(), Literal::Number(_)))
    });
    match literal_rest && Number::of(&first_type).is_some() {
        true => Some(first_type),
        false => {
            let types: Vec<String> = arguments
                .iter()
                .map(|argument| operand(argument, parts))
                .collect::<Option<_>>()?;
            types
                .iter()
                .all(|spelling| *spelling == first_type)
                .then_some(first_type)
        }
    }
}

/// A built-in function's result type, for the functions SQLBuild's ClickHouse models call.
fn function_type(
    name: &str,
    arguments: &[&Expression],
    parts: &[(&Expression, Lineage)],
) -> Option<String> {
    if let Some((_, result)) = CLICKHOUSE_FIXED_RESULTS
        .iter()
        .find(|(function, _)| function.eq_ignore_ascii_case(name))
    {
        return Some((*result).to_owned());
    }
    let first = || operand(arguments.first()?, parts);
    let number = || Number::of(&first()?);
    match name.to_ascii_lowercase().as_str() {
        "count" | "countdistinct" | "uniq" | "uniqexact" | "row_number" | "rank" | "dense_rank" => {
            Some(CLICKHOUSE_COUNT_TYPE.to_owned())
        }
        "min" | "max" | "any" | "anylast" | "argmin" | "argmax" | "nullif" | "first_value"
        | "last_value" | "laginframe" | "leadinframe" => first(),
        "now" | "todatetime" if arguments.len() <= 1 => Some(CLICKHOUSE_DATETIME_TYPE.to_owned()),
        "sum" => Some(
            match number()? {
                Number::Unsigned(_) => Number::Unsigned(WIDEST),
                Number::Signed(_) => Number::Signed(WIDEST),
                Number::Float(_) => FLOAT64,
            }
            .spelling(),
        ),
        "avg" => number().map(|_| FLOAT64.spelling()),
        "round" => number().map(Number::spelling),
        "abs" => Some(
            match number()? {
                Number::Signed(bits) => Number::Unsigned(bits),
                other => other,
            }
            .spelling(),
        ),
        "if" => common(arguments.get(1..)?, parts),
        "upper" | "lower" => first().filter(|text| {
            text == CLICKHOUSE_STRING_TYPE || text.starts_with(CLICKHOUSE_FIXED_STRING_PREFIX)
        }),
        "tostartofday" | "tostartofhour" if arguments.len() == 1 => start_of_day(&first()?),
        "todecimal32" | "todecimal64" | "todecimal128" | "todecimal256" => {
            match arguments.get(1)? {
                Expression::Literal(literal) => match literal.as_ref() {
                    Literal::Number(scale) => canonical(&format!("{}({scale})", name.get(2..)?)),
                    _ => None,
                },
                _ => None,
            }
        }
        "coalesce" | "ifnull" => coalesced(arguments, parts),
        _ => None,
    }
}
