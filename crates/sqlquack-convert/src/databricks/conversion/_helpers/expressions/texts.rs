//! Text read as DOUBLE the way Databricks casts a STRING argument in ANSI mode: plain numbers convert, digitless text raises, the rest raises loudly.

use sqlquack_diagnostics::failures::constants::TEXT_NOT_A_NUMBER;

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::constants::{
    CAST_RULE, DIGITLESS_NUMBER_ERROR, DOUBLE_ARGUMENT_FUNCTIONS, NUMBER_READING_ERROR,
    NUMBER_SHAPED_TEXT, PLAIN_DOUBLE_TEXT,
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

/// The arguments of `name`, each STRING column read as DOUBLE when `name` takes only DOUBLE parameters.
pub(crate) fn text_arguments(name: &str, values: Vec<Emitted>, site: Site<'_>) -> Vec<Emitted> {
    let reads = DOUBLE_ARGUMENT_FUNCTIONS.contains(&name)
        && values
            .iter()
            .any(|value| value.data_type == DatabricksType::String && value.literal.is_none());
    if !reads {
        return values;
    }
    let _rule = site.log.rule(CAST_RULE);
    values
        .into_iter()
        .map(|value| match (value.data_type, &value.literal) {
            (DatabricksType::String, None) => text_as_double(&value),
            _ => value,
        })
        .collect()
}
