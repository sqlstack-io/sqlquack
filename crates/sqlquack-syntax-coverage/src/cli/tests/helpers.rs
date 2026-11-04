use crate::check::models::{LocalFailure, LocalVerdict};

/// The verdict of a worker that failed with `failure`.
pub(crate) fn failed(failure: LocalFailure) -> LocalVerdict {
    LocalVerdict::Failed { failure }
}
