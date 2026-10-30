//! Arithmetic, comparison and logical operators.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::decimal_arithmetic::decimal_arithmetic;
use sqlquack_duckdb::blocks::main::double_arithmetic::double_arithmetic;
use sqlquack_duckdb::blocks::main::exact_decimal_divide::exact_decimal_divide;
use sqlquack_duckdb::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb::semi_structured::main::json_number_equal::json_number_equal;
use sqlquack_duckdb::semi_structured::main::json_output::json_output;
use sqlquack_duckdb::semi_structured::main::json_text::json_text;
use sqlquack_duckdb::text::constants::TEXT_NUMBER_SCALE;
use sqlquack_duckdb::text::main::compared_number_range::compared_number_range;
use sqlquack_duckdb::text::main::compared_number_text::compared_number_text;
use sqlquack_snowflake::typing::constants::MAX_PRECISION;
use sqlquack_snowflake::typing::main::comparison::comparison;
use sqlquack_snowflake::typing::main::numeric_add_subtract::numeric_add_subtract;
use sqlquack_snowflake::typing::main::numeric_divide::numeric_divide;
use sqlquack_snowflake::typing::main::numeric_modulo::numeric_modulo;
use sqlquack_snowflake::typing::main::numeric_multiply::numeric_multiply;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::{diverge, require_widening_range};
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, node_construct, payload_object, require_understood,
};
use crate::conversion::_helpers::support::typed_leaves::compared_by_text;
use sqlquack_duckdb::blocks::constants::NANOSECOND_DIGITS;

use crate::conversion::constants::{
    COMPARISON, DATE_LITERAL_COMPARISON, DECIMAL_ARITHMETIC, DECIMAL_DIVISION, DECIMAL_MODULO,
    FLOAT_ARITHMETIC, FLOAT_MODULO, LOGIC, MOD_PAIR, NULL_SAFE_COMPARISON, NUMERIC_RANGE,
    TEXT_NUMBER_COMPARISON, VARIANT_EQUALITY,
};

/// Characters in a `YYYY-MM-DD` date.
const ISO_DATE_LENGTH: usize = 10;
/// Characters in an `HH:MI:SS` time.
const CLOCK_TIME_LENGTH: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arithmetic {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Arithmetic {
    fn of(kind: &str) -> Self {
        match kind {
            "add" => Self::Add,
            "sub" => Self::Subtract,
            "mul" => Self::Multiply,
            _ => Self::Divide,
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
        }
    }
}

/// Two operands of a binary node.
fn operands(kind: &str, payload: &Value, site: Site<'_>) -> Result<(Emitted, Emitted), Refusal> {
    let construct = node_construct(kind);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["left", "right"], &construct)?;
    Ok((
        convert_expression(field(payload, "left", &construct)?, site)?,
        convert_expression(field(payload, "right", &construct)?, site)?,
    ))
}

/// `+ - * /`.
pub(crate) fn arithmetic(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let operator = Arithmetic::of(kind);
    let (left, right) = operands(kind, payload, site)?;
    let log = site.context.log;
    let rule = match (left.data_type, right.data_type, operator) {
        (SnowflakeType::Number { .. }, SnowflakeType::Number { .. }, Arithmetic::Divide) => {
            DECIMAL_DIVISION
        }
        (SnowflakeType::Number { .. }, SnowflakeType::Number { .. }, _) => DECIMAL_ARITHMETIC,
        _ => FLOAT_ARITHMETIC,
    };
    let _rule = log.rule(rule);
    let typed = match operator {
        Arithmetic::Add | Arithmetic::Subtract => {
            numeric_add_subtract(left.data_type, right.data_type, log)
        }
        Arithmetic::Multiply => numeric_multiply(left.data_type, right.data_type, log),
        Arithmetic::Divide => numeric_divide(left.data_type, right.data_type, log),
    };
    let result = typed.map_err(|reason| outside_conditions(construct.clone(), reason))?;
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    match (left.data_type, right.data_type, result) {
        (
            SnowflakeType::Number { .. },
            SnowflakeType::Number { .. },
            SnowflakeType::Number { .. },
        ) => exact_arithmetic(operator, (&left, &right), result, site.context).map_err(refuse),
        (_, _, SnowflakeType::Float) => Ok(Emitted::new(
            double_arithmetic(operator.symbol(), &left.sql, &right.sql, log),
            SnowflakeType::Float,
        )),
        _ => Err(refuse(format!(
            "`{}` of {} and {}",
            operator.symbol(),
            left.data_type,
            right.data_type
        ))),
    }
}

