//! DuckDB's result types, as the engine binds them; no type where the engine's is not modelled.

use std::rc::Rc;

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::typed_functions::models::DateTimeField;

use crate::lineage::_helpers::typing::duckdb_types::{
    DuckType, Number, common_type, decimal_shape, parsed_type, unsigned_coercion,
};
use crate::lineage::constants::{
    DUCKDB_BIGINT_BITS, DUCKDB_DECIMAL_LIMIT, DUCKDB_EPOCH_PART, DUCKDB_INTEGER_BITS,
    DUCKDB_JSON_TYPES, DUCKDB_JULIAN_PART, DUCKDB_SMALL_DECIMAL, DUCKDB_WIDEST_INTEGER,
    ONE_ARGUMENT, TWO_ARGUMENTS,
};
use crate::lineage::models::Lineage;

/// What a rule answers: `None` when no rule owns the node, else its type, which may be unknown.
type Owned = Option<Option<DuckType>>;

/// The arithmetic operators the rules distinguish.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    IntegerDiv,
}

/// The children of the node being typed, with their lineage.
struct Operands<'p, 'e> {
    parts: &'p [(&'e Expression, Lineage)],
}

impl Operands<'_, '_> {
    /// The lineage type spelling of `child`, a child of the node.
    fn spelled(&self, child: &Expression) -> Option<Rc<str>> {
        self.parts
            .iter()
            .find(|(part, _)| std::ptr::eq(*part, child))
            .and_then(|(_, lineage)| lineage.data_type.clone())
    }

    /// The type of `child`, when its spelling is one the rules model.
    fn typed(&self, child: &Expression) -> Option<DuckType> {
        parsed_type(&self.spelled(child)?)
    }
}

/// The DuckDB type of a literal, when the engine's is modelled.
pub(crate) fn duckdb_literal_type(literal: &Literal) -> Option<DuckType> {
    match literal {
        Literal::String(_)
        | Literal::NationalString(_)
        | Literal::EscapeString(_)
        | Literal::DollarString(_)
        | Literal::TripleQuotedString(_, _)
        | Literal::RawString(_) => Some(DuckType::Text),
        Literal::Number(text) => number_type(text),
        Literal::Date(_) => Some(DuckType::Date),
        Literal::Time(_) => Some(DuckType::Time(false)),
        Literal::Timestamp(_) | Literal::Datetime(_) => Some(DuckType::Timestamp(false)),
        _ => None,
    }
}

/// A number literal: DOUBLE with an exponent, an exact DECIMAL with a point, else an integer.
fn number_type(text: &str) -> Option<DuckType> {
    if text.contains(['e', 'E']) {
        return Some(DuckType::Double);
    }
    if let Some((whole, fraction)) = text.split_once('.') {
        if !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let (Ok(scale), Ok(precision)) = (
            u32::try_from(fraction.len()),
            u32::try_from(whole.len() + fraction.len()),
        ) else {
            return None;
        };
        return (precision <= DUCKDB_DECIMAL_LIMIT)
            .then(|| DuckType::Decimal(precision.max(scale).max(1), scale));
    }
    match text.parse::<i128>() {
        Ok(value) => Some(integer_literal_type(value)),
        Err(_not_an_integer) => None,
    }
}

fn integer_literal_type(value: i128) -> DuckType {
    [DUCKDB_INTEGER_BITS, DUCKDB_BIGINT_BITS]
        .into_iter()
        .find(|bits| fits(value, *bits, false))
        .map_or(DuckType::Integer(DUCKDB_WIDEST_INTEGER, false), |bits| {
            DuckType::Integer(bits, false)
        })
}

