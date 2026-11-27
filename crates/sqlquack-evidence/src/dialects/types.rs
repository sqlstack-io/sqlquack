//! What an evidence profile answers.

use sqlquack_compare::comparison::models::RecordedResult;
use sqlquack_compare::values::models::ResultSet;

use crate::cases::models::{CaseDefinition, Spelling};
use crate::errors::EvidenceError;
use crate::results::models::{RecordedFailure, ResultMetadata};
use sqlquack_diagnostics::failures::models::ErrorKind;

/// How one warehouse writes its cases and stores its rows where they differ from the shared form.
pub trait EvidenceProfile: std::fmt::Debug + Sync {
    /// The SQL a case runs on the warehouse when written its own way; `None` for the shared form.
    fn source_sql(&self, _case: &CaseDefinition, _spelling: Spelling) -> Option<String> {
        None
    }

    /// Whether an input row may be recorded alone, as the shared form's row sources are.
    fn records_rows_alone(&self) -> bool {
        true
    }

    /// The recorded rows `metadata` describes when stored its own way; `None` for typed Arrow.
    fn recorded_result(
        &self,
        _metadata: &ResultMetadata,
        _rows: &ResultSet,
    ) -> Option<Result<RecordedResult, EvidenceError>> {
        None
    }

    /// The kind of a recorded error the warehouse names its own way; `None` to read Snowflake's codes.
    fn failure_kind(&self, _failure: &RecordedFailure) -> Option<Option<ErrorKind>> {
        None
    }
}
