//! Number functions: remainders and quotients raising on zero, rounding, signs and NaN tests.

use sqlquack_diagnostics::failures::constants::ARGUMENT_REJECTED;

use crate::databricks::conversion::_helpers::expressions::bitwise::shifted;
use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::{
    as_type, comparable, whole_text,
};
use crate::databricks::conversion::_helpers::functions::rewrites::{Rewrite, small_integer};
use crate::databricks::conversion::_helpers::support::typing::{same_cast, wider_integer};
use crate::databricks::conversion::constants::{
    BIGINT_MAX, BIGINT_MIN, BIGINT_SPAN_DOUBLE, BIT_POSITION_ERROR, FACTORIAL_MAX,
    MAX_DECIMAL_PRECISION,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal, Shift};

/// The rewrite of the number function `name`, `None` when no rewrite handles it.
pub(crate) fn number_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "MOD" => modulo,
        "DIV" => integral_divide,
        "EQUAL_NULL" => equal_null,
        "NANVL" => nanvl,
        "ISNAN" => nan_check,
        "ISNULL" => null_check,
        "ISNOTNULL" => not_null_check,
        "ZEROIFNULL" => zero_if_null,
        "NULLIFZERO" => null_if_zero,
        "NEGATIVE" => negative,
        "POSITIVE" => positive,
        "SIGN" | "SIGNUM" => sign,
        "CEIL" | "CEILING" => ceil,
        "FLOOR" => floor,
        "ROUND" => round_half_up,
        "BROUND" => round_half_even,
        "SQRT" => square_root,
        "FACTORIAL" => factorial,
        "BIT_COUNT" => bit_count,
        "PMOD" => positive_modulo,
        "BIN" => binary_text,
        "WIDTH_BUCKET" => width_bucket,
        "BIT_GET" | "GETBIT" => bit_get,
        "SHIFTLEFT" => |values| shift_function(values, Shift::Left),
        "SHIFTRIGHT" => |values| shift_function(values, Shift::Right),
        "SHIFTRIGHTUNSIGNED" => |values| shift_function(values, Shift::RightUnsigned),
        _ => return None,
    };
    Some(rewrite)
}

/// `a % b` as Databricks computes it: the dividend's sign, 0 by -1 where DuckDB overflows, raising on zero.
pub(crate) fn remainder(left: &Emitted, right: &Emitted) -> Option<Emitted> {
    use DatabricksType as T;
    if let Some(wider) = wider_integer(left.data_type, right.data_type) {
        let (dividend, divisor) = (as_type(left, wider), as_type(right, wider));
        return Some(typed(
            format!("CASE WHEN {divisor} = -1 THEN {dividend} * 0 ELSE {dividend} % {divisor} END"),
            wider,
        ));
    }
    if let (Some((p1, s1)), Some((p2, s2))) = (exact(left.data_type), exact(right.data_type)) {
        let scale = s1.max(s2);
        let precision = (p1 - s1).min(p2 - s2) + scale;
        let result = T::Decimal { precision, scale };
        return Some(typed(
            format!(
                "CAST({} % {} AS {})",
                as_decimal(left, p1, s1),
                as_decimal(right, p2, s2),
                result.duckdb_name()
            ),
            result,
        ));
    }
    let (dividend, divisor) = (double(left)?, double(right)?);
    Some(typed(
        raising_on_zero(&dividend, &divisor, &format!("{dividend} % {divisor}")),
        T::Double,
    ))
}

/// `a / b` of integers or DOUBLEs by a divisor that may be zero: DOUBLE, raising on zero.
pub(crate) fn quotient(left: &Emitted, right: &Emitted) -> Option<Emitted> {
    let numeric = |value: &Emitted| {
        value.data_type.is_integral() || value.data_type == DatabricksType::Double
    };
    if !numeric(left) || !numeric(right) {
        return None;
    }
    let (dividend, divisor) = (double(left)?, double(right)?);
    Some(typed(
        raising_on_zero(&dividend, &divisor, &format!("{dividend} / {divisor}")),
        DatabricksType::Double,
    ))
}