/// The DuckDB type of `expression`, when a rule owns it; its children's lineage in `parts`.
pub(crate) fn duckdb_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
) -> Option<Option<Rc<str>>> {
    let operands = Operands { parts };
    if let Some(same) = passed_through(expression, &operands) {
        return Some(same);
    }
    let owned: Owned = operator_type(expression, &operands)
        .or_else(|| aggregate_type(expression, &operands))
        .or_else(|| scalar_type(expression, &operands))
        .or_else(|| match expression {
            Expression::Function(function) if !function.quoted => {
                named_type(&function.name, &function.args, &operands)
            }
            _ => None,
        });
    owned.map(spelled)
}

fn spelled(data_type: Option<DuckType>) -> Option<Rc<str>> {
    data_type.map(|data_type| Rc::from(data_type.spelling()))
}

/// Rules that keep their operand's type as spelled: MIN, MAX, ABS, LAG, LEAD and NULLIF.
fn passed_through(expression: &Expression, operands: &Operands<'_, '_>) -> Option<Option<Rc<str>>> {
    let (operand, numeric): (&Expression, bool) = match expression {
        Expression::Min(aggregate) | Expression::Max(aggregate) => (&aggregate.this, false),
        Expression::Lag(function) | Expression::Lead(function) => (&function.this, false),
        Expression::NullIf(function) => (&function.this, false),
        Expression::Abs(function) => (&function.this, true),
        Expression::Function(function) if !function.quoted => {
            let name: String = function.name.to_ascii_uppercase();
            let first: &Expression = function.args.first()?;
            match name.as_str() {
                "MIN" | "MAX" if function.args.len() == ONE_ARGUMENT => (first, false),
                "LAG" | "LEAD" | "FIRST_VALUE" | "LAST_VALUE" => (first, false),
                "NULLIF" if function.args.len() == TWO_ARGUMENTS => (first, false),
                "ABS" => (first, true),
                _ => return None,
            }
        }
        _ => return None,
    };
    let spelled: Option<Rc<str>> = operands.spelled(operand);
    let parsed: Option<DuckType> = spelled.as_deref().and_then(parsed_type);
    if numeric && parsed.as_ref().and_then(DuckType::number).is_none() {
        return Some(None);
    }
    Some(match parsed {
        Some(DuckType::Decimal(..)) if !spelled.as_deref()?.contains('(') => {
            parsed.map(|parsed| Rc::from(parsed.spelling()))
        }
        _ => spelled,
    })
}

fn operator_type(expression: &Expression, operands: &Operands<'_, '_>) -> Owned {
    let arithmetic = |operator: Operator, left: &Expression, right: &Expression| {
        Some(arithmetic_type(operator, (left, right), operands))
    };
    match expression {
        Expression::Add(operation) => arithmetic(Operator::Add, &operation.left, &operation.right),
        Expression::Sub(operation) => arithmetic(Operator::Sub, &operation.left, &operation.right),
        Expression::Mul(operation) => arithmetic(Operator::Mul, &operation.left, &operation.right),
        Expression::Div(operation) => arithmetic(Operator::Div, &operation.left, &operation.right),
        Expression::Mod(operation) => arithmetic(Operator::Mod, &operation.left, &operation.right),
        Expression::ModFunc(function) => {
            arithmetic(Operator::Mod, &function.this, &function.expression)
        }
        Expression::IntDiv(function) => {
            arithmetic(Operator::IntegerDiv, &function.this, &function.expression)
        }
        Expression::Neg(operation) => Some(
            operands
                .typed(&operation.this)
                .filter(|operand| operand.number().is_some() || *operand == DuckType::Interval),
        ),
        Expression::Concat(operation) => {
            Some(concat_type(&[&operation.left, &operation.right], operands))
        }
        Expression::NullSafeEq(_) | Expression::NullSafeNeq(_) => Some(Some(DuckType::Boolean)),
        Expression::Count(_) => Some(Some(DuckType::Integer(64, false))),
        Expression::Case(case) => Some(unified(
            case.whens
                .iter()
                .map(|(_, result)| result)
                .chain(case.else_.iter()),
            operands,
        )),
        Expression::Coalesce(function)
        | Expression::Greatest(function)
        | Expression::Least(function) => Some(unified(function.expressions.iter(), operands)),
        Expression::IfNull(function) | Expression::Nvl(function) => {
            Some(unified([&function.this, &function.expression], operands))
        }
        _ => None,
    }
}

