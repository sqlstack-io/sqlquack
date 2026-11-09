use std::path::Path;

use crate::check::models::{FormState, LocalVerdict, Reason};

/// A change to a tree where every form verifies, and what the checker must then say.
pub(crate) struct CheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) mutate: fn(&Path),
    /// The current parser.
    pub(crate) parse: fn(&str) -> LocalVerdict,
    /// The parser tree the check runs against; none for an uncommitted parser.
    pub(crate) parser_tree: Option<&'static str>,
    pub(crate) form: &'static str,
    pub(crate) expected_state: FormState,
    /// A reason the form must carry; none when it must carry none.
    pub(crate) expected_reason: Option<Reason>,
    /// A fragment of a reference problem; none when there must be no problem.
    pub(crate) expected_problem: Option<&'static str>,
}

/// Account parameter names with their values.
pub(crate) type Parameters<'a> = Vec<(&'a str, Option<&'a str>)>;

/// A receipt `account_state` fragment and the account state each case must then carry.
pub(crate) struct AccountStateTestCase {
    pub(crate) description: &'static str,
    /// Inserted after the first receipt's `phase_basis`, the receipt of `select-17`.
    pub(crate) account_state: &'static str,
    /// Each case id with its parameter names and values, in case-id order.
    pub(crate) expected_cases: Vec<(&'static str, Parameters<'static>)>,
}

/// Receipts on a case id no form cites, and the receipt verdicts the checker must then give.
pub(crate) struct ReceiptCheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) mutate: fn(&Path),
    pub(crate) parse: fn(&str) -> LocalVerdict,
    pub(crate) expected_contradictions: Vec<&'static str>,
    pub(crate) expected_unjudged: Vec<&'static str>,
}
