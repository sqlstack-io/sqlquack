//! Declared rule domains, the hazard catalogue, and the coverage report.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlquack_core::provenance::models::Shape;

use crate::cases::models::LoadedCase;

/// One file of declared rule domains.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainFile {
    /// The domain format version.
    pub version: u32,
    /// The rules it declares.
    #[serde(default, rename = "rule")]
    pub rules: Vec<RuleDomain>,
}

/// What one rule accepts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDomain {
    /// The rule id.
    pub id: String,
    /// Why the accepted domain cannot be stated as forms; reported as a finding.
    #[serde(default)]
    pub undeclarable: Option<String>,
    /// The forms it accepts; their combinations are its accepted domain.
    #[serde(default, rename = "form")]
    pub forms: Vec<Form>,
    /// Shapes the code accepts outside the domain, for generated cases to decide; never accepted.
    #[serde(default, rename = "probe")]
    pub probes: Vec<Form>,
}

/// One accepted form of a rule: a construct, its arguments' families and kinds, and facets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    /// The construct key shapes record, such as `fn:GET`, `node:like` or `query`.
    pub construct: String,
    /// Constructs accepted alike, such as other comparison operators; hazards count once for all.
    #[serde(default)]
    pub also: Vec<String>,
    /// The arguments, in source order; absent when arguments are not a dimension of the form.
    #[serde(default)]
    pub arguments: Option<Vec<ArgumentDomain>>,
    /// Whether the last argument repeats, as in a list of patterns.
    #[serde(default)]
    pub repeat: bool,
    /// Whether each position's families need a case alone, not crossed with the other positions.
    #[serde(default)]
    pub one_wise: bool,
    /// Further dimensions and their accepted values; a shape without one records `none`.
    #[serde(default)]
    pub facets: BTreeMap<String, Vec<String>>,
    /// Facets whose values each need one passing case, not crossed, such as a partition family.
    #[serde(default)]
    pub each: BTreeMap<String, Vec<String>>,
    /// Benign cells generated cases use at given argument positions, overriding the family's.
    #[serde(default)]
    pub benign: BTreeMap<String, String>,
    /// Snowflake types generated cases declare at given argument positions.
    #[serde(default)]
    pub sql_types: BTreeMap<String, String>,
    /// `group` or `window` when the form reads rows, adding that family's hazards.
    #[serde(default)]
    pub rows: Option<String>,
    /// The expression generated cases call, `{0}`, `{1}`… standing for the arguments.
    #[serde(default)]
    pub template: Option<String>,
    /// Rows, cells per non-literal argument, for one further case: ties the hazards cannot pair.
    #[serde(default)]
    pub extra_rows: Vec<Vec<String>>,
    /// Further templates for the constructs and facet values generated cases also exercise.
    #[serde(default)]
    pub variants: Vec<String>,
    /// A query generated cases run instead, over `input`, with `{expression}` for the template.
    #[serde(default)]
    pub query: Option<String>,
    /// The case category generated cases are filed under.
    #[serde(default)]
    pub category: Option<String>,
    /// Hazard id, `family.id` or `position.family.id` to the divergence its case expects.
    #[serde(default)]
    pub divergences: BTreeMap<String, String>,
    /// `position.family.id` hazards generated cases hold alone, expecting no divergence.
    #[serde(default)]
    pub alone: Vec<String>,
    /// `position.family.id` to why no case of this form can record the hazard there.
    #[serde(default)]
    pub unrecordable: BTreeMap<String, String>,
    /// Argument positions generated cases fill with each row's number, so ordering keys never tie.
    #[serde(default)]
    pub ordinal: Vec<String>,
    /// Whether generated grouped cases give each row its own group.
    #[serde(default)]
    pub row_groups: bool,
}

/// One argument position: the families and, when the rule tells them apart, kinds it accepts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgumentDomain {
    /// Type families, such as `text`, `integer` or `variant`.
    pub families: Vec<String>,
    /// `literal` and `value`; absent when the rule treats both alike.
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
    /// Whether the position is a grammar, such as a format or a lambda body, taking no hazards.
    #[serde(default)]
    pub grammar: bool,
}

/// The hazard catalogue: known-dangerous inputs per type family, and per kind of row set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardCatalogue {
    /// The catalogue version; raised whenever hazards are added, changed or removed.
    pub version: u32,
    /// Each family's hazards.
    pub families: BTreeMap<String, FamilyHazards>,
}