fn aggregate_type(expression: &Expression, operands: &Operands<'_, '_>) -> Owned {
    match expression {
        Expression::Sum(aggregate) => Some(sum_type(operands.typed(&aggregate.this))),
        Expression::Avg(aggregate) => Some(average_type(operands.typed(&aggregate.this))),
        Expression::Stddev(aggregate)
        | Expression::StddevPop(aggregate)
        | Expression::StddevSamp(aggregate)
        | Expression::Variance(aggregate)
        | Expression::VarPop(aggregate)
        | Expression::VarSamp(aggregate) => Some(statistic_type(operands.typed(&aggregate.this))),
        Expression::LogicalAnd(_) | Expression::LogicalOr(_) => Some(Some(DuckType::Boolean)),
        Expression::StringAgg(aggregate) => Some(string_result(Some(&aggregate.this), operands)),
        Expression::RowNumber(_) | Expression::Rank(_) | Expression::DenseRank(_) => {
            Some(Some(DuckType::Integer(64, false)))
        }
        _ => None,
    }
}

fn scalar_type(expression: &Expression, operands: &Operands<'_, '_>) -> Owned {
    let text = Some(Some(DuckType::Text));
    match expression {
        Expression::Round(function) => Some(round_type(
            operands.typed(&function.this),
            function.decimals.as_ref(),
        )),
        Expression::Floor(function) if function.scale.is_none() && function.to.is_none() => {
            Some(floor_type(operands.typed(&function.this)))
        }
        Expression::Ceil(function) if function.decimals.is_none() && function.to.is_none() => {
            Some(floor_type(operands.typed(&function.this)))
        }
        Expression::Floor(_) | Expression::Ceil(_) => Some(None),
        Expression::Sign(function) => Some(
            operands
                .typed(&function.this)
                .and_then(|operand| operand.number())
                .map(|_| DuckType::Integer(8, false)),
        ),
        Expression::Sqrt(function) | Expression::Ln(function) | Expression::Exp(function) => {
            Some(transcendental_type(&[&function.this], operands))
        }
        Expression::Power(function) => Some(transcendental_type(
            &[&function.this, &function.expression],
            operands,
        )),
        Expression::Length(_) | Expression::StrPosition(_) => {
            Some(Some(DuckType::Integer(64, false)))
        }
        Expression::Upper(function)
        | Expression::Lower(function)
        | Expression::LTrim(function)
        | Expression::RTrim(function)
        | Expression::Reverse(function)
        | Expression::Initcap(function) => Some(string_result(Some(&function.this), operands)),
        Expression::Trim(function) => Some(string_result(Some(&function.this), operands)),
        Expression::Substring(function) => Some(string_result(Some(&function.this), operands)),
        Expression::Replace(function) => Some(string_result(Some(&function.this), operands)),
        Expression::Left(_)
        | Expression::Right(_)
        | Expression::Repeat(_)
        | Expression::Lpad(_)
        | Expression::Rpad(_)
        | Expression::ConcatWs(_) => text,
        Expression::Random(_) | Expression::Epoch(_) => Some(Some(DuckType::Double)),
        Expression::DateTrunc(function) | Expression::TimestampTrunc(function) => {
            Some(truncated_type(operands.typed(&function.this)))
        }
        Expression::CurrentDate(_) => Some(Some(DuckType::Date)),
        Expression::CurrentTime(_) => Some(Some(DuckType::Time(true))),
        Expression::CurrentTimestamp(_) => Some(Some(DuckType::Timestamp(true))),
        Expression::Localtime(_) => Some(Some(DuckType::Time(false))),
        Expression::Localtimestamp(_) => Some(Some(DuckType::Timestamp(false))),
        Expression::CurrentUser(_) | Expression::SessionUser(_) | Expression::CurrentSchema(_) => {
            text
        }
        Expression::Year(_)
        | Expression::Month(_)
        | Expression::Day(_)
        | Expression::Hour(_)
        | Expression::Minute(_)
        | Expression::Second(_) => Some(Some(DuckType::Integer(64, false))),
        Expression::Uuid(_) => Some(Some(DuckType::Uuid)),
        Expression::Extract(function) => Some(extracted_type(&function.field)),
        Expression::Interval(_) => Some(Some(DuckType::Interval)),
        _ => None,
    }
}

