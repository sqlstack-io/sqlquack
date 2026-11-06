//! Recorded statements, their links to catalogue forms, and the result-matching measure (D139).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `sqlquack verify --translation-statements`: every recorded case's outcome and constructs.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementsFile {
    pub format: String,
    pub statements: Vec<RecordedStatement>,
}

/// One case's Snowflake statement and what verify made of it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedStatement {
    /// `<file>#<id>`, relative to the evidence cases directory.
    pub reference: String,
    pub hash: String,
    pub outcome: StatementOutcome,
    /// Diagnostic codes of its refusals, or `divergence:<id>` for a guard's raise.
    #[serde(default)]
    pub codes: Vec<String>,
    /// The refused construct keys.
    #[serde(default)]
    pub refused: Vec<String>,
    /// Every construct the statement uses, sorted.
    pub constructs: Vec<String>,
    /// Why SQLQuack's parser refused the statement; its constructs are then only token spellings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unparsed: Option<String>,
}

/// What one statement shows; only the first two are results matching Snowflake.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementOutcome {
    Matching,
    /// DuckDB raised the same kind of error Snowflake recorded.
    MatchingError,
    /// Only types were recorded, or only Snowflake ran: no DuckDB result was compared.
    ShapeOnly,
    /// DuckDB raised a declared divergence's guard where Snowflake returned rows.
    DeclaredDivergence,
    Refused,
    Unproven,
    UnrecordedTranslates,
    UnrecordedRefused,
    /// The replay differed from the recording: a bug.
    Differs,
}

impl StatementOutcome {
    /// Whether the statement translates with results matching Snowflake's recording.
    pub fn matches(self) -> bool {
        matches!(self, Self::Matching | Self::MatchingError)
    }

    /// Whether a Snowflake result is recorded for it.
    pub fn recorded(self) -> bool {
        !matches!(self, Self::UnrecordedTranslates | Self::UnrecordedRefused)
    }

    /// Whether it is refused, or raised where Snowflake did not, or cannot be judged.
    pub fn refused(self) -> bool {
        matches!(
            self,
            Self::Refused | Self::UnrecordedRefused | Self::DeclaredDivergence | Self::Unproven
        )
    }
}

/// `form-links.toml`: how statement constructs other than function calls link to forms.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinksFile {
    #[serde(default, rename = "link")]
    pub links: Vec<FormLink>,
    #[serde(default, rename = "no_form")]
    pub no_forms: Vec<NoForm>,
}

/// A statement using every one of `constructs` exercises each of `forms`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormLink {
    pub constructs: Vec<String>,
    pub forms: Vec<String>,
}

/// Constructs that are part of other forms' grammar and name no form of their own.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoForm {
    pub constructs: Vec<String>,
    pub reason: String,
}

/// `not-translatable.toml`: forms DuckDB has no meaningful equivalent for, each with a reason.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotTranslatableFile {
    #[serde(default, rename = "group")]
    pub groups: Vec<NotTranslatableGroup>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NotTranslatableGroup {
    /// A stable slug the auditor's sign-off names.
    pub id: String,
    pub reason: String,
    pub forms: Vec<String>,
}

/// `reviews/auditor-results.toml`: the auditor's sign-offs, each bound to a SHA-256.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsReviewFile {
    #[serde(default, rename = "signoff")]
    pub signoffs: Vec<ResultsSignoff>,
}

/// `subject` is `form-links`, `function-links` or `not-translatable:<id>`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsSignoff {
    pub subject: String,
    pub sha256: String,
    /// `full` or `sample`.
    pub scope: String,
    pub reviewed_utc: String,
}

/// The loaded link and list files, checked against the catalogue.
#[derive(Clone, Debug)]
pub struct ResultsFiles {
    pub links: LinksFile,
    pub not_translatable: NotTranslatableFile,
    pub review: ResultsReviewFile,
    /// SHA-256 of `form-links.toml`'s text.
    pub links_sha256: String,
}