/// `a % b` of two NUMBERs; a zero divisor fails as DuckDB's division by zero does.
pub(crate) fn modulo(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands("mod", payload, site)?;
    modulo_of((left, right), &node_construct("mod"), site)
}

/// MOD(a, b), spelled as `a % b`.
pub(crate) fn mod_call(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let [left, right] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "MOD takes two arguments".to_owned(),
        ));
    };
    site.context.log.equivalence(MOD_PAIR);
    let operands = (
        convert_expression(left, site)?,
        convert_expression(right, site)?,
    );
    modulo_of(operands, construct, site)
}

fn modulo_of(
    (left, right): (Emitted, Emitted),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(DECIMAL_MODULO);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let result = numeric_modulo(left.data_type, right.data_type, log).map_err(refuse)?;
    let nonzero_literal = matches!(
        &right.literal,
        Some(Literal::Number(digits)) if digits.bytes().any(|byte| byte.is_ascii_digit() && byte != b'0')
    );
    if result == SnowflakeType::Float && nonzero_literal {
        let _float = log.rule(FLOAT_MODULO);
        return Ok(Emitted::new(
            format!(
                "((CAST({} AS DOUBLE) % CAST({} AS DOUBLE)) + 0.0)",
                left.sql, right.sql
            ),
            SnowflakeType::Float,
        ));
    }
    if !matches!(
        (left.data_type, right.data_type, result),
        (
            SnowflakeType::Number { .. },
            SnowflakeType::Number { .. },
            SnowflakeType::Number { .. }
        )
    ) {
        return Err(refuse(format!(
            "`%` of {} and {}, which no recording covers",
            left.data_type, right.data_type
        )));
    }
    let remainder = match (left.data_type, right.data_type) {
        (
            SnowflakeType::Number {
                precision: left_precision,
                scale: left_scale,
            },
            SnowflakeType::Number {
                precision: right_precision,
                scale: right_scale,
            },
        ) if (left_precision - left_scale).max(right_precision - right_scale)
            + left_scale.max(right_scale)
            > MAX_PRECISION =>
        {
            if right_scale != 0 {
                return Err(refuse(format!(
                    "`%` of {} and {}, which DuckDB computes as a double",
                    left.data_type, right.data_type
                )));
            }
            whole_divisor_remainder((&left.sql, &right.sql))
        }
        _ => format!("({} % {})", left.sql, right.sql),
    };
    let sql = cast(&remainder, result, log).map_err(refuse)?;
    Ok(Emitted::new(sql, result))
}

/// `x % y` for a whole-number `y`, exact where DuckDB's `%` of the two would use a double.
fn whole_divisor_remainder((dividend, divisor): (&str, &str)) -> String {
    format!(
        "list_transform([{dividend}], lambda modulo_dividend: \
         (CAST(trunc(modulo_dividend) AS DECIMAL({MAX_PRECISION},0)) % {divisor}) \
         + (modulo_dividend - trunc(modulo_dividend)))[1]"
    )
}

