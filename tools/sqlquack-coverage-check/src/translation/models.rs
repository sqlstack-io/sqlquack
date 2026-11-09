//! What conversion made of each counted case, each form, and each family.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::formats::models::SyntaxCoverage;

/// Where the measure's inputs are and how conversion runs.
#[derive(Debug)]
pub struct TranslationInputs<'a> {
    pub coverage: &'a SyntaxCoverage,
    /// The coverage root, holding the census sidecars.
    pub root: &'a Path,
    /// Constructs with a current by-design sign-off.
    pub by_design: &'a BTreeSet<String>,
    pub binary: &'a Path,
    pub policy: &'a Path,
    /// A scratch directory for each case's project of census relations.
    pub work: &'a Path,
    pub family: Option<&'a str>,
    pub jobs: usize,
}

/// What conversion made of one case Snowflake parsed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum CaseOutcome {
    /// Converted, DuckDB bound it; `binds` is none when its setup cannot be replayed.
    Translated { binds: Option<bool> },
    /// Refused only for constructs the signed-off by-design list names.
    ByDesign { constructs: Vec<String> },
    /// Refused only because a known divergence can return another value.
    DeclaredValueDivergence { constructs: Vec<String> },
    /// Refused for anything else, each refusal as `category: construct`.
    NotYetTranslated { refusals: Vec<String> },
    /// Refused only for unresolved names even with its census schema.
    NeedsSchema,
    /// Refused only for unresolved names, with no census schema: a syntax probe, never scored.
    NoRelationInScope,
    /// Conversion ran past its bound or failed to report; not translated.
    DidNotFinish { cause: String },
}

impl CaseOutcome {
    /// Whether the case is measured at all: needs-schema and no-relation cases are not.
    pub fn measured(&self) -> bool {
        !matches!(self, Self::NeedsSchema | Self::NoRelationInScope)
    }
}

/// One counted case and what conversion made of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaseTranslation {
    pub case_id: String,
    pub forms: Vec<String>,
    /// Whether conversion read a census schema for it.
    pub census_schema: bool,
    /// Whether its setup is empty, so binding is measured.
    pub empty_setup: bool,
    #[serde(flatten)]
    pub outcome: CaseOutcome,
}

/// A form's translation state over its counted cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormState {
    /// Every measured case converts.
    Translates,
    /// Every case not translated is refused by design.
    ByDesign,
    /// Every case not translated is a declared value divergence, and none is refused otherwise.
    DeclaredValueDivergence,
    NotYetTranslated,
    /// No measured case, and at least one needs a schema.
    NeedsSchema,
    /// No measured case: only syntax probes with no relation in scope.
    NoRelationInScope,
    /// No case Snowflake parsed, so outside the denominator.
    NotMeasurable,
}

/// One form's translation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FormTranslation {
    pub id: String,
    pub family: String,
    pub state: FormState,
    pub counted_cases: usize,
    pub measured_cases: usize,
    pub translated_cases: usize,
    /// Whether every measured case with an empty setup binds; none when no such case exists.
    pub binds: Option<bool>,
    /// The refusals of its cases not yet translated, sorted and distinct.
    pub refusals: Vec<String>,
}

/// One family's counts, or the whole catalogue's.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TranslationTally {
    pub forms: usize,
    pub not_measurable_forms: usize,
    pub counted_forms: usize,
    pub needs_schema_forms: usize,
    pub no_relation_in_scope_forms: usize,
    /// Counted forms less needs-schema and no-relation forms: the translated percentage's base.
    pub measurable_forms: usize,
    pub translated_forms: usize,
    pub by_design_forms: usize,
    pub declared_value_divergence_forms: usize,
    pub not_yet_translated_forms: usize,
    pub binding_forms: usize,
    pub bind_measured_forms: usize,
    pub counted_cases: usize,
    pub translated_cases: usize,
    pub needs_schema_cases: usize,
    pub no_relation_in_scope_cases: usize,
    pub did_not_finish_cases: usize,
}

/// The whole report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TranslationReport {
    pub total: TranslationTally,
    pub families: Vec<(String, TranslationTally)>,
    /// Not-yet-translated refusals by how many forms each blocks, most first.
    pub refusals: Vec<(String, usize)>,
    pub results_match: String,
    pub forms: Vec<FormTranslation>,
    pub cases: Vec<CaseTranslation>,
}

/// `by-design.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByDesignFile {
    #[serde(default, rename = "by_design")]
    pub entries: Vec<ByDesignEntry>,
}

/// A construct DuckDB has no equivalent for, refused by design.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByDesignEntry {
    /// The construct as convert reports it, such as `fn:SYSTEM$WAIT`.
    pub construct: String,
    pub reason: String,
    pub duckdb_note: String,
}

/// `reviews/by-design.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByDesignReviewFile {
    #[serde(default, rename = "signoff")]
    pub signoffs: Vec<ByDesignSignoff>,
}

/// A reviewer's sign-off of one entry, bound to its construct and the SHA-256 of its reason.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByDesignSignoff {
    pub construct: String,
    pub reason_sha256: String,
    pub reviewed_utc: String,
}

/// One census sidecar line: the relations a case reads, with typed columns.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CensusEntry {
    pub case_id: String,
    pub relations: Vec<CensusRelation>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CensusRelation {
    pub name: String,
    pub columns: Vec<CensusColumn>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CensusColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
}

/// What conversion of one case reported, as `sqlquack convert --json` prints it.
#[derive(Clone, Debug, Deserialize)]
pub struct ConvertOutput {
    pub converted: bool,
    pub refusals: Vec<ConvertRefusal>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConvertRefusal {
    pub category: String,
    pub construct: String,
}
