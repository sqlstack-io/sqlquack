//! `diff_results`: a baseline against a later one; drops out of matching and into differs regress.

use crate::results::_helpers::diff::{compare, form_regresses, statement_regresses};
use crate::results::models::{ResultsBaseline, ResultsDiff};

/// How `head` differs from `base`; any regression fails the gate, any change needs a new baseline.
pub fn diff_results(base: &ResultsBaseline, head: &ResultsBaseline) -> ResultsDiff {
    let statements = compare(
        "statement",
        (&base.statements, &head.statements),
        statement_regresses,
    );
    let forms = compare("form", (&base.forms, &head.forms), form_regresses);
    ResultsDiff {
        regressions: [statements.regressions, forms.regressions].concat(),
        changes: [statements.changes, forms.changes].concat(),
    }
}