/// `result` unless the divisor is zero, where an integer remainder by zero raises in DuckDB too.
fn raising_on_zero(dividend: &str, divisor: &str, result: &str) -> String {
    format!(
        "CASE WHEN {dividend} IS NULL THEN NULL WHEN {divisor} = 0 \
         THEN CAST(0 % CAST({divisor} AS INTEGER) AS DOUBLE) ELSE {result} END"
    )
}

/// `MOD(a, b)`, the `%` operator's function form.
fn modulo(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    remainder(left, right)
}

/// `PMOD(a, n)` of integers as Java computes it: `(r + n) % n` for a negative `r`, the sum wrapping in INT or BIGINT and computed in INT for narrower types.
fn positive_modulo(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    let Some(wider) = wider_integer(left.data_type, right.data_type) else {
        return exact_positive_modulo(left, right);
    };
    let (a, n) = (as_type(left, wider), as_type(right, wider));
    let r = format!("(CASE WHEN {n} = -1 THEN {a} * 0 ELSE {a} % {n} END)");
    let wide = wider.duckdb_name();
    let positive = match wider {
        DatabricksType::Int => {
            let sum = wrapped(&format!("(CAST({r} AS BIGINT) + {n})"), 32, "BIGINT");
            format!("CAST(CAST({sum} AS {wide}) % {n} AS {wide})")
        }
        DatabricksType::BigInt => {
            let sum = wrapped(&format!("(CAST({r} AS HUGEINT) + {n})"), 64, "HUGEINT");
            format!("CAST(CAST({sum} AS {wide}) % {n} AS {wide})")
        }
        _ => format!("CAST((CAST({r} AS INTEGER) + {n}) % {n} AS {wide})"),
    };
    Some(typed(
        format!("CASE WHEN {r} < 0 THEN {positive} ELSE {r} END"),
        wider,
    ))
}

/// `PMOD(a, n)` of decimals or DOUBLEs as Spark computes it: `(r + n) % n` for a negative remainder `r`, in the remainder's type.
fn exact_positive_modulo(left: &Emitted, right: &Emitted) -> Option<Emitted> {
    let rest = remainder(left, right)?;
    let (r, n) = (&rest.sql, &right.sql);
    Some(typed(
        format!(
            "(CASE WHEN {r} < 0 THEN CAST(({r} + {n}) % {n} AS {name}) ELSE {r} END)",
            name = rest.data_type.duckdb_name()
        ),
        rest.data_type,
    ))
}

/// `value`, of a type wider than `bits`, wrapped into the `bits`-bit two's complement range.
pub(crate) fn wrapped(value: &str, bits: u32, wide: &str) -> String {
    let half = format!("CAST('{}' AS {wide})", 1_u128 << (bits - 1));
    let span = format!("CAST('{}' AS {wide})", 1_u128 << bits);
    format!("((({value} + {half}) % {span} + {span}) % {span} - {half})")
}

/// `BIN(x)`: the binary digits of the integer's 64-bit two's complement.
fn binary_text(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    value.data_type.is_integral().then(|| {
        typed(
            format!("bin({})", as_type(value, DatabricksType::BigInt)),
            DatabricksType::String,
        )
    })
}

/// `a DIV b` of integers: the quotient truncated toward zero as BIGINT, raising on zero and overflow.
fn integral_divide(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    wider_integer(left.data_type, right.data_type)?;
    Some(typed(
        format!(
            "({} // {})",
            as_type(left, DatabricksType::BigInt),
            as_type(right, DatabricksType::BigInt)
        ),
        DatabricksType::BigInt,
    ))
}

/// `EQUAL_NULL(a, b)`: `a <=> b`, TRUE for two NULLs.
fn equal_null(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    let (left, right) = comparable(left, right)?;
    Some(typed(
        format!("({left} IS NOT DISTINCT FROM {right})"),
        DatabricksType::Boolean,
    ))
}

/// `NANVL(a, b)`: `b` when `a` is NaN, else `a`, both as DOUBLE.
fn nanvl(values: &[Emitted]) -> Option<Emitted> {
    let [value, fallback] = values else {
        return None;
    };
    let (value, fallback) = (double(value)?, double(fallback)?);
    Some(typed(
        format!("CASE WHEN isnan({value}) THEN {fallback} ELSE {value} END"),
        DatabricksType::Double,
    ))
}

