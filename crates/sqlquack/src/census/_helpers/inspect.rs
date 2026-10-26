//! One file's census: its constructs and refusal categories.

use std::collections::BTreeSet;
use std::path::Path;

use sqlquack_core::classification::models::Construct;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_core::refusals::types::ConstructPolicy;

use sqlquack_rules::conversion::models::{ColumnSpec, ConversionOptions, Schema};

use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::provenance::models::Provenance;

use crate::analysis::main::analyze::analyze;
use crate::analysis::main::refused_construct::refused_construct;
use crate::census::_helpers::placeholders::rewrite_placeholders;
use crate::census::constants::GUARDS_DIR_VARIABLE;
use crate::census::errors::CensusError;

pub(crate) struct FileCensus {
    pub(crate) parsed: bool,
    pub(crate) statements: usize,
    pub(crate) constructs: BTreeSet<Construct>,
    pub(crate) refusal_categories: BTreeSet<RefusalCategory>,
    pub(crate) refusal_constructs: BTreeSet<(RefusalCategory, String)>,
    pub(crate) outputs: Option<Vec<ColumnSpec>>,
    pub(crate) provenance: Option<Provenance>,
}

pub(crate) fn inspect_file(
    path: &Path,
    dialect: SourceDialect,
    policy: &(dyn ConstructPolicy + Sync),
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<FileCensus, CensusError> {
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let sql = rewrite_placeholders(&String::from_utf8_lossy(&bytes));
    let analysis = analyze(&sql, dialect, policy, (schema, options));
    let outputs = match (
        analysis.refusals.is_empty(),
        analysis.conversions.as_slice(),
    ) {
        (true, [converted]) => Some(converted.columns.clone()),
        _ => None,
    };
    let provenance = match analysis.refusals.is_empty() && analysis.parsed {
        true => Some(merge_provenance(
            analysis
                .conversions
                .iter()
                .map(|converted| &converted.provenance),
        )),
        false => None,
    };
    if let (Ok(dir), Some(provenance)) = (std::env::var(GUARDS_DIR_VARIABLE), &provenance)
        && !provenance.divergences.is_empty()
    {
        let name = path.to_string_lossy().replace('/', "_");
        let text = serde_json::json!({
            "path": path.to_string_lossy(),
            "divergences": provenance.divergences,
        });
        let _ = std::fs::write(format!("{dir}/{name}.json"), text.to_string());
    }
    Ok(FileCensus {
        outputs,
        provenance,
        parsed: analysis.parsed,
        statements: analysis.statements,
        constructs: analysis
            .occurrences
            .into_iter()
            .map(|occurrence| occurrence.construct)
            .collect(),
        refusal_categories: analysis
            .refusals
            .iter()
            .map(|refusal| refusal.category)
            .collect(),
        refusal_constructs: analysis
            .refusals
            .iter()
            .map(|refusal| (refusal.category, refused_construct(refusal)))
            .collect(),
    })
}