/// EXTRACT's type: DOUBLE for the epoch and the Julian day, BIGINT for every numbered part.
fn extracted_type(field: &DateTimeField) -> Option<DuckType> {
    match field {
        DateTimeField::Epoch => Some(DuckType::Double),
        DateTimeField::Custom(name) if name.eq_ignore_ascii_case(DUCKDB_JULIAN_PART) => {
            Some(DuckType::Double)
        }
        DateTimeField::Date | DateTimeField::Time | DateTimeField::Custom(_) => None,
        _ => Some(DuckType::Integer(64, false)),
    }
}

/// A function written by name, when DuckDB's rules know the name.
fn named_type(name: &str, arguments: &[Expression], operands: &Operands<'_, '_>) -> Owned {
    let first: Option<&Expression> = arguments.first();
    let first_type = || first.and_then(|first| operands.typed(first));
    let bigint = Some(Some(DuckType::Integer(64, false)));
    let text = Some(Some(DuckType::Text));
    match name.to_ascii_uppercase().as_str() {
        "COUNT" | "ROW_NUMBER" | "RANK" | "DENSE_RANK" => bigint,
        "LENGTH" | "CHAR_LENGTH" | "CHARACTER_LENGTH" | "STRPOS" => bigint,
        "YEAR" | "MONTH" | "DAY" | "HOUR" | "MINUTE" | "SECOND" => bigint,
        "SUM" => Some(first.and_then(|_| sum_type(first_type()))),
        "AVG" => Some(first.and_then(|_| average_type(first_type()))),
        "STDDEV" | "STDDEV_SAMP" | "STDDEV_POP" | "VARIANCE" | "VAR_SAMP" | "VAR_POP" => {
            Some(first.and_then(|_| statistic_type(first_type())))
        }
        "BOOL_AND" | "BOOL_OR" | "EVERY" => Some(Some(DuckType::Boolean)),
        "ROUND" if arguments.len() <= TWO_ARGUMENTS => {
            Some(first.and_then(|_| round_type(first_type(), arguments.get(1))))
        }
        "FLOOR" | "CEIL" | "CEILING" if arguments.len() <= ONE_ARGUMENT => {
            Some(first.and_then(|_| floor_type(first_type())))
        }
        "SIGN" => Some(
            first_type()
                .and_then(|operand| operand.number())
                .map(|_| DuckType::Integer(8, false)),
        ),
        "SQRT" | "LN" | "EXP" | "POWER" | "POW" if !arguments.is_empty() => {
            let arguments: Vec<&Expression> = arguments.iter().collect();
            Some(transcendental_type(&arguments, operands))
        }
        "MOD" if arguments.len() == TWO_ARGUMENTS => Some(arithmetic_type(
            Operator::Mod,
            (&arguments[0], &arguments[1]),
            operands,
        )),
        "UPPER" | "LOWER" | "TRIM" | "LTRIM" | "RTRIM" | "REPLACE" | "SUBSTRING" | "SUBSTR"
        | "LEFT" | "RIGHT" | "REVERSE" | "REPEAT" | "LPAD" | "RPAD" | "INITCAP" | "MD5"
        | "STRING_AGG" => Some(string_result(first, operands)),
        "CONCAT" => {
            let arguments: Vec<&Expression> = arguments.iter().collect();
            Some(concat_type(&arguments, operands))
        }
        "CONCAT_WS" | "STRFTIME" | "DAYNAME" | "MONTHNAME" => text,
        "NOW" | "CURRENT_TIMESTAMP" | "TRANSACTION_TIMESTAMP" | "GET_CURRENT_TIMESTAMP" => {
            Some(Some(DuckType::Timestamp(true)))
        }
        "CURRENT_TIME" => Some(Some(DuckType::Time(true))),
        "LOCALTIME" => Some(Some(DuckType::Time(false))),
        "LOCALTIMESTAMP" => Some(Some(DuckType::Timestamp(false))),
        "CURRENT_DATE" | "TODAY" => Some(Some(DuckType::Date)),
        "USER" | "CURRENT_USER" | "SESSION_USER" | "CURRENT_ROLE" | "CURRENT_SCHEMA"
        | "CURRENT_CATALOG" | "CURRENT_DATABASE"
            if arguments.is_empty() =>
        {
            text
        }
        "GEN_RANDOM_UUID" | "UUID" => Some(Some(DuckType::Uuid)),
        "RANDOM" | "EPOCH" => Some(Some(DuckType::Double)),
        "DATE_TRUNC" if arguments.len() == TWO_ARGUMENTS => {
            Some(truncated_type(operands.typed(&arguments[1])))
        }
        "DATE_PART" if arguments.len() == TWO_ARGUMENTS => Some(part_type(&arguments[0])),
        "COALESCE" | "GREATEST" | "LEAST" | "IFNULL" => Some(unified(arguments.iter(), operands)),
        "LIST_VALUE" | "LIST_PACK" => Some(
            unified(arguments.iter(), operands).map(|element| DuckType::Array(Box::new(element))),
        ),
        _ => None,
    }
}

