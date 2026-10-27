//! Corpora and outcomes shared by the exact-check tests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_polyglot_check::expected::main::load_expected::load_expected;
use sqlquack_polyglot_check::runs::main::run_corpus::run_corpus;
use sqlquack_polyglot_check::runs::models::RunSummary;

use crate::test_types::ExactOutcome;

/// Findings kept in an outcome, so a failure shows the first few in full.
const SHOWN_FINDINGS: usize = 5;

/// The repository root.
pub(crate) fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The summary of comparing `entries` under the committed expected differences.
pub(crate) fn summary_of(entries: &[CorpusEntry]) -> RunSummary {
    let expected =
        load_expected(&repository().join("corpora/polyglot-check/expected_differences.toml"))
            .expect("the expected differences load");
    run_corpus(entries, &expected)
}

/// The findings of comparing every entry, or a failed load.
pub(crate) fn exact_outcome(entries: &Result<Vec<CorpusEntry>, String>) -> ExactOutcome {
    let Ok(entries) = entries else {
        return ExactOutcome {
            loaded: false,
            rejected: BTreeMap::new(),
            differences: Vec::new(),
            tokens_lost: Vec::new(),
            type_names: Vec::new(),
            unlisted: Vec::new(),
            unexplained: Vec::new(),
        };
    };
    let summary = summary_of(entries);
    let shown = |findings: &[sqlquack_polyglot_check::runs::models::Finding]| -> Vec<String> {
        findings
            .iter()
            .take(SHOWN_FINDINGS)
            .map(|finding| format!("{}\n{}", finding.from, finding.detail))
            .collect()
    };
    ExactOutcome {
        loaded: true,
        rejected: summary.rejected.clone(),
        differences: shown(&summary.differences),
        tokens_lost: shown(&summary.tokens_lost),
        type_names: shown(&summary.type_names),
        unlisted: shown(&summary.unlisted),
        unexplained: shown(&summary.unexplained),
    }
}