/// `ISNAN(x)`: FALSE for NULL, as Databricks never returns NULL from it.
fn nan_check(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    let value = double(value)?;
    Some(typed(
        format!("coalesce(isnan({value}), FALSE)"),
        DatabricksType::Boolean,
    ))
}

fn null_check(values: &[Emitted]) -> Option<Emitted> {
    null_test(values, "IS NULL")
}

fn not_null_check(values: &[Emitted]) -> Option<Emitted> {
    null_test(values, "IS NOT NULL")
}

/// `ISNULL(x)` or `ISNOTNULL(x)` as the null test it names.
fn null_test(values: &[Emitted], test: &str) -> Option<Emitted> {
    let [value] = values else { return None };
    Some(typed(
        format!("({} {test})", value.sql),
        DatabricksType::Boolean,
    ))
}

/// `ZEROIFNULL(x)` of an integer or DOUBLE: 0 of its type for NULL.
fn zero_if_null(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    if let DatabricksType::Decimal { precision, scale } = value.data_type {
        let digits = DatabricksType::Int.integer_digits()?;
        let precision = (precision - scale).max(digits) + scale;
        if precision > MAX_DECIMAL_PRECISION {
            return None;
        }
        let wider = DatabricksType::Decimal { precision, scale };
        let name = wider.duckdb_name();
        return Some(typed(
            format!("CAST(coalesce(CAST({} AS {name}), 0) AS {name})", value.sql),
            wider,
        ));
    }
    whole_or_double(value.data_type)?;
    Some(typed(
        format!(
            "coalesce({}, CAST(0 AS {}))",
            value.sql,
            value.data_type.duckdb_name()
        ),
        value.data_type,
    ))
}

/// `NULLIFZERO(x)` of an integer or DOUBLE: NULL for 0.
fn null_if_zero(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    if value.data_type == DatabricksType::String {
        return Some(typed(
            format!(
                "(CASE WHEN {} = 0 THEN NULL ELSE {} END)",
                whole_text(&value.sql),
                value.sql
            ),
            DatabricksType::String,
        ));
    }
    if matches!(value.data_type, DatabricksType::Decimal { .. }) {
        return Some(typed(format!("nullif({}, 0)", value.sql), value.data_type));
    }
    whole_or_double(value.data_type)?;
    Some(typed(format!("nullif({}, 0)", value.sql), value.data_type))
}

/// `NEGATIVE(x)`: the unary minus, raising on overflow in both engines.
fn negative(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    match value.data_type.is_numeric() {
        true => Some(typed(format!("(- {})", value.sql), value.data_type)),
        false => None,
    }
}

/// `POSITIVE(x)`: the number itself.
fn positive(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    match value.data_type.is_numeric() {
        true => Some(typed(format!("({})", value.sql), value.data_type)),
        false => None,
    }
}

/// `SIGN(x)` as DOUBLE; for a DOUBLE, Java's signum keeps zero's sign and NaN, which DuckDB orders above 0.
fn sign(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    let sql = match value.data_type {
        DatabricksType::Double => format!(
            "CASE WHEN isnan({x}) THEN {x} WHEN {x} > 0 THEN CAST(1 AS DOUBLE) \
             WHEN {x} < 0 THEN CAST(-1 AS DOUBLE) ELSE {x} END",
            x = value.sql
        ),
        data_type if exact(data_type).is_some() => {
            format!("CAST(sign({}) AS DOUBLE)", value.sql)
        }
        _ => return None,
    };
    Some(typed(sql, DatabricksType::Double))
}

fn ceil(values: &[Emitted]) -> Option<Emitted> {
    rounded(values, "ceil")
}

fn floor(values: &[Emitted]) -> Option<Emitted> {
    rounded(values, "floor")
}

/// `CEIL(x)` or `FLOOR(x)`: BIGINT for integers and DOUBLE, as Java's saturating cast; `DECIMAL(p - s + 1, 0)`.
fn rounded(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [value] = values else { return None };
    match value.data_type {
        data_type if data_type.is_integral() => Some(typed(
            as_type(value, DatabricksType::BigInt),
            DatabricksType::BigInt,
        )),
        DatabricksType::Decimal { precision, scale } => {
            whole_decimal(&format!("{function}({})", value.sql), precision, scale)
        }
        DatabricksType::Double => Some(typed(
            saturated(&format!("{function}({})", value.sql), &value.sql),
            DatabricksType::BigInt,
        )),
        _ => None,
    }
}

