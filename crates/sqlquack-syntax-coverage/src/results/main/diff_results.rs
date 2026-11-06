//! `diff_results`: a baseline against a later one; drops out of matching or into differs regress.

use crate::results::_helpers::diff::{
    compare, compare_obligations, form_regresses, statement_regresses,
};
use crate::results::models::{ResultsBaseline, ResultsDiff};

/// How `head` differs from `base`; any regression fails the gate, any change needs a new baseline.
pub fn diff_results(base: &ResultsBaseline, head: &ResultsBaseline) -> ResultsDiff {
    let statements = compare(
        "statement",
        (&base.statements, &head.statements),
        statement_regresses,
    );
    let forms = compare("form", (&base.forms, &head.forms), form_regresses);
    let obligations = compare_obligations((&base.obligations, &head.obligations), &head.retired);
    ResultsDiff {
        regressions: [
            statements.regressions,
            forms.regressions,
            obligations.regressions,
        ]
        .concat(),
        changes: [statements.changes, forms.changes, obligations.changes].concat(),
    }
}
