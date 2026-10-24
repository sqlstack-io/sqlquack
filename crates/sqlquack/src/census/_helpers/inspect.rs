//! One file's census: its constructs, print-back outcome and refusal categories.

use std::collections::BTreeSet;
use std::path::Path;

use sqlquack_core::classification::models::Construct;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_core::refusals::types::ConstructPolicy;
use sqlquack_core::reparse::models::FidelityOutcome;

use crate::analysis::main::analyze::analyze;
use crate::census::_helpers::placeholders::rewrite_placeholders;
use crate::census::errors::CensusError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FidelityClass {
    Identical,
    Canonicalised,
    Unstable,
    Failed,
}

pub(crate) struct FileCensus {
    pub(crate) parsed: bool,
    pub(crate) statements: usize,
    pub(crate) constructs: BTreeSet<Construct>,
    pub(crate) fidelity: Option<FidelityClass>,
    pub(crate) refusal_categories: BTreeSet<RefusalCategory>,
}

pub(crate) fn inspect_file(
    path: &Path,
    dialect: SourceDialect,
    policy: &(dyn ConstructPolicy + Sync),
) -> Result<FileCensus, CensusError> {
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let sql = rewrite_placeholders(&String::from_utf8_lossy(&bytes));
    let analysis = analyze(&sql, dialect, policy);
    Ok(FileCensus {
        parsed: analysis.parsed,
        statements: analysis.statements,
        constructs: analysis
            .occurrences
            .into_iter()
            .map(|occurrence| occurrence.construct)
            .collect(),
        fidelity: analysis.fidelity.iter().map(fidelity_class).max(),
        refusal_categories: analysis
            .refusals
            .iter()
            .map(|refusal| refusal.category)
            .collect(),
    })
}

fn fidelity_class(outcome: &FidelityOutcome) -> FidelityClass {
    match outcome {
        FidelityOutcome::Identical => FidelityClass::Identical,
        FidelityOutcome::Canonicalised { .. } => FidelityClass::Canonicalised,
        FidelityOutcome::Unstable { .. } => FidelityClass::Unstable,
        FidelityOutcome::PrintFailed { .. } | FidelityOutcome::ReparseFailed { .. } => {
            FidelityClass::Failed
        }
    }
}
