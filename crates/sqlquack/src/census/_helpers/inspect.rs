//! One file's census: its constructs and refusal categories.

use std::collections::BTreeSet;
use std::path::Path;

use sqlquack_core::classification::models::Construct;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_core::refusals::types::ConstructPolicy;

use sqlquack_rules::conversion::constants::{UNKNOWN_RELATION_PREFIX, UNKNOWN_RELATION_SUFFIX};
use sqlquack_rules::conversion::models::{ColumnSpec, ConversionOptions, Schema};

use sqlquack_core::provenance::main::merge_provenance::merge_provenance;
use sqlquack_core::provenance::models::Provenance;

use crate::analysis::main::analyze::analyze;
use crate::analysis::main::collect_all_refusals::collect_all_refusals;
use crate::analysis::main::refused_construct::refused_construct;
use crate::census::_helpers::placeholders::{original_location, rewrite_with_references};
use crate::census::constants::GUARDS_DIR_VARIABLE;
use crate::census::errors::CensusError;
use crate::census::models::PlacedRefusal;

#[derive(Clone)]
pub(crate) struct FileCensus {
    pub(crate) parsed: bool,
    pub(crate) statements: usize,
    pub(crate) constructs: BTreeSet<Construct>,
    pub(crate) refusal_categories: BTreeSet<RefusalCategory>,
    pub(crate) refusal_constructs: BTreeSet<(RefusalCategory, String)>,
    /// The refusals other than an unknown relation's, which blockers trace to their own roots.
    pub(crate) root_constructs: BTreeSet<(RefusalCategory, String)>,
    pub(crate) outputs: Option<Vec<ColumnSpec>>,
    pub(crate) provenance: Option<Provenance>,
    pub(crate) elapsed: std::time::Duration,
    /// The placeholder relations the file references, in upper case.
    pub(crate) references: BTreeSet<String>,
    /// Relations a refusal found without known columns.
    pub(crate) unknown_relations: BTreeSet<String>,
    /// Every refusal with its position, in source order.
    pub(crate) placed: Vec<PlacedRefusal>,
    /// Whether the file may hold refusals that converting past the others did not reach.
    pub(crate) possibly_incomplete: bool,
}

/// A refused file's census with every refusal's roots, and the columns its conversion reached.
pub(crate) fn complete_refusals(
    (census, path): (&FileCensus, &Path),
    dialect: SourceDialect,
    policy: &(dyn ConstructPolicy + Sync),
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<(FileCensus, Option<Vec<ColumnSpec>>), CensusError> {
    let started = std::time::Instant::now();
    let mut census = census.clone();
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let sql = rewrite_with_references(&String::from_utf8_lossy(&bytes)).sql;
    let collected = collect_all_refusals(&sql, dialect, policy, (schema, options));
    if !collected.refusals.is_empty() {
        let (unknown_relations, root_constructs) = traced(&collected.refusals);
        census.unknown_relations.extend(unknown_relations);
        census.root_constructs = root_constructs;
        census.possibly_incomplete = collected.possibly_incomplete;
    }
    census.elapsed += started.elapsed();
    Ok((census, collected.columns))
}

/// The relations refusals found without known columns, and the other refusals' constructs.
fn traced(refusals: &[Refusal]) -> (BTreeSet<String>, BTreeSet<(RefusalCategory, String)>) {
    let unknown_relation = |reason: &str| {
        reason
            .strip_prefix(UNKNOWN_RELATION_PREFIX)?
            .strip_suffix(UNKNOWN_RELATION_SUFFIX)
            .map(str::to_owned)
    };
    let unknown_relations: BTreeSet<String> = refusals
        .iter()
        .filter_map(|refusal| unknown_relation(&refusal.reason))
        .collect();
    let root_constructs: BTreeSet<(RefusalCategory, String)> = refusals
        .iter()
        .filter(|refusal| unknown_relation(&refusal.reason).is_none())
        .map(|refusal| (refusal.category, refused_construct(refusal)))
        .collect();
    (unknown_relations, root_constructs)
}

pub(crate) fn inspect_file(
    path: &Path,
    dialect: SourceDialect,
    policy: &(dyn ConstructPolicy + Sync),
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<FileCensus, CensusError> {
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let original = String::from_utf8_lossy(&bytes);
    let rewritten = rewrite_with_references(&original);
    let analysis = analyze(&rewritten.sql, dialect, policy, (schema, options));
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
    let (unknown_relations, root_constructs) = traced(&analysis.refusals);
    let mut placed: Vec<PlacedRefusal> = Vec::new();
    for refusal in &analysis.refusals {
        let location = refusal
            .location
            .map(|location| original_location(&rewritten, &original, location));
        placed.push(PlacedRefusal {
            line: location.map(|location| location.line),
            column: location.map(|location| location.column),
            category: refusal.category.label().to_owned(),
            construct: refused_construct(refusal),
        });
    }
    placed.sort();
    Ok(FileCensus {
        placed,
        possibly_incomplete: false,
        elapsed: started.elapsed(),
        references: rewritten.references,
        unknown_relations,
        root_constructs,
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