fn exact_arithmetic(
    operator: Arithmetic,
    (left, right): (&Emitted, &Emitted),
    result: SnowflakeType,
    context: &Context<'_>,
) -> Result<Emitted, String> {
    let log = context.log;
    let (
        SnowflakeType::Number {
            precision: left_precision,
            scale: left_scale,
        },
        SnowflakeType::Number {
            precision: right_precision,
            scale: right_scale,
        },
        SnowflakeType::Number { scale, .. },
    ) = (left.data_type, right.data_type, result)
    else {
        return Err("exact arithmetic needs NUMBER operands".to_owned());
    };
    if operator == Arithmetic::Divide {
        let (sql, guarded) = exact_decimal_divide(
            (&left.sql, left_precision, left_scale),
            (&right.sql, right_precision, right_scale),
            result,
            log,
        )?;
        if guarded {
            diverge(context, NUMERIC_RANGE, &node_construct("div"))
                .map_err(|refusal| refusal.reason)?;
        }
        return Ok(Emitted::new(sql, result));
    }
    let exact_sum_digits =
        u32::from((left_precision - left_scale).max(right_precision - right_scale))
            + 1
            + u32::from(left_scale.max(right_scale));
    if matches!(operator, Arithmetic::Add | Arithmetic::Subtract)
        && exact_sum_digits > u32::from(MAX_PRECISION)
    {
        diverge(context, NUMERIC_RANGE, &node_construct(operator.kind()))
            .map_err(|refusal| refusal.reason)?;
    }
    let product_scale = left_scale + right_scale;
    let intermediate = u32::from(left_precision - left_scale)
        + u32::from(right_precision - right_scale)
        + u32::from(product_scale);
    if operator == Arithmetic::Multiply
        && scale < product_scale
        && intermediate > u32::from(MAX_PRECISION)
    {
        return Err(format!(
            "the exact product needs {intermediate} digits before Snowflake cuts its scale"
        ));
    }
    if operator == Arithmetic::Multiply && intermediate > u32::from(MAX_PRECISION) {
        diverge(context, NUMERIC_RANGE, &node_construct(operator.kind()))
            .map_err(|refusal| refusal.reason)?;
    }
    let sql = decimal_arithmetic(
        operator.symbol(),
        [(&left.sql, left_scale), (&right.sql, right_scale)],
        result,
        log,
    )?;
    Ok(Emitted::new(sql, result))
}

/// `= <> < <= > >=`.
pub(crate) fn comparison_predicate(
    kind: &str,
    payload: &Value,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let (left, right) = operands(kind, payload, site)?;
    if matches!(kind, "eq" | "neq")
        && let Some(compared) = variant_equality(kind, (&left, &right), &construct, site.context)?
    {
        return Ok(compared);
    }
    let (left, right) = comparable(left, right, &construct, site.context)?;
    let symbol = match kind {
        "eq" => "=",
        "neq" => "<>",
        "lt" => "<",
        "lte" => "<=",
        "gt" => ">",
        _ => ">=",
    };
    Ok(Emitted::new(
        format!("({} {symbol} {})", left.sql, right.sql),
        SnowflakeType::Boolean,
    ))
}

/// `=` or `<>` with a VARIANT, compared by Snowflake's text of each; None for other operands.
fn variant_equality(
    kind: &str,
    (left, right): (&Emitted, &Emitted),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Option<Emitted>, Refusal> {
    let (variant, other) = match (left.data_type, right.data_type) {
        (SnowflakeType::SemiStructured, _) => (left, right),
        (_, SnowflakeType::SemiStructured) => (right, left),
        _ => return Ok(None),
    };
    let negation = match kind {
        "neq" => "NOT ",
        _ => "",
    };
    let compared = match other.data_type {
        SnowflakeType::Varchar => {
            format!("({} = {})", json_text(&variant.sql, context.log), other.sql)
        }
        SnowflakeType::Number { scale: 0, .. } => {
            json_number_equal((&variant.sql, &other.sql), context.log)
        }
        SnowflakeType::SemiStructured => format!(
            "(CAST({} AS VARCHAR) = CAST({} AS VARCHAR))",
            json_output(&variant.sql, context.log),
            json_output(&other.sql, context.log)
        ),
        _ => return Ok(None),
    };
    let _rule = context.log.rule(VARIANT_EQUALITY);
    diverge(context, JSON_FIDELITY, construct)?;
    let other_variant = other.data_type == SnowflakeType::SemiStructured;
    compared_by_text(
        (
            variant.untainted && (other.untainted || !other_variant),
            variant.sourced || (other.sourced && other_variant),
        ),
        construct,
        context,
    )?;
    Ok(Some(Emitted::new(
        format!("({negation}{compared})"),
        SnowflakeType::Boolean,
    )))
}

/// EQUAL_NULL(a, b): `a IS NOT DISTINCT FROM b`.
pub(crate) fn equal_null(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(NULL_SAFE_COMPARISON);
    let [left, right] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "EQUAL_NULL takes two arguments".to_owned(),
        ));
    };
    let left = convert_expression(left, site)?;
    let right = convert_expression(right, site)?;
    let (left, right) = comparable(left, right, construct, site.context)?;
    Ok(Emitted::new(
        format!("({} IS NOT DISTINCT FROM {})", left.sql, right.sql),
        SnowflakeType::Boolean,
    ))
}