/// A whole DOUBLE as BIGINT the way Java casts it: 0 for NaN, the nearest bound beyond the range.
fn saturated(whole: &str, value: &str) -> String {
    format!(
        "CASE WHEN isnan({value}) THEN CAST(0 AS BIGINT) \
         WHEN {whole} >= {BIGINT_MAX} THEN CAST({BIGINT_MAX} AS BIGINT) \
         WHEN {whole} <= {BIGINT_MIN} THEN CAST({BIGINT_MIN} AS BIGINT) \
         ELSE CAST({whole} AS BIGINT) END"
    )
}

/// A decimal rounded to scale 0, typed `DECIMAL(p - s + 1, 0)` as Databricks types it.
fn whole_decimal(sql: &str, precision: u8, scale: u8) -> Option<Emitted> {
    let result = DatabricksType::Decimal {
        precision: (precision - scale + 1).min(MAX_DECIMAL_PRECISION),
        scale: 0,
    };
    Some(typed(
        format!("CAST({sql} AS {})", result.duckdb_name()),
        result,
    ))
}

/// `ROUND(x)` or `ROUND(x, 0)`: half away from zero, which DuckDB's round also does.
fn round_half_up(values: &[Emitted]) -> Option<Emitted> {
    rounded_to_whole(values, "round", "")
}

/// `BROUND(x)` or `BROUND(x, 0)`: half to even, DuckDB's round_even.
fn round_half_even(values: &[Emitted]) -> Option<Emitted> {
    rounded_to_whole(values, "round_even", ", 0")
}

/// Rounding to a whole number: integers kept, decimals to `DECIMAL(p - s + 1, 0)`, DOUBLE kept, whose shortest text ends in .5 only at exact halves.
fn rounded_to_whole(values: &[Emitted], function: &str, digits: &str) -> Option<Emitted> {
    let value = match values {
        [value] => value,
        [value, scale] if scale.literal.as_ref().is_some_and(|scale| scale == &zero()) => value,
        _ => return None,
    };
    match value.data_type {
        data_type if data_type.is_integral() => Some(typed(format!("({})", value.sql), data_type)),
        DatabricksType::Decimal { precision, scale } => whole_decimal(
            &format!("{function}({}{digits})", value.sql),
            precision,
            scale,
        ),
        DatabricksType::Double => Some(typed(
            format!("{function}({}{digits})", value.sql),
            DatabricksType::Double,
        )),
        _ => None,
    }
}

/// `SQRT(x)` as DOUBLE, correctly rounded in both engines, NaN below zero.
fn square_root(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    Some(typed(
        format!("sqrt({})", double(value)?),
        DatabricksType::Double,
    ))
}

/// `FACTORIAL(n)` as BIGINT, NULL outside 0 to 20.
fn factorial(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    if !small_integer(value.data_type) {
        return None;
    }
    Some(typed(
        format!(
            "CASE WHEN {n} BETWEEN 0 AND {FACTORIAL_MAX} THEN CAST(factorial({n}) AS BIGINT) END",
            n = value.sql
        ),
        DatabricksType::BigInt,
    ))
}

/// `BIT_COUNT(x)` of an integer: the set bits of its 64-bit two's complement, as Java's Long.bitCount, as INT.
fn bit_count(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    if value.data_type == DatabricksType::Boolean {
        return Some(typed(
            format!("CAST({} AS INTEGER)", value.sql),
            DatabricksType::Int,
        ));
    }
    value.data_type.is_integral().then(|| {
        typed(
            format!(
                "CAST(bit_count({}) AS INTEGER)",
                as_type(value, DatabricksType::BigInt)
            ),
            DatabricksType::Int,
        )
    })
}

/// `SHIFTLEFT`, `SHIFTRIGHT` or `SHIFTRIGHTUNSIGNED(x, n)`, the shift operators' function forms.
fn shift_function(values: &[Emitted], direction: Shift) -> Option<Emitted> {
    let [value, count] = values else { return None };
    shifted(value, count, direction)
}

