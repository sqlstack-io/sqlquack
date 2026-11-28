//! STRING values read as numbers, BOOLEAN, DATE or TIMESTAMP the way Databricks casts them in ANSI mode, and BIGINT arguments narrowed to INT parameters.

use sqlquack_diagnostics::failures::constants::TEXT_NOT_A_NUMBER;

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::whole_text;
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::constants::{
    BOOLEAN_READING_ERROR, BOOLEAN_TEXT_ERROR, CAST_RULE, DATE_READING_ERROR, DATE_TEXT_FRAGMENT,
    DIGITLESS_DATE_ERROR, DIGITLESS_NUMBER_ERROR, DIGITLESS_TIMESTAMP_ERROR, FALSE_TEXTS,
    INT_PARAMETERS, INVALID_TIMESTAMP_ERROR, NARROW_NUMBER_ERROR, NUMBER_READING_ERROR,
    NUMBER_SHAPED_TEXT, PLAIN_DATE_TEXT, PLAIN_DOUBLE_TEXT, PLAIN_TIMESTAMP_TEXT, TEXT_READINGS,
    TIMESTAMP_READING_ERROR, TIMESTAMP_TEXT_FRAGMENT, TRUE_TEXTS, UNTRIMMED_TEXT,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// A STRING column read as DOUBLE as Databricks casts it: NULL stays NULL, a plain number converts, digitless text raises.
fn text_as_double(text: &Emitted) -> Emitted {
    let t = &text.sql;
    typed(
        format!(
            "(CASE WHEN {t} IS NULL THEN CAST(NULL AS DOUBLE) \
             WHEN regexp_full_match({t}, '{PLAIN_DOUBLE_TEXT}') THEN CAST(trim({t}, ' ') AS DOUBLE) \
             WHEN NOT regexp_matches({t}, '{NUMBER_SHAPED_TEXT}') \
             THEN CAST(error('{TEXT_NOT_A_NUMBER}: {DIGITLESS_NUMBER_ERROR}') AS DOUBLE) \
             ELSE CAST(error('{NUMBER_READING_ERROR}') AS DOUBLE) END)"
        ),
        DatabricksType::Double,
    )
}

/// A STRING column read as DATE as Databricks casts it: NULL stays NULL, `YYYY-MM-DD` converts, digitless text raises.
fn text_as_date(text: &Emitted) -> Emitted {
    let t = &text.sql;
    typed(
        format!(
            "(CASE WHEN {t} IS NULL THEN CAST(NULL AS DATE) \
             WHEN regexp_full_match({t}, '{PLAIN_DATE_TEXT}') THEN CAST({t} AS DATE) \
             WHEN NOT regexp_matches({t}, '[0-9]') \
             THEN CAST(error('{DATE_TEXT_FRAGMENT}: {DIGITLESS_DATE_ERROR}') AS DATE) \
             ELSE CAST(error('{DATE_READING_ERROR}') AS DATE) END)"
        ),
        DatabricksType::Date,
    )
}

/// A STRING column read as a timestamp as Databricks casts it: NULL stays NULL, `YYYY-MM-DD HH:MM:SS[.ffffff]` converts, digitless text raises.
fn text_as_timestamp(text: &Emitted, target: DatabricksType) -> Emitted {
    let (t, name) = (&text.sql, target.duckdb_name());
    typed(
        format!(
            "(CASE WHEN {t} IS NULL THEN CAST(NULL AS {name}) \
             WHEN regexp_full_match({t}, '{PLAIN_TIMESTAMP_TEXT}') \
             THEN coalesce(TRY_CAST({t} AS {name}), CAST(error('{TIMESTAMP_TEXT_FRAGMENT}: {INVALID_TIMESTAMP_ERROR}') AS {name})) \
             WHEN NOT regexp_matches({t}, '[0-9]') \
             THEN CAST(error('{TIMESTAMP_TEXT_FRAGMENT}: {DIGITLESS_TIMESTAMP_ERROR}') AS {name}) \
             ELSE CAST(error('{TIMESTAMP_READING_ERROR}') AS {name}) END)"
        ),
        target,
    )
}

/// A STRING column read as the integer `target` narrower than BIGINT: a whole number outside it raises as malformed text, as Databricks's exact cast does.
fn text_as_narrow(text: &Emitted, target: DatabricksType) -> Option<Emitted> {
    let bits = match target {
        DatabricksType::TinyInt => 7,
        DatabricksType::SmallInt => 15,
        DatabricksType::Int => 31,
        _ => return None,
    };
    let (read, name) = (whole_text(&text.sql), target.duckdb_name());
    let (low, high) = (-(1_i64 << bits), (1_i64 << bits) - 1);
    Some(typed(
        format!(
            "(CASE WHEN {read} IS NULL THEN CAST(NULL AS {name}) \
             WHEN {read} BETWEEN {low} AND {high} THEN CAST({read} AS {name}) \
             ELSE CAST(error('{TEXT_NOT_A_NUMBER}: {NARROW_NUMBER_ERROR}') AS {name}) END)"
        ),
        target,
    ))
}

/// A STRING column read as BOOLEAN as Databricks casts it: the words of truth in any case, other text with untrimmed ends raises as malformed.
fn text_as_boolean(text: &Emitted) -> Emitted {
    let t = &text.sql;
    let words = |texts: &[&str]| {
        texts
            .iter()
            .map(|word| format!("'{word}'"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    typed(
        format!(
            "(CASE WHEN {t} IS NULL THEN CAST(NULL AS BOOLEAN) \
             WHEN lower({t}) IN ({}) THEN TRUE WHEN lower({t}) IN ({}) THEN FALSE \
             WHEN {t} = '' OR regexp_full_match({t}, '{UNTRIMMED_TEXT}') \
             THEN CAST(error('{BOOLEAN_TEXT_ERROR}') AS BOOLEAN) \
             ELSE CAST(error('{BOOLEAN_READING_ERROR}') AS BOOLEAN) END)",
            words(TRUE_TEXTS),
            words(FALSE_TEXTS)
        ),
        DatabricksType::Boolean,
    )
}

/// A STRING column read as `target` as Databricks casts it, `None` for a target no rule reads text as.
pub(crate) fn read_text(value: &Emitted, target: DatabricksType) -> Option<Emitted> {
    match target {
        DatabricksType::Double => Some(text_as_double(value)),
        DatabricksType::BigInt => Some(typed(whole_text(&value.sql), DatabricksType::BigInt)),
        DatabricksType::TinyInt | DatabricksType::SmallInt | DatabricksType::Int => {
            text_as_narrow(value, target)
        }
        DatabricksType::Boolean => Some(text_as_boolean(value)),
        DatabricksType::Date => Some(text_as_date(value)),
        DatabricksType::Timestamp | DatabricksType::TimestampNtz => {
            Some(text_as_timestamp(value, target))
        }
        _ => None,
    }
}

/// The arguments of `name`, each STRING column read as the type Databricks casts it to for `name`.
pub(crate) fn text_arguments(name: &str, values: Vec<Emitted>, site: Site<'_>) -> Vec<Emitted> {
    let Some((_, target, positions)) = TEXT_READINGS
        .iter()
        .find(|(function, _, _)| *function == name)
    else {
        return values;
    };
    let read = |position: usize, value: &Emitted| {
        positions.contains(&position)
            && value.data_type == DatabricksType::String
            && value.literal.is_none()
    };
    if !values
        .iter()
        .enumerate()
        .any(|(position, value)| read(position, value))
    {
        return values;
    }
    let beside: Vec<DatabricksType> = values
        .iter()
        .enumerate()
        .filter(|(position, value)| !read(*position, value))
        .map(|(_, value)| value.data_type)
        .collect();
    let partnered = !beside.is_empty() && beside.iter().all(|data_type| data_type.is_integral());
    if *target == DatabricksType::BigInt && values.len() > 1 && !partnered {
        return values;
    }
    let _rule = site.log.rule(CAST_RULE);
    values
        .into_iter()
        .enumerate()
        .map(|(position, value)| match read(position, &value) {
            true => read_text(&value, *target).unwrap_or(value),
            false => value,
        })
        .collect()
}

/// The arguments of `name` with each BIGINT at an INT parameter narrowed to INT, raising on overflow as Databricks's implicit cast does.
pub(crate) fn narrowed_arguments(name: &str, values: Vec<Emitted>) -> Vec<Emitted> {
    let Some((_, positions)) = INT_PARAMETERS
        .iter()
        .find(|(function, _)| *function == name)
    else {
        return values;
    };
    values
        .into_iter()
        .enumerate()
        .map(|(position, value)| {
            match positions.contains(&position) && value.data_type == DatabricksType::BigInt {
                true => typed(
                    format!("CAST({} AS INTEGER)", value.sql),
                    DatabricksType::Int,
                ),
                false => value,
            }
        })
        .collect()
}
