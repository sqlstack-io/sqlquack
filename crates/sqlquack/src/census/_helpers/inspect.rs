//! One file's census: its constructs and refusal categories, and its diagnostics when blocked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::analysis::models::SourceDialect;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_diagnostics::refusals::types::ConstructPolicy;

use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use sqlquack_semantics::schemas::constants::{UNKNOWN_RELATION_PREFIX, UNKNOWN_RELATION_SUFFIX};
use sqlquack_semantics::snowflake::typing::types::{ColumnSpec, Schema};

use sqlquack_diagnostics::provenance::main::merge_provenance::merge_provenance;
use sqlquack_diagnostics::provenance::models::Provenance;

use crate::analysis::constants::STATEMENT_REFUSAL;
use crate::analysis::main::collect_all_refusals::collect_all_refusals;
use crate::analysis::main::refused_construct::refused_construct;
use crate::census::_helpers::corpus::referenced_model;
use crate::census::_helpers::files::model_name;
use crate::census::_helpers::placeholders::{original_location, rewrite_with_references};
use crate::census::constants::{
    CONVERTED_DIR_VARIABLE, GUARDS_DIR_VARIABLE, READS_REFUSED_MODEL_NOTE,
};
use crate::census::errors::CensusError;
use crate::census::models::{BlockedDiagnostics, CensusFile, PlacedRefusal};
use sqlquack_convert::snowflake::analysis::main::analyze::analyze;
use sqlquack_convert::snowflake::analysis::main::assumption_id::assumption_id;
use sqlquack_convert::snowflake::analysis::main::with_divergence_help::with_divergence_help;
use sqlquack_convert::snowflake::analysis::models::ConversionChoices;
use sqlquack_diagnostics::diagnostics::main::diagnose::diagnose;
use sqlquack_policy::policy::models::Policy;

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
    /// Of those, the ones model references name.
    pub(crate) model_references: BTreeSet<String>,
    /// Relations a refusal found without known columns.
    pub(crate) unknown_relations: BTreeSet<String>,
    /// Every refusal with its position, in source order.
    pub(crate) placed: Vec<PlacedRefusal>,
    /// The first conversion's refusals, located in the original file.
    pub(crate) refusals: Vec<Refusal>,
    /// Whether the file may hold refusals that converting past the others did not reach.
    pub(crate) possibly_incomplete: bool,
}

/// A refused file's census with every refusal's roots, and the columns its conversion reached.
pub(crate) fn complete_refusals(
    (census, path): (&FileCensus, &Path),
    (dialect, cursor_bound): (SourceDialect, Option<&str>),
    policy: &(dyn ConstructPolicy + Sync),
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<(FileCensus, Option<Vec<ColumnSpec>>), CensusError> {
    let started = std::time::Instant::now();
    let mut census = census.clone();
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let sql = rewrite_with_references(&String::from_utf8_lossy(&bytes), cursor_bound).sql;
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

/// The relation a refusal's `reason` found without known columns.
pub(crate) fn unknown_relation(reason: &str) -> Option<String> {
    reason
        .strip_prefix(UNKNOWN_RELATION_PREFIX)?
        .strip_suffix(UNKNOWN_RELATION_SUFFIX)
        .map(str::to_owned)
}

/// The relations refusals found without known columns, and the other refusals' constructs.
fn traced(refusals: &[Refusal]) -> (BTreeSet<String>, BTreeSet<(RefusalCategory, String)>) {
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
    (dialect, cursor_bound): (SourceDialect, Option<&str>),
    policy: &(dyn ConstructPolicy + Sync),
    (schema, options): (&Schema, &ConversionOptions),
) -> Result<FileCensus, CensusError> {
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let original = String::from_utf8_lossy(&bytes);
    let rewritten = rewrite_with_references(&original, cursor_bound);
    let analysis = match dialect {
        SourceDialect::Snowflake => analyze(&rewritten.sql, policy, (schema, options)),
    };
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
    if let (Ok(dir), true, [converted]) = (
        std::env::var(CONVERTED_DIR_VARIABLE),
        analysis.refusals.is_empty(),
        analysis.conversions.as_slice(),
    ) {
        let name = path.to_string_lossy().replace('/', "_");
        let _ = std::fs::write(format!("{dir}/{name}"), &converted.sql);
    }
    let (unknown_relations, root_constructs) = traced(&analysis.refusals);
    let mut placed: Vec<PlacedRefusal> = Vec::new();
    let mut refusals: Vec<Refusal> = Vec::with_capacity(analysis.refusals.len());
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
        refusals.push(Refusal {
            location,
            ..refusal.clone()
        });
    }
    placed.sort();
    Ok(FileCensus {
        placed,
        refusals,
        possibly_incomplete: false,
        elapsed: started.elapsed(),
        references: rewritten.references,
        model_references: rewritten.model_references,
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

/// A blocked file's diagnostics, and the causes of those not blamed on a refused model it reads.
pub(crate) fn diagnosed_file(
    file: &CensusFile,
    refused_models: &BTreeMap<String, &Path>,
    (policy, choices): (Option<&Policy>, &ConversionChoices),
) -> Result<(BlockedDiagnostics, Vec<(String, String)>), CensusError> {
    let source = read_source(&file.path)?;
    let diagnostics = with_divergence_help(
        &file.refusals,
        diagnose(&file.refusals, &source),
        (policy, choices, &BTreeMap::new()),
    );
    let mut own: Vec<(String, String)> = Vec::new();
    let mut shown = Vec::with_capacity(diagnostics.len());
    for (refusal, mut diagnostic) in file.refusals.iter().zip(diagnostics) {
        match blamed_model(refusal, refused_models, &file.path) {
            Some(note) => diagnostic.notes.push(note),
            None => own.push((diagnostic.code.to_owned(), cause(refusal))),
        }
        shown.push(diagnostic);
    }
    let diagnosed = BlockedDiagnostics {
        path: file.path.clone(),
        diagnostics: shown,
        source,
    };
    Ok((diagnosed, own))
}

/// The text of the file at `path`.
fn read_source(path: &Path) -> Result<String, CensusError> {
    let bytes = std::fs::read(path).map_err(|error| CensusError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The note blaming a refusal on another refused model the file reads, if it is one.
fn blamed_model(
    refusal: &Refusal,
    refused_models: &BTreeMap<String, &Path>,
    path: &Path,
) -> Option<String> {
    let relation = unknown_relation(&refusal.reason)?;
    let upstream = refused_models
        .get(&referenced_model(&relation))
        .filter(|upstream| **upstream != path)?;
    Some(READS_REFUSED_MODEL_NOTE.replace("{model}", &model_name(upstream)))
}

/// What a refusal blames: its divergence, its construct's name, or the statement.
fn cause(refusal: &Refusal) -> String {
    if let Some(id) = refusal.divergence() {
        return assumption_id(id).to_owned();
    }
    refusal.construct.as_ref().map_or_else(
        || STATEMENT_REFUSAL.to_owned(),
        |construct| construct.name.clone(),
    )
}
