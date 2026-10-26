//! Corpora and outcomes shared by the exact-check tests.

use std::path::{Path, PathBuf};

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_polyglot_check::expected::main::load_expected::load_expected;
use sqlquack_polyglot_check::runs::main::run_corpus::run_corpus;

use crate::test_types::ExactOutcome;

/// Findings kept in an outcome, so a failure shows the first few in full.
const SHOWN_FINDINGS: usize = 5;

/// The repository root.
pub(crate) fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The findings of comparing every entry, or a failed load.
pub(crate) fn exact_outcome(entries: &Result<Vec<CorpusEntry>, String>) -> ExactOutcome {
    let Ok(entries) = entries else {
        return ExactOutcome {
            loaded: false,
            differences: Vec::new(),
            tokens_lost: Vec::new(),
            type_names: Vec::new(),
            unlisted: Vec::new(),
            unexplained: Vec::new(),
        };
    };
    let expected =
        load_expected(&repository().join("corpora/polyglot-check/expected_differences.toml"))
            .expect("the expected differences load");
    let summary = run_corpus(entries, &expected);
    let shown = |findings: &[sqlquack_polyglot_check::runs::models::Finding]| -> Vec<String> {
        findings
            .iter()
            .take(SHOWN_FINDINGS)
            .map(|finding| format!("{}\n{}", finding.from, finding.detail))
            .collect()
    };
    ExactOutcome {
        loaded: true,
        differences: shown(&summary.differences),
        tokens_lost: shown(&summary.tokens_lost),
        type_names: shown(&summary.type_names),
        unlisted: shown(&summary.unlisted),
        unexplained: shown(&summary.unexplained),
    }
}
