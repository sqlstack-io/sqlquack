//! STRING arguments read as DOUBLE or BIGINT the way Databricks casts them in ANSI mode: plain numbers convert, other text raises.

use sqlquack_diagnostics::failures::constants::TEXT_NOT_A_NUMBER;

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::whole_text;
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::constants::{
    CAST_RULE, DIGITLESS_NUMBER_ERROR, NUMBER_READING_ERROR, NUMBER_SHAPED_TEXT, PLAIN_DOUBLE_TEXT,
    TEXT_READINGS,
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

/// The arguments of `name`, each STRING column read as the type Databricks casts it to for `name`.
pub(crate) fn text_arguments(name: &str, values: Vec<Emitted>, site: Site<'_>) -> Vec<Emitted> {
    let Some((_, target)) = TEXT_READINGS.iter().find(|(function, _)| *function == name) else {
        return values;
    };
    let reads = values
        .iter()
        .any(|value| value.data_type == DatabricksType::String && value.literal.is_none());
    if !reads {
        return values;
    }
    let _rule = site.log.rule(CAST_RULE);
    values
        .into_iter()
        .map(|value| match (value.data_type, &value.literal, target) {
            (DatabricksType::String, None, DatabricksType::Double) => text_as_double(&value),
            (DatabricksType::String, None, _) => {
                typed(whole_text(&value.sql), DatabricksType::BigInt)
            }
            _ => value,
        })
        .collect()
}
