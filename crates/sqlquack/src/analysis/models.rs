//! Pipeline results.

use std::collections::BTreeSet;

use serde::Serialize;
use sqlquack_core::classification::models::Occurrence;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_core::reparse::models::FidelityOutcome;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::analysis::main::analyze::analyze;

/// What the pipeline found in one SQL input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Analysis {
    /// Whether the input parsed.
    pub parsed: bool,
    /// Statements parsed.
    pub statements: usize,
    /// Every construct used, in source order.
    pub occurrences: Vec<Occurrence>,
    /// The source print-back check for each statement.
    pub fidelity: Vec<FidelityOutcome>,
    /// Every refusal; the input converts only when this is empty.
    pub refusals: Vec<Refusal>,
}

/// The production converter: the SQLQuack pipeline with the built-in rule catalogue.
#[derive(Clone, Copy, Debug, Default)]
pub struct PipelineConverter;

impl CaseConverter for PipelineConverter {
    fn convert(&self, case: &LoadedCase) -> Conversion {
        let analysis = analyze(
            &case.source_sql,
            SourceDialect::Snowflake,
            &builtin_catalogue(),
        );
        let reasons: BTreeSet<String> = analysis
            .refusals
            .iter()
            .map(|refusal| format!("{}: {}", refusal.category, refusal.reason))
            .collect();
        if reasons.is_empty() {
            return Conversion::Refused(vec![
                "no rule emits DuckDB SQL for this case yet".to_owned(),
            ]);
        }
        Conversion::Refused(reasons.into_iter().collect())
    }
}