/// `WIDTH_BUCKET(v, min, max, n)` as DOUBLEs: NULL for `n` below 1 or BIGINT's largest, equal or non-finite bounds, or a NaN `v`; Java's DOUBLE-to-BIGINT cast of the bucket.
fn width_bucket(values: &[Emitted]) -> Option<Emitted> {
    let [value, low, high, count] = values else {
        return None;
    };
    if !count.data_type.is_integral() {
        return None;
    }
    let (v, a, b) = (double(value)?, double(low)?, double(high)?);
    let n = as_type(count, DatabricksType::BigInt);
    let bucket = |offset: &str| {
        let quotient =
            format!("(CAST({n} AS DOUBLE) * ({offset}) / (greatest({a}, {b}) - least({a}, {b})))");
        format!(
            "CASE WHEN isnan({quotient}) THEN 1 WHEN {quotient} >= {BIGINT_SPAN_DOUBLE} THEN CAST({BIGINT_MIN} AS BIGINT) \
             ELSE CAST(trunc({quotient}) AS BIGINT) + 1 END"
        )
    };
    Some(typed(
        format!(
            "CASE WHEN {v} IS NULL OR {a} IS NULL OR {b} IS NULL OR {n} IS NULL THEN NULL \
             WHEN {n} <= 0 OR {n} = {BIGINT_MAX} OR isnan({v}) OR {a} = {b} \
             OR isnan({a}) OR isinf({a}) OR isnan({b}) OR isinf({b}) THEN NULL \
             WHEN {a} < {b} THEN CASE WHEN {v} < {a} THEN 0 WHEN {v} >= {b} THEN {n} + 1 ELSE {rising} END \
             ELSE CASE WHEN {v} > {a} THEN 0 WHEN {v} <= {b} THEN {n} + 1 ELSE {falling} END END",
            rising = bucket(&format!("{v} - {a}")),
            falling = bucket(&format!("{a} - {v}"))
        ),
        DatabricksType::BigInt,
    ))
}

/// `BIT_GET(x, p)`: bit `p` of an integer counting from the lowest as TINYINT, raising for `p` outside its width.
fn bit_get(values: &[Emitted]) -> Option<Emitted> {
    let [value, position] = values else {
        return None;
    };
    let width = match value.data_type {
        DatabricksType::TinyInt => 8,
        DatabricksType::SmallInt => 16,
        DatabricksType::Int => 32,
        DatabricksType::BigInt => 64,
        _ => return None,
    };
    if !small_integer(position.data_type) {
        return None;
    }
    Some(typed(
        format!(
            "CASE WHEN {x} IS NULL OR {p} IS NULL THEN NULL \
             WHEN {p} < 0 OR {p} >= {width} THEN CAST(error('{ARGUMENT_REJECTED}: {BIT_POSITION_ERROR}') AS TINYINT) \
             ELSE CAST(({x} >> CAST({p} AS INTEGER)) & 1 AS TINYINT) END",
            x = value.sql,
            p = position.sql
        ),
        DatabricksType::TinyInt,
    ))
}

/// A value's SQL as DOUBLE when DuckDB converts it exactly as Databricks does.
pub(crate) fn double(value: &Emitted) -> Option<String> {
    match same_cast(value.data_type, DatabricksType::Double) {
        true => Some(as_type(value, DatabricksType::Double)),
        false => None,
    }
}

/// The precision and scale of an exact number: a decimal, or an integer as the decimal holding it.
fn exact(data_type: DatabricksType) -> Option<(u8, u8)> {
    match data_type {
        DatabricksType::Decimal { precision, scale } => Some((precision, scale)),
        data_type => data_type.integer_digits().map(|digits| (digits, 0)),
    }
}

/// An exact number's SQL as `DECIMAL(p, s)`.
fn as_decimal(value: &Emitted, precision: u8, scale: u8) -> String {
    as_type(value, DatabricksType::Decimal { precision, scale })
}

/// An integer or DOUBLE type.
fn whole_or_double(data_type: DatabricksType) -> Option<()> {
    match data_type.is_integral() || data_type == DatabricksType::Double {
        true => Some(()),
        false => None,
    }
}

/// The literal 0.
fn zero() -> Literal {
    Literal::Number("0".to_owned())
}