/// `IS [NOT] DISTINCT FROM`: NULL compares equal to NULL and unequal to any value.
pub(crate) fn null_safe_comparison(
    kind: &str,
    payload: &Value,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let _rule = site.context.log.rule(NULL_SAFE_COMPARISON);
    let (left, right) = operands(kind, payload, site)?;
    let (left, right) = comparable(left, right, &construct, site.context)?;
    let symbol = match kind {
        "null_safe_eq" => "IS NOT DISTINCT FROM",
        _ => "IS DISTINCT FROM",
    };
    Ok(Emitted::new(
        format!("({} {symbol} {})", left.sql, right.sql),
        SnowflakeType::Boolean,
    ))
}

/// Both operands, made directly comparable in DuckDB, or a refusal.
pub(crate) fn comparable(
    left: Emitted,
    right: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Emitted, Emitted), Refusal> {
    match (left.data_type, right.data_type) {
        (SnowflakeType::Date, SnowflakeType::Varchar) => {
            let right = date_literal(right, construct, context)?;
            return Ok((left, right));
        }
        (SnowflakeType::Varchar, SnowflakeType::Date) => {
            let left = date_literal(left, construct, context)?;
            return Ok((left, right));
        }
        (SnowflakeType::Time { .. }, SnowflakeType::Varchar) => {
            let right = time_literal(right, construct, context)?;
            return Ok((left, right));
        }
        (SnowflakeType::Varchar, SnowflakeType::Time { .. }) => {
            let left = time_literal(left, construct, context)?;
            return Ok((left, right));
        }
        (SnowflakeType::Varchar, number @ SnowflakeType::Number { .. }) => {
            let text = number_text(left, number, construct, context)?;
            return Ok((text, number_range(right, context)));
        }
        (number @ SnowflakeType::Number { .. }, SnowflakeType::Varchar) => {
            let text = number_text(right, number, construct, context)?;
            return Ok((number_range(left, context), text));
        }
        _ => {}
    }
    let log = context.log;
    let _rule = log.rule(COMPARISON);
    comparison(left.data_type, right.data_type, log)
        .map_err(|reason| outside_conditions(construct.clone(), reason))?;
    require_widening_range(&[left.data_type, right.data_type], construct, context)?;
    match (left.data_type, right.data_type) {
        (SnowflakeType::Float, SnowflakeType::Number { .. })
        | (SnowflakeType::Number { .. }, SnowflakeType::Float) => Ok((
            as_double(left, construct, context)?,
            as_double(right, construct, context)?,
        )),
        _ => Ok((left, right)),
    }
}

/// A NUMBER compared with text, raising where Snowflake's comparison overflows.
fn number_range(number: Emitted, context: &Context<'_>) -> Emitted {
    match number.data_type {
        SnowflakeType::Number { precision, scale } => Emitted::new(
            compared_number_range(&number.sql, (precision, scale), context.log),
            number.data_type,
        ),
        _ => number,
    }
}

/// Text compared with `number`, read unrounded as Snowflake reads it; relies on text_number_scale.
fn number_text(
    text: Emitted,
    number: SnowflakeType,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    let SnowflakeType::Number { precision, scale } = number else {
        return Err(outside_conditions(
            construct.clone(),
            format!("text compared with {number}"),
        ));
    };
    let scale = scale.max(MAX_PRECISION - precision);
    let _rule = context.log.rule(TEXT_NUMBER_COMPARISON);
    diverge(context, TEXT_NUMBER_SCALE, construct)?;
    let sql = compared_number_text(&text.sql, scale, context.log)
        .map_err(|reason| outside_conditions(construct.clone(), reason))?;
    Ok(Emitted::new(
        sql,
        SnowflakeType::Number {
            precision: MAX_PRECISION,
            scale,
        },
    ))
}

fn as_double(
    emitted: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    Ok(Emitted::new(
        cast(&emitted.sql, SnowflakeType::Float, context.log)
            .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        SnowflakeType::Float,
    ))
}