/// The hazards of one type family, or of row sets for `group` and `window`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyHazards {
    /// The Snowflake type generated cases declare for the family.
    #[serde(default)]
    pub sql_type: Option<String>,
    /// An unremarkable value of the family, for the arguments a case does not vary.
    #[serde(default)]
    pub benign: Option<String>,
    /// A second unremarkable value, different from `benign`.
    #[serde(default)]
    pub other: Option<String>,
    /// A third unremarkable value, different from both, for forms taking three of the family.
    #[serde(default)]
    pub third: Option<String>,
    /// Further distinct unremarkable values, for forms taking more than three of the family.
    #[serde(default)]
    pub more: Vec<String>,
    /// The hazards.
    #[serde(default, rename = "hazard")]
    pub hazards: Vec<Hazard>,
}

/// One known-dangerous input.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hazard {
    /// A stable id, unique in its family.
    pub id: String,
    /// What makes it dangerous, and the finding it comes from when one does.
    pub description: String,
    /// Cells, as source literals, that are the hazard; generated cases use them.
    #[serde(default)]
    pub cells: Vec<String>,
    /// Text whose presence in a cell also makes it the hazard.
    #[serde(default)]
    pub contains: Vec<String>,
    /// `case_only`: two cells of one column differing only by case show it; `cells` only generate.
    #[serde(default)]
    pub pair: Option<String>,
    /// Whether generated cases hold the hazard alone, since it often makes an engine raise.
    #[serde(default)]
    pub isolate: bool,
    /// For row sets: `all_null`, `mixed`, `null_beside_value`, `duplicates` or `single_row`.
    #[serde(default)]
    pub group: Option<String>,
    /// For row sets: text in the case's SQL that makes it the hazard, such as an empty frame.
    #[serde(default)]
    pub sql_contains: Vec<String>,
    /// For row sets: the query generated cases run, `{expression}` standing for the call.
    #[serde(default)]
    pub query: Option<String>,
    /// For row sets: the rows generated cases use, as values of the group and one argument.
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
    /// The divergence every case holding the hazard relies on; its cases hold it alone.
    #[serde(default)]
    pub divergence: Option<String>,
    /// Why no case can record the hazard; it is then neither covered nor a gap, only reported.
    #[serde(default)]
    pub unrecordable: Option<String>,
}

/// A passing case's shape for a rule, as coverage reads it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Covering<'a> {
    /// The shape.
    pub(crate) shape: &'a Shape,
    /// The case.
    pub(crate) case: &'a LoadedCase,
    /// Whether its recording is an error, which hides every row but the one raising.
    pub(crate) errored: bool,
    /// How many shapes the rule has in the case.
    pub(crate) alongside: usize,
}

/// Coverage of every declared rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CoverageReport {
    /// The catalogue version the report used.
    pub catalogue_version: u32,
    /// One entry per rule, sorted by id.
    pub rules: Vec<RuleCoverage>,
    /// Totals over every rule.
    pub totals: CoverageTotals,
    /// In a projected report, each pending case the converter refuses, which would cover nothing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projected_refusals: Vec<String>,
}

/// Coverage of one rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RuleCoverage {
    /// The rule id.
    pub rule: String,
    /// Whether a domain is declared for it.
    pub declared: bool,
    /// Why its domain cannot be stated, when declared undeclarable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undeclarable: Option<String>,
    /// Accepted combinations.
    pub combinations: usize,
    /// Combinations without a passing recorded case.
    pub shape_gaps: Vec<String>,
    /// Shapes converted cases show the rule accepting outside its declared domain.
    pub undeclared_shapes: Vec<String>,
    /// Hazards its arguments and rows need.
    pub hazards: usize,
    /// Hazards without a passing recorded case.
    pub hazard_gaps: Vec<String>,
    /// Hazards no case can record, left out of `hazards`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrecordable_hazards: Vec<String>,
}

/// Totals over every rule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CoverageTotals {
    /// Rules in the catalogue of rules.
    pub rules: usize,
    /// Rules with no declared domain.
    pub undeclared_rules: usize,
    /// Rules declared undeclarable.
    pub undeclarable_rules: usize,
    /// Accepted combinations.
    pub combinations: usize,
    /// Combinations without a passing case.
    pub shape_gaps: usize,
    /// Shapes accepted outside a declared domain.
    pub undeclared_shapes: usize,
    /// Hazards needed.
    pub hazards: usize,
    /// Hazards without a passing case.
    pub hazard_gaps: usize,
    /// Hazards no case can record, counted in neither of the above.
    pub unrecordable_hazards: usize,
}

impl CoverageTotals {
    /// Whether anything is uncovered or undeclared.
    pub fn has_gaps(&self) -> bool {
        self.undeclared_rules
            + self.undeclarable_rules
            + self.shape_gaps
            + self.undeclared_shapes
            + self.hazard_gaps
            > 0
    }
}
