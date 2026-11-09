//! `measure_translation`: what conversion makes of every case Snowflake parsed, by form.

use std::collections::BTreeMap;

use crate::translation::_helpers::cases::{convert_cases, queued_cases};
use crate::translation::_helpers::census::load_census;
use crate::translation::_helpers::forms::{catalogued_forms, report};
use crate::translation::models::{CensusEntry, TranslationInputs, TranslationReport};

/// The report, or every problem with the census sidecars.
pub fn measure_translation(inputs: &TranslationInputs) -> Result<TranslationReport, Vec<String>> {
    let census: BTreeMap<String, CensusEntry> = load_census(inputs.root, inputs.coverage)?;
    let catalogued: BTreeMap<&str, &str> = catalogued_forms(inputs.coverage, inputs.family);
    let queue = queued_cases(inputs.coverage, &catalogued);
    Ok(report(convert_cases(&queue, &census, inputs), &catalogued))
}