fn date_literal(
    emitted: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    match &emitted.literal {
        Some(Literal::Text(text)) if is_iso_date(text) => {
            let _rule = context.log.rule(DATE_LITERAL_COMPARISON);
            Ok(Emitted::new(
                cast(&emitted.sql, SnowflakeType::Date, context.log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?,
                SnowflakeType::Date,
            ))
        }
        _ => Err(outside_conditions(
            construct.clone(),
            "comparing DATE with VARCHAR needs an implicit conversion".to_owned(),
        )),
    }
}

/// A text literal compared with a TIME, read as the TIME it spells in `HH:MI:SS`.
fn time_literal(
    emitted: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    match &emitted.literal {
        Some(Literal::Text(text)) if is_clock_time(text) => {
            let _rule = context.log.rule(DATE_LITERAL_COMPARISON);
            let data_type = SnowflakeType::Time {
                precision: NANOSECOND_DIGITS,
            };
            Ok(Emitted::new(
                cast(&emitted.sql, data_type, context.log)
                    .map_err(|reason| outside_conditions(construct.clone(), reason))?,
                data_type,
            ))
        }
        _ => Err(outside_conditions(
            construct.clone(),
            "comparing TIME with VARCHAR needs an implicit conversion".to_owned(),
        )),
    }
}

/// Whether text is an `HH:MI:SS` time of day, hours 00-23 and minutes and seconds 00-59.
fn is_clock_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    let field = |start: usize, most: &[u8; 2]| {
        bytes
            .get(start..start + 2)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_digit) && digits <= &most[..])
    };
    text.len() == CLOCK_TIME_LENGTH
        && bytes[2] == b':'
        && bytes[5] == b':'
        && field(0, b"23")
        && field(3, b"59")
        && field(6, b"59")
}

/// Whether text is a `YYYY-MM-DD` date.
pub(crate) fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == ISO_DATE_LENGTH
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// Whether two types belong to one family that compares without conversion.
pub(crate) fn same_family(
    left: SnowflakeType,
    right: SnowflakeType,
    construct: &Construct,
) -> Result<(), Refusal> {
    let family = |data_type: SnowflakeType| match data_type {
        SnowflakeType::Number { .. } => Some(0),
        SnowflakeType::Float => Some(1),
        SnowflakeType::Varchar => Some(2),
        SnowflakeType::Date => Some(3),
        SnowflakeType::Boolean => Some(4),
        SnowflakeType::TimestampNtz { .. } => Some(5),
        SnowflakeType::TimestampTz { .. } | SnowflakeType::TimestampLtz { .. } => Some(6),
        _ => None,
    };
    match (left, right) {
        (SnowflakeType::Null, _) | (_, SnowflakeType::Null) => Ok(()),
        _ if family(left).is_some() && family(left) == family(right) => Ok(()),
        _ => Err(outside_conditions(
            construct.clone(),
            format!("{left} and {right} are not of one type family"),
        )),
    }
}

/// A BOOLEAN (or NULL) operand's SQL.
pub(crate) fn boolean_operand(node: &Value, site: Site<'_>) -> Result<String, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Boolean | SnowflakeType::Null => Ok(emitted.sql),
        other => Err(outside_conditions(
            node_construct("and"),
            format!("a condition of type {other} needs an implicit conversion"),
        )),
    }
}

/// AND and OR.
pub(crate) fn logic(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let _rule = site.context.log.rule(LOGIC);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["left", "right"], &construct)?;
    let left = boolean_operand(field(payload, "left", &construct)?, site)?;
    let right = boolean_operand(field(payload, "right", &construct)?, site)?;
    let keyword = match kind {
        "and" => "AND",
        _ => "OR",
    };
    Ok(Emitted::new(
        format!("({left} {keyword} {right})"),
        SnowflakeType::Boolean,
    ))
}

/// NOT.
pub(crate) fn negation(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("not");
    let _rule = site.context.log.rule(LOGIC);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this"], &construct)?;
    let inner = boolean_operand(field(payload, "this", &construct)?, site)?;
    Ok(Emitted::new(
        format!("(NOT {inner})"),
        SnowflakeType::Boolean,
    ))
}