/// A form's state over the recorded statements linked to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultState {
    /// Listed with a reason and signed off: outside the denominator.
    NotTranslatable,
    /// None of its statements is refused or differs, and at least one returns Snowflake's rows.
    Matching,
    /// None refused or differs, and none returns compared rows: only types or matching errors.
    SyntaxOnly,
    /// A statement's refusal or guard raise names it, or a statement linked to it cannot be judged.
    Refused,
    /// At least one linked statement's replay differs from Snowflake's result: a bug.
    Differs,
    /// No recorded statement links to it.
    Unmeasured,
}

/// One form's state and the statements behind it.
#[derive(Clone, Debug, Serialize)]
pub struct FormResult {
    pub id: String,
    pub family: String,
    pub state: ResultState,
    pub statements: usize,
    pub matching_statements: usize,
    /// The refusal codes of its statements, sorted and distinct.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
}

/// Form counts, for the whole catalogue or one family.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FormTally {
    pub catalogued: usize,
    pub not_translatable: usize,
    /// Catalogued less signed-off not-translatable: the form figure's denominator.
    pub translatable: usize,
    pub matching: usize,
    pub syntax_only: usize,
    pub refused: usize,
    pub differs: usize,
    pub unmeasured: usize,
}

/// Statement counts by outcome; `recorded` is the statement figure's denominator.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StatementTally {
    pub recorded: usize,
    /// Rows or errors matching Snowflake: the sum of the next two.
    pub matching: usize,
    /// DuckDB returned Snowflake's recorded rows.
    pub matching_rows: usize,
    /// Both sides raised the same kind of error.
    pub matching_errors: usize,
    pub by_outcome: BTreeMap<StatementOutcome, usize>,
    /// Recorded statements that link to no catalogue form.
    pub unlinked: usize,
}

/// One worklist entry: a gap to close, most urgent first.
#[derive(Clone, Debug, Serialize)]
pub struct WorkItem {
    /// `differs`, `refused` or `needs_receipt`.
    pub kind: String,
    /// The code and refused construct, or the statement reference.
    pub subject: String,
    /// Translatable forms this gap keeps from matching.
    pub forms: usize,
    pub statements: usize,
    /// Up to a few statement references.
    pub examples: Vec<String>,
}

/// The whole measure.
#[derive(Clone, Debug, Serialize)]
pub struct ResultsReport {
    pub forms: FormTally,
    pub families: BTreeMap<String, FormTally>,
    pub statements: StatementTally,
    /// Which link files and list entries carry a current sign-off.
    pub signed_off: Vec<String>,
    pub unsigned: Vec<String>,
    pub not_translatable: Vec<NotTranslatableGroup>,
    pub worklist: Vec<WorkItem>,
    /// Translatable query-result forms with no matching or refused statement: the capture queue.
    pub capture_forms: Vec<String>,
    /// Translatable DDL, DML, session and utility forms with none, needing an effect receipt.
    pub effect_receipt_forms: Vec<String>,
    /// Statements that convert but have no compared result, the queue's re-captures.
    pub capture_statements: Vec<String>,
    pub unlinked_statements: Vec<String>,
    /// SHA-256 of the function-name index, which a `function-links` sign-off names.
    pub function_links_sha256: String,
    pub form_results: Vec<FormResult>,
    pub statement_outcomes: BTreeMap<String, StatementOutcome>,
}

/// `results-baseline.json`: every statement's outcome and every form's state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResultsBaseline {
    pub format: String,
    pub statements: BTreeMap<String, StatementOutcome>,
    pub forms: BTreeMap<String, ResultState>,
}

/// How a report differs from the baseline.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ResultsDiff {
    /// Drops out of matching, or from refused to differs; these fail the gate.
    pub regressions: Vec<String>,
    /// Every other change; the baseline must be re-recorded to hold them.
    pub changes: Vec<String>,
}