/// The value of an integer literal, negated or in parentheses.
fn integer_literal(expression: &Expression) -> Option<i128> {
    match unwrapped(expression) {
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(text)
                if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) =>
            {
                let parsed: Result<i128, _> = text.parse();
                parsed.into_iter().next()
            }
            _ => None,
        },
        Expression::Neg(operation) => integer_literal(&operation.this).map(|value| -value),
        _ => None,
    }
}

fn unwrapped(mut expression: &Expression) -> &Expression {
    loop {
        expression = match expression {
            Expression::Paren(paren) => &paren.this,
            Expression::Annotated(annotated) => &annotated.this,
            _ => return expression,
        };
    }
}

fn is_string_literal(expression: &Expression) -> bool {
    matches!(unwrapped(expression), Expression::Literal(literal) if literal.is_string())
}

fn is_null(expression: &Expression) -> bool {
    matches!(unwrapped(expression), Expression::Null(_))
}

/// Whether `value` fits an integer of `bits` bits.
fn fits(value: i128, bits: u16, unsigned: bool) -> bool {
    if unsigned {
        value >= 0 && (bits >= DUCKDB_WIDEST_INTEGER || value < (1i128 << bits))
    } else if bits >= DUCKDB_WIDEST_INTEGER {
        true
    } else {
        let limit: i128 = 1i128 << (bits - 1);
        value >= -limit && value < limit
    }
}

fn arithmetic_type(
    operator: Operator,
    (left, right): (&Expression, &Expression),
    operands: &Operands<'_, '_>,
) -> Option<DuckType> {
    let left_type: DuckType = operands.typed(left)?;
    let right_type: DuckType = operands.typed(right)?;
    match (left_type.number(), right_type.number()) {
        (Some(left_number), Some(right_number)) => numeric_arithmetic(
            operator,
            (left_number, &left_type, integer_literal(left)),
            (right_number, &right_type, integer_literal(right)),
        ),
        _ => temporal_arithmetic(operator, &left_type, &right_type),
    }
}

