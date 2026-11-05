use crate::results::models::{ResultState, StatementOutcome};

/// The outcomes of the statements calling COUNT_IF, and the state its form must take.
pub(crate) struct FormStateTestCase {
    pub(crate) description: &'static str,
    pub(crate) outcomes: &'static [StatementOutcome],
    /// Writes the not-translatable list for COUNT_IF's form, or nothing.
    pub(crate) list: fn(&str) -> String,
    /// Writes the auditor's sign-off of that list, or nothing.
    pub(crate) review: fn(&str) -> String,
    pub(crate) expected_state: ResultState,
    /// The form figure's denominator: catalogued forms less signed-off listed ones.
    pub(crate) expected_translatable: usize,
}

/// Translation files the measure must refuse, and a fragment of the problem it reports.
pub(crate) struct RefusalTestCase {
    pub(crate) description: &'static str,
    pub(crate) links: &'static str,
    pub(crate) constructs: &'static [&'static str],
    pub(crate) expected_problem: &'static str,
}

/// A statement matching a signed-off not-translatable form, and the problem reported.
pub(crate) struct ListedMatchTestCase {
    pub(crate) description: &'static str,
    pub(crate) outcome: StatementOutcome,
    pub(crate) expected_problem: &'static str,
}

/// The constructs of a refused IFF statement, and the states of the forms it links to.
pub(crate) struct LinkTestCase {
    pub(crate) description: &'static str,
    pub(crate) refused_constructs: &'static [&'static str],
    pub(crate) expected_iff: (ResultState, usize),
    pub(crate) expected_top: ResultState,
    /// Forms the worklist's first refusal keeps from matching.
    pub(crate) expected_blocked_forms: usize,
}

/// The constructs a statement's refusal names, and the state IFF's form must take.
pub(crate) struct RefusedConstructTestCase {
    pub(crate) description: &'static str,
    pub(crate) refused: &'static [&'static str],
    pub(crate) expected_iff: ResultState,
}

/// A statement's outcome in the baseline and now, and whether the change regresses.
pub(crate) struct DiffTestCase {
    pub(crate) description: &'static str,
    pub(crate) base: Option<StatementOutcome>,
    pub(crate) head: Option<StatementOutcome>,
    pub(crate) expected_regression: bool,
}
