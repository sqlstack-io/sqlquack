use std::process::ExitCode;

use crate::results::models::{ObligationState, ResultState, StatementOutcome};

/// The outcomes of the statements calling COUNT_IF, and the state its form must take.
pub(crate) struct FormStateTestCase {
    pub(crate) description: &'static str,
    pub(crate) outcomes: &'static [StatementOutcome],
    /// Writes the not-translatable list for COUNT_IF's form, or nothing.
    pub(crate) list: fn(&str) -> String,
    /// Writes a reviewer's sign-off of that list, or nothing.
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

/// The keys of one statement calling SUM, REGR_SYY, EXTRACT or CURRENT_DATE, and the forms it credits.
pub(crate) struct CallShapeTestCase {
    pub(crate) description: &'static str,
    pub(crate) constructs: &'static [&'static str],
    pub(crate) expected_forms: &'static [&'static str],
}

/// An obligation file the loader must refuse, and a fragment of the problem it reports.
pub(crate) struct ObligationLoadTestCase {
    pub(crate) description: &'static str,
    pub(crate) obligations: String,
    /// The retired-obligations file, or nothing.
    pub(crate) retired: &'static str,
    pub(crate) expected_problem: &'static str,
}

/// COUNT_IF's obligations, its cited case's outcome beside a matching statement, and the states.
pub(crate) struct ObligationStateTestCase {
    pub(crate) description: &'static str,
    pub(crate) cited_outcome: StatementOutcome,
    pub(crate) obligations: String,
    pub(crate) expected_form: ResultState,
    /// Each obligation's state, in id order.
    pub(crate) expected_states: Vec<ObligationState>,
}

/// The results command over an obligation file, and its exit and JSON buckets.
pub(crate) struct ObligationsCliTestCase {
    pub(crate) description: &'static str,
    /// The functions obligation file, or no obligations directory at all.
    pub(crate) obligations: Option<String>,
    pub(crate) expected_exit: ExitCode,
    /// Met, held and unmeasured obligations in the report, and obligations in the baseline.
    pub(crate) expected_buckets: Option<(u64, u64, u64, u64)>,
}

/// One obligation's state before and after, and how many regressions the diff reports.
pub(crate) struct ObligationDiffTestCase {
    pub(crate) description: &'static str,
    pub(crate) before: ObligationState,
    /// None when the head drops the obligation.
    pub(crate) after: Option<ObligationState>,
    /// Whether the head's retired-obligations.toml lists it.
    pub(crate) retired: bool,
    pub(crate) expected_regressions: usize,
}

/// Where `syntax-coverage results` reads its cases, beside stale local copies of both kinds.
pub(crate) struct ResultsRootsTestCase {
    pub(crate) description: &'static str,
    /// The recorded syntax cases' root, under the test directory.
    pub(crate) evidence: &'static str,
    /// The recorded translation cases' root, under the test directory.
    pub(crate) cases: &'static str,
    pub(crate) expected_exit: ExitCode,
}

/// The arguments after `syntax-coverage results`, and whether they parse.
pub(crate) struct ResultsArgumentsTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_parses: bool,
}