fn numeric_arithmetic(
    operator: Operator,
    (left, left_type, left_literal): (Number, &DuckType, Option<i128>),
    (right, right_type, right_literal): (Number, &DuckType, Option<i128>),
) -> Option<DuckType> {
    if matches!(left, Number::Real | Number::Double)
        || matches!(right, Number::Real | Number::Double)
    {
        return Some(if left == Number::Double || right == Number::Double {
            DuckType::Double
        } else {
            DuckType::Real
        });
    }
    if operator == Operator::Div {
        return Some(DuckType::Double);
    }
    if let (
        Number::Integer(left_bits, left_unsigned),
        Number::Integer(right_bits, right_unsigned),
    ) = (left, right)
    {
        match (left_literal, right_literal) {
            (None, Some(value)) if fits(value, left_bits, left_unsigned) => {
                return Some(left_type.clone());
            }
            (Some(value), None) if fits(value, right_bits, right_unsigned) => {
                return Some(right_type.clone());
            }
            _ => {}
        }
        return unsigned_coercion(left_type, right_type, true)
            .or_else(|| Some(DuckType::Integer(left_bits.max(right_bits), false)));
    }
    if operator == Operator::IntegerDiv {
        return Some(DuckType::Double);
    }
    let (left_precision, left_scale) = decimal_shape(left, false)?;
    let (right_precision, right_scale) = decimal_shape(right, false)?;
    let widest: u32 = left_precision.max(right_precision);
    let scale: u32 = left_scale.max(right_scale);
    let integral: u32 = (left_precision - left_scale).max(right_precision - right_scale);
    match operator {
        Operator::Add | Operator::Sub => {
            let mut width: u32 = (scale + integral).max(widest) + 1;
            if width > DUCKDB_SMALL_DECIMAL && widest <= DUCKDB_SMALL_DECIMAL {
                width = DUCKDB_SMALL_DECIMAL;
            }
            Some(DuckType::Decimal(width.min(DUCKDB_DECIMAL_LIMIT), scale))
        }
        Operator::Mul => {
            let product_scale: u32 = left_scale + right_scale;
            if product_scale > DUCKDB_DECIMAL_LIMIT {
                return None;
            }
            let mut width: u32 = left_precision + right_precision;
            if width > DUCKDB_SMALL_DECIMAL
                && widest <= DUCKDB_SMALL_DECIMAL
                && product_scale < DUCKDB_SMALL_DECIMAL
            {
                width = DUCKDB_SMALL_DECIMAL;
            }
            Some(DuckType::Decimal(
                width.min(DUCKDB_DECIMAL_LIMIT),
                product_scale,
            ))
        }
        Operator::Mod => {
            let width: u32 = (scale + integral).max(widest);
            Some(if width > DUCKDB_DECIMAL_LIMIT {
                DuckType::Double
            } else {
                DuckType::Decimal(width, scale)
            })
        }
        Operator::Div | Operator::IntegerDiv => None,
    }
}

/// Date and time arithmetic, the overloads DuckDB binds.
fn temporal_arithmetic(operator: Operator, left: &DuckType, right: &DuckType) -> Option<DuckType> {
    use DuckType::{Date, Interval, Time, Timestamp};
    let offset = |value: &DuckType| matches!(value, DuckType::Integer(..));
    let numeric = |value: &DuckType| value.number().is_some();
    match operator {
        Operator::Add => match (left, right) {
            (Date, other) | (other, Date) if offset(other) => Some(Date),
            (Date, Interval) | (Interval, Date) => Some(Timestamp(false)),
            (Timestamp(zone), Interval) | (Interval, Timestamp(zone)) => Some(Timestamp(*zone)),
            (Time(zone), Interval) | (Interval, Time(zone)) => Some(Time(*zone)),
            (Date, Time(false)) | (Time(false), Date) => Some(Timestamp(false)),
            (Interval, Interval) => Some(Interval),
            _ => None,
        },
        Operator::Sub => match (left, right) {
            (Date, other) if offset(other) => Some(Date),
            (Date, Date) => Some(DuckType::Integer(64, false)),
            (Date, Interval) => Some(Timestamp(false)),
            (Timestamp(zone), Interval) => Some(Timestamp(*zone)),
            (Timestamp(left_zone), Timestamp(right_zone)) if left_zone == right_zone => {
                Some(Interval)
            }
            (Date, Timestamp(false)) | (Timestamp(false), Date) => Some(Interval),
            (Time(zone), Interval) => Some(Time(*zone)),
            (Interval, Interval) => Some(Interval),
            _ => None,
        },
        Operator::Mul => match (left, right) {
            (Interval, other) | (other, Interval) if numeric(other) => Some(Interval),
            _ => None,
        },
        Operator::Div => match (left, right) {
            (Interval, other) if numeric(other) => Some(Interval),
            _ => None,
        },
        Operator::Mod | Operator::IntegerDiv => None,
    }
}

