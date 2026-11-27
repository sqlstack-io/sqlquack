//! The Databricks SQL evidence profile.

use sqlquack_compare::comparison::models::RecordedResult;
use sqlquack_compare::values::models::ResultSet;

use crate::cases::models::{CaseDefinition, Spelling};
use crate::databricks::profile::_helpers::cells::text_result;
use crate::databricks::profile::_helpers::failures::databricks_kind;
use crate::databricks::profile::_helpers::source_sql::values_source_sql;
use crate::dialects::types::EvidenceProfile;
use crate::errors::EvidenceError;
use crate::results::models::{RecordedFailure, ResultMetadata};
use sqlquack_diagnostics::failures::models::ErrorKind;

/// Input rows as one `VALUES` list of `CAST` literals; results stored as the text Databricks returned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DatabricksEvidence;

impl EvidenceProfile for DatabricksEvidence {
    fn source_sql(&self, case: &CaseDefinition, spelling: Spelling) -> Option<String> {
        Some(values_source_sql(case, spelling))
    }

    fn records_rows_alone(&self) -> bool {
        false
    }

    fn recorded_result(
        &self,
        metadata: &ResultMetadata,
        rows: &ResultSet,
    ) -> Option<Result<RecordedResult, EvidenceError>> {
        Some(text_result(metadata, rows))
    }

    fn failure_kind(&self, failure: &RecordedFailure) -> Option<Option<ErrorKind>> {
        Some(databricks_kind(failure))
    }
}
