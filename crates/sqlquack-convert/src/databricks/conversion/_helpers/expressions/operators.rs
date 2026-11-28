//! Operators: arithmetic, comparisons, boolean logic, null tests, `BETWEEN`, `IN`, `CASE` and `LIKE`.

use serde_json::Value;
use sqlquack_diagnostics::failures::constants::TEXT_NOT_A_NUMBER;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::functions::numbers::{quotient, remainder};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{
    digits_value, field, flag, list, refused, understood,
};
use crate::databricks::conversion::_helpers::support::typing::wider_integer;
use crate::databricks::conversion::constants::{
    ARITHMETIC_RULE, BETWEEN_RULE, BOOLEAN_RULE, CASE_RULE, COMPARISON_RULE, DIVISION_RULE,
    IN_LIST_RULE, MALFORMED_NUMBER_ERROR, MALFORMED_NUMBER_TEXT, NOT_NUMBER_CHARACTER,
    NULL_TEST_RULE, UNREAD_NUMBER_ERROR, WHOLE_NUMBER_TEXT,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// `+`, `-` or `*` on integers, both cast to the wider type as Databricks widens them.
pub(crate) fn arithmetic(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands(kind, payload, site)?;
    let _rule = site.log.rule(ARITHMETIC_RULE);
    let Some(wider) = wider_integer(left.data_type, right.data_type) else {
        return Err(refused(
            kind,
            format!(
                "no rule shows DuckDB computes {} {kind} {} as Databricks does",
                left.data_type.name(),
                right.data_type.name()
            ),
        ));
    };
    let symbol = match kind {
        "add" => "+",
        "sub" => "-",
        _ => "*",
    };
    Ok(typed(
        format!(
            "({} {symbol} {})",
            as_type(&left, wider),
            as_type(&right, wider)
        ),
        wider,
    ))
}

/// `/` or `%`: by a nonzero integer constant directly, else raising on a zero divisor as Databricks does.
pub(crate) fn division(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands(kind, payload, site)?;
    let _rule = site.log.rule(DIVISION_RULE);
    let divisor = match &right.literal {
        Some(Literal::Number(number)) => match number.strip_prefix('-') {
            Some(digits) => digits_value(digits).map(|value| (true, value)),
            None => digits_value(number).map(|value| (false, value)),
        },
        _ => None,
    };
    let safe = match (kind, divisor) {
        (_, None | Some((_, 0))) => false,
        ("div", Some(_)) => true,
        (_, Some((negative, _))) => !negative,
    };
    let wider = wider_integer(left.data_type, right.data_type);
    match (safe, wider, kind) {
        (true, Some(_), "div") => Ok(typed(
            format!(
                "({} / {})",
                as_type(&left, DatabricksType::Double),
                as_type(&right, DatabricksType::Double)
            ),
            DatabricksType::Double,
        )),
        (true, Some(wider), _) => Ok(typed(
            format!("({} % {})", as_type(&left, wider), as_type(&right, wider)),
            wider,
        )),
        _ => {
            let untyped = [&left, &right]
                .iter()
                .any(|value| value.data_type == DatabricksType::Null);
            let general = match (untyped, kind) {
                (true, _) => None,
                (false, "div") => quotient(&left, &right),
                (false, _) => remainder(&left, &right),
            };
            general.ok_or_else(|| {
                refused(
                    kind,
                    format!(
                        "no rule converts {} {kind} {}",
                        left.data_type.name(),
                        right.data_type.name()
                    ),
                )
            })
        }
    }
}

/// `=`, `<>`, `<`, `<=`, `>`, `>=` or `<=>` on values both engines compare the same way.
pub(crate) fn comparison(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands(kind, payload, site)?;
    let _rule = site.log.rule(COMPARISON_RULE);
    let symbol = match kind {
        "eq" => "=",
        "neq" => "<>",
        "lt" => "<",
        "lte" => "<=",
        "gt" => ">",
        "gte" => ">=",
        _ => "IS NOT DISTINCT FROM",
    };
    let (left_sql, right_sql) =
        comparable(&left, &right).ok_or_else(|| uncomparable(kind, &left, &right))?;
    Ok(typed(
        format!("({left_sql} {symbol} {right_sql})"),
        DatabricksType::Boolean,
    ))
}

/// `AND` or `OR` on booleans.
pub(crate) fn logical(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands(kind, payload, site)?;
    let _rule = site.log.rule(BOOLEAN_RULE);
    let word = match kind {
        "and" => "AND",
        _ => "OR",
    };
    Ok(typed(
        format!(
            "({} {word} {})",
            boolean(&left, kind)?,
            boolean(&right, kind)?
        ),
        DatabricksType::Boolean,
    ))
}

/// `NOT` on a boolean.
pub(crate) fn not(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let not = understood(payload, "not", &["this"])?;
    let operand = expression(field(not, "this"), site)?;
    let _rule = site.log.rule(BOOLEAN_RULE);
    Ok(typed(
        format!("(NOT {})", boolean(&operand, "not")?),
        DatabricksType::Boolean,
    ))
}

/// `IS [NOT] NULL`.
pub(crate) fn null_test(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let test = understood(payload, "is_null", &["this", "not", "postfix_form"])?;
    let operand = expression(field(test, "this"), site)?;
    let _rule = site.log.rule(NULL_TEST_RULE);
    let not = match flag(test, "not") {
        true => " NOT",
        false => "",
    };
    Ok(typed(
        format!("({} IS{not} NULL)", operand.sql),
        DatabricksType::Boolean,
    ))
}

/// `[NOT] BETWEEN`, which Databricks reads as `x >= low AND x <= high`.
pub(crate) fn between(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let between = understood(payload, "between", &["this", "low", "high", "not"])?;
    let operand = expression(field(between, "this"), site)?;
    let low = expression(field(between, "low"), site)?;
    let high = expression(field(between, "high"), site)?;
    let _rule = site.log.rule(BETWEEN_RULE);
    let (above, low) =
        comparable(&operand, &low).ok_or_else(|| uncomparable("between", &operand, &low))?;
    let (below, high) =
        comparable(&operand, &high).ok_or_else(|| uncomparable("between", &operand, &high))?;
    let range = format!("({above} >= {low} AND {below} <= {high})");
    let sql = match flag(between, "not") {
        true => format!("(NOT {range})"),
        false => range,
    };
    Ok(typed(sql, DatabricksType::Boolean))
}

/// `[NOT] IN` a list whose values share the operand's type, integers widened to the widest.
pub(crate) fn in_list(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let test = understood(payload, "in", &["this", "expressions", "not"])?;
    let operand = expression(field(test, "this"), site)?;
    let mut values = vec![operand];
    for item in list(test, "expressions") {
        values.push(expression(item, site)?);
    }
    let _rule = site.log.rule(IN_LIST_RULE);
    if common_type(&values).is_none() {
        return mixed_in_list(&values, flag(test, "not"));
    }
    let common = common_type(&values).ok_or_else(|| {
        refused(
            "in",
            "no rule shows DuckDB compares the IN list's values as Databricks does".to_owned(),
        )
    })?;
    let mut sqls = values.iter().map(|value| as_type(value, common));
    let operand_sql = sqls.next().unwrap_or_default();
    let not = match flag(test, "not") {
        true => " NOT",
        false => "",
    };
    Ok(typed(
        format!(
            "({operand_sql}{not} IN ({}))",
            sqls.collect::<Vec<_>>().join(", ")
        ),
        DatabricksType::Boolean,
    ))
}

/// `[NOT] IN` a list whose values Databricks compares with the operand by casting, such as text with integers: one equality each, which keeps IN's NULLs.
fn mixed_in_list(values: &[Emitted], not: bool) -> Result<Emitted, Refusal> {
    let [operand, items @ ..] = values else {
        return Err(refused("in", "an IN list holds no values".to_owned()));
    };
    let mut tests: Vec<String> = Vec::new();
    for item in items {
        let (left, right) = comparable(operand, item).ok_or_else(|| {
            refused(
                "in",
                "no rule shows DuckDB compares the IN list's values as Databricks does".to_owned(),
            )
        })?;
        tests.push(format!("({left} = {right})"));
    }
    let any = format!("({})", tests.join(" OR "));
    Ok(typed(
        match not {
            true => format!("(NOT {any})"),
            false => any,
        },
        DatabricksType::Boolean,
    ))
}

/// A `CASE`, simple or searched, whose results share one type, integers widened to the widest.
pub(crate) fn case(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let case = understood(payload, "case", &["operand", "whens", "else_"])?;
    let operand = match field(case, "operand") {
        Value::Null => None,
        node => Some(expression(node, site)?),
    };
    let mut conditions: Vec<String> = Vec::new();
    let mut results: Vec<Emitted> = Vec::new();
    for when in list(case, "whens") {
        let [condition, result] = when.as_array().map_or(&[][..], Vec::as_slice) else {
            return Err(refused(
                "case",
                "a WHEN is not a condition and a result".to_owned(),
            ));
        };
        let condition = expression(condition, site)?;
        conditions.push(match &operand {
            Some(operand) => {
                let (left, right) = comparable(operand, &condition)
                    .ok_or_else(|| uncomparable("case", operand, &condition))?;
                format!("({left} = {right})")
            }
            None => boolean(&condition, "case")?,
        });
        results.push(expression(result, site)?);
    }
    let otherwise = match field(case, "else_") {
        Value::Null => None,
        node => Some(expression(node, site)?),
    };
    let _rule = site.log.rule(CASE_RULE);
    let all: Vec<Emitted> = results.iter().chain(&otherwise).cloned().collect();
    let common = common_type(&all).filter(|common| *common != DatabricksType::Null);
    let Some(common) = common else {
        return Err(refused(
            "case",
            "no rule shows the CASE results share one type in both engines".to_owned(),
        ));
    };
    let mut sql = "CASE".to_owned();
    for (condition, result) in conditions.iter().zip(&results) {
        sql.push_str(&format!(
            " WHEN {condition} THEN {}",
            as_type(result, common)
        ));
    }
    if let Some(otherwise) = &otherwise {
        sql.push_str(&format!(" ELSE {}", as_type(otherwise, common)));
    }
    sql.push_str(" END");
    Ok(typed(sql, common))
}

/// The type a list of values compares or unifies as: one shared type, or the widest integer.
pub(crate) fn common_type(values: &[Emitted]) -> Option<DatabricksType> {
    let mut common = DatabricksType::Null;
    for value in values {
        common = match (common, value.data_type) {
            (common, DatabricksType::Null) => common,
            (DatabricksType::Null, data_type) => data_type,
            (common, data_type) if common == data_type => common,
            (common, data_type) => wider_integer(common, data_type)?,
        };
    }
    Some(common)
}

/// `value`'s SQL, cast to `data_type` when its own type differs and is not NULL.
pub(crate) fn as_type(value: &Emitted, data_type: DatabricksType) -> String {
    match value.data_type == data_type || value.data_type == DatabricksType::Null {
        true => value.sql.clone(),
        false => format!("CAST({} AS {})", value.sql, data_type.duckdb_name()),
    }
}

/// Both sides as DuckDB compares them as Databricks does: exact numbers by value, DOUBLE as DOUBLE.
pub(crate) fn comparable(left: &Emitted, right: &Emitted) -> Option<(String, String)> {
    use DatabricksType as T;
    let exact = |data_type: T| data_type.is_integral() || matches!(data_type, T::Decimal { .. });
    match (left.data_type, right.data_type) {
        (T::Array(_), _) | (_, T::Array(_)) => None,
        (T::Null, _) | (_, T::Null) => Some((left.sql.clone(), right.sql.clone())),
        (left_type, right_type) if left_type == right_type => {
            Some((left.sql.clone(), right.sql.clone()))
        }
        (T::String, T::Int | T::BigInt) => Some((whole_text(&left.sql), as_type(right, T::BigInt))),
        (T::Int | T::BigInt, T::String) => Some((as_type(left, T::BigInt), whole_text(&right.sql))),
        (left_type, right_type) if left_type.is_integral() && right_type.is_integral() => {
            let wider = wider_integer(left_type, right_type)?;
            Some((as_type(left, wider), as_type(right, wider)))
        }
        (left_type, right_type)
            if (exact(left_type) || left_type == T::Double)
                && (exact(right_type) || right_type == T::Double) =>
        {
            Some((left.sql.clone(), right.sql.clone()))
        }
        _ => None,
    }
}

/// Text read as BIGINT as Databricks reads it against an integer: NULL stays NULL, signed digits between spaces convert, and other text raises.
pub(crate) fn whole_text(text: &str) -> String {
    format!(
        "(CASE WHEN {text} IS NULL THEN CAST(NULL AS BIGINT) \
         WHEN regexp_full_match({text}, '{WHOLE_NUMBER_TEXT}') THEN CAST(trim({text}, ' ') AS BIGINT) \
         WHEN {text} = '' OR regexp_matches({text}, '{NOT_NUMBER_CHARACTER}') \
         OR regexp_full_match({text}, '{MALFORMED_NUMBER_TEXT}') \
         THEN CAST(error('{TEXT_NOT_A_NUMBER}: {MALFORMED_NUMBER_ERROR}') AS BIGINT) \
         ELSE CAST(error('{UNREAD_NUMBER_ERROR}') AS BIGINT) END)"
    )
}

pub(crate) fn uncomparable(kind: &str, left: &Emitted, right: &Emitted) -> Refusal {
    refused(
        kind,
        format!(
            "no rule shows DuckDB compares {} with {} as Databricks does",
            left.data_type.name(),
            right.data_type.name()
        ),
    )
}

/// A boolean operand's SQL, refusing any other type.
pub(crate) fn boolean(operand: &Emitted, kind: &str) -> Result<String, Refusal> {
    match operand.data_type {
        DatabricksType::Boolean | DatabricksType::Null => Ok(operand.sql.clone()),
        other => Err(refused(
            kind,
            format!("no rule reads a {} as a boolean", other.name()),
        )),
    }
}

/// The two operands of a binary operator.
pub(crate) fn operands(
    kind: &str,
    payload: &Value,
    site: Site<'_>,
) -> Result<(Emitted, Emitted), Refusal> {
    let operator = understood(payload, kind, &["left", "right"])?;
    Ok((
        expression(field(operator, "left"), site)?,
        expression(field(operator, "right"), site)?,
    ))
}