/// The common type of CASE or COALESCE values: NULL aside, string and integer literals adapting.
fn unified<'a>(
    values: impl IntoIterator<Item = &'a Expression>,
    operands: &Operands<'_, '_>,
) -> Option<DuckType> {
    let mut typed: Option<DuckType> = None;
    let mut literals: Vec<i128> = Vec::new();
    let mut string_literal: bool = false;
    for value in values {
        if is_null(value) {
            continue;
        }
        if is_string_literal(value) {
            string_literal = true;
            continue;
        }
        if let Some(literal) = integer_literal(value) {
            literals.push(literal);
            continue;
        }
        let value_type: DuckType = operands.typed(value)?;
        typed = Some(match typed {
            None => value_type,
            Some(current) => common_type(&current, &value_type)?,
        });
    }
    for literal in literals {
        let magnitude: i128 = match i128::try_from(literal.unsigned_abs()) {
            Ok(magnitude) => magnitude,
            Err(_too_large) => return None,
        };
        let literal_type: DuckType = integer_literal_type(magnitude);
        typed = Some(match typed {
            None => literal_type,
            Some(current) => match current.integer_shape() {
                Some((bits, unsigned)) if bits > 0 && fits(literal, bits, unsigned) => current,
                _ => common_type(&current, &literal_type)?,
            },
        });
    }
    match typed {
        Some(data_type) => Some(data_type),
        None if string_literal => Some(DuckType::Text),
        None => None,
    }
}

/// SUM: HUGEINT over integers and BOOLEAN, DOUBLE over floats, DECIMAL(38, s) over decimals.
fn sum_type(operand: Option<DuckType>) -> Option<DuckType> {
    match operand? {
        DuckType::Boolean | DuckType::Integer(_, false) => {
            Some(DuckType::Integer(DUCKDB_WIDEST_INTEGER, false))
        }
        DuckType::Integer(bits, true) if bits < DUCKDB_WIDEST_INTEGER => {
            Some(DuckType::Integer(DUCKDB_WIDEST_INTEGER, false))
        }
        DuckType::Integer(..) | DuckType::Real | DuckType::Double => Some(DuckType::Double),
        DuckType::Decimal(_, scale) => Some(DuckType::Decimal(DUCKDB_DECIMAL_LIMIT, scale)),
        _ => None,
    }
}

fn average_type(operand: Option<DuckType>) -> Option<DuckType> {
    let operand: DuckType = operand?;
    if operand.number().is_some() {
        return Some(DuckType::Double);
    }
    match operand {
        DuckType::Interval => Some(DuckType::Interval),
        DuckType::Date => Some(DuckType::Timestamp(false)),
        DuckType::Timestamp(_) | DuckType::Time(_) => Some(operand),
        _ => None,
    }
}

fn statistic_type(operand: Option<DuckType>) -> Option<DuckType> {
    operand?.number().map(|_| DuckType::Double)
}

