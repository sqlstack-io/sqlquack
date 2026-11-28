//! STRING values read as DOUBLE, BIGINT, DATE or TIMESTAMP the way Databricks casts them in ANSI mode: plain numbers convert, other text raises.

use sqlquack_diagnostics::failures::constants::TEXT_NOT_A_NUMBER;

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::whole_text;
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::constants::{
    CAST_RULE, DATE_READING_ERROR, DATE_TEXT_FRAGMENT, DIGITLESS_DATE_ERROR,
    DIGITLESS_NUMBER_ERROR, DIGITLESS_TIMESTAMP_ERROR, NUMBER_READING_ERROR, NUMBER_SHAPED_TEXT,
    PLAIN_DATE_TEXT, PLAIN_DOUBLE_TEXT, PLAIN_TIMESTAMP_TEXT, TEXT_READINGS,
    TIMESTAMP_READING_ERROR, TIMESTAMP_TEXT_FRAGMENT,
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
             WHEN regexp_full_match({t}, '{PLAIN_TIMESTAMP_TEXT}') THEN CAST({t} AS {name}) \
             WHEN NOT regexp_matches({t}, '[0-9]') \
             THEN CAST(error('{TIMESTAMP_TEXT_FRAGMENT}: {DIGITLESS_TIMESTAMP_ERROR}') AS {name}) \
             ELSE CAST(error('{TIMESTAMP_READING_ERROR}') AS {name}) END)"
        ),
        target,
    )
}

/// A STRING column read as `target` as Databricks casts it, `None` for a target no rule reads text as.
pub(crate) fn read_text(value: &Emitted, target: DatabricksType) -> Option<Emitted> {
    match target {
        DatabricksType::Double => Some(text_as_double(value)),
        DatabricksType::BigInt => Some(typed(whole_text(&value.sql), DatabricksType::BigInt)),
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
