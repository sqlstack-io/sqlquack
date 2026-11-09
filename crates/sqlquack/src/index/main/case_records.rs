//! `case_records`: one conversion record per conversion case, recorded result or not.

use sqlquack_evidence::cases::models::{CaseRole, LoadedCase};
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;

use crate::index::models::{ConversionRecord, RecordSource};

/// What `converter` relied on, or refused, for each case; typing cases verify no conversion.
pub fn case_records(cases: &[LoadedCase], converter: &dyn CaseConverter) -> Vec<ConversionRecord> {
    cases
        .iter()
        .filter(|case| case.role != CaseRole::Typing)
        .map(|case| {
            let source = RecordSource::Case(case.reference.clone());
            match converter.convert(case) {
                Conversion::Converted { provenance, .. } => ConversionRecord {
                    source,
                    provenance: Some(provenance),
                    refused_constructs: Vec::new(),
                },
                Conversion::Refused { constructs, .. } => ConversionRecord {
                    source,
                    provenance: None,
                    refused_constructs: constructs,
                },
            }
        })
        .collect()
}