fn round_type(operand: Option<DuckType>, digits: Option<&Expression>) -> Option<DuckType> {
    match operand?.number()? {
        Number::Integer(bits, false) => Some(DuckType::Integer(bits, false)),
        Number::Integer(bits, true) if bits <= DUCKDB_INTEGER_BITS => {
            Some(DuckType::Integer(DUCKDB_BIGINT_BITS, false))
        }
        Number::Integer(bits, true) if bits == DUCKDB_BIGINT_BITS => {
            Some(DuckType::Integer(DUCKDB_WIDEST_INTEGER, false))
        }
        Number::Integer(..) | Number::Double => Some(DuckType::Double),
        Number::Real => Some(DuckType::Real),
        Number::Decimal(precision, scale) => match digits {
            None => Some(DuckType::Decimal(precision, 0)),
            Some(digits) => {
                let digits: u32 = match u32::try_from(integer_literal(digits)?) {
                    Ok(digits) => digits,
                    Err(_negative) => return None,
                };
                Some(DuckType::Decimal(precision, scale.min(digits)))
            }
        },
    }
}

fn floor_type(operand: Option<DuckType>) -> Option<DuckType> {
    match operand?.number()? {
        Number::Integer(..) | Number::Double => Some(DuckType::Double),
        Number::Real => Some(DuckType::Real),
        Number::Decimal(precision, _) => Some(DuckType::Decimal(precision, 0)),
    }
}

fn transcendental_type(arguments: &[&Expression], operands: &Operands<'_, '_>) -> Option<DuckType> {
    for argument in arguments {
        operands.typed(argument)?.number()?;
    }
    Some(DuckType::Double)
}

/// VARCHAR, unless the input is binary or a list.
fn string_result(input: Option<&Expression>, operands: &Operands<'_, '_>) -> Option<DuckType> {
    match input.and_then(|input| operands.typed(input)) {
        Some(DuckType::Binary | DuckType::Array(_)) => None,
        _ => Some(DuckType::Text),
    }
}

/// CONCAT: VARCHAR when a value is text, or when every value is known.
fn concat_type(arguments: &[&Expression], operands: &Operands<'_, '_>) -> Option<DuckType> {
    let mut has_string: bool = false;
    let mut all_known: bool = true;
    for argument in arguments {
        if is_string_literal(argument) {
            has_string = true;
            continue;
        }
        match operands.spelled(argument) {
            Some(spelled) => match parsed_type(&spelled) {
                Some(DuckType::Binary | DuckType::Array(_)) => return None,
                Some(DuckType::Text) => has_string = true,
                Some(_) => {}
                None if is_semi_structured(&spelled) => return None,
                None => {}
            },
            None => all_known &= is_null(argument),
        }
    }
    (has_string || all_known).then_some(DuckType::Text)
}

/// JSON values, which CONCAT does not take as text.
fn is_semi_structured(spelled: &str) -> bool {
    let upper: String = spelled.trim().to_ascii_uppercase();
    DUCKDB_JSON_TYPES.contains(&upper.as_str())
}

fn truncated_type(value: Option<DuckType>) -> Option<DuckType> {
    match value? {
        DuckType::Date => Some(DuckType::Timestamp(false)),
        DuckType::Timestamp(zone) => Some(DuckType::Timestamp(zone)),
        DuckType::Interval => Some(DuckType::Interval),
        _ => None,
    }
}

/// DATE_PART: DOUBLE for epoch and julian, BIGINT for other fields named by a string.
fn part_type(field: &Expression) -> Option<DuckType> {
    let Expression::Literal(literal) = unwrapped(field) else {
        return None;
    };
    let Literal::String(name) = literal.as_ref() else {
        return None;
    };
    Some(
        if name.eq_ignore_ascii_case(DUCKDB_EPOCH_PART)
            || name.eq_ignore_ascii_case(DUCKDB_JULIAN_PART)
        {
            DuckType::Double
        } else {
            DuckType::Integer(64, false)
        },
    )
}
