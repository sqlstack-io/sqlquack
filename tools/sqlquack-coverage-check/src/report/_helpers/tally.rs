//! Counting forms into tallies.

use crate::check::models::{FormResult, FormState};
use crate::report::models::Tally;

/// Percent with two decimals; zero of zero is zero.
pub(crate) fn percent(part: usize, whole: usize) -> f64 {
    match whole {
        0 => 0.0,
        _ => (part as f64 * 10_000.0 / whole as f64).round() / 100.0,
    }
}

/// `tally` with `form` counted in.
pub(crate) fn counted(tally: Tally, form: &FormResult) -> Tally {
    Tally {
        forms: tally.forms + 1,
        verified: tally.verified + usize::from(form.state == FormState::Verified),
        excluded: tally.excluded + usize::from(form.state == FormState::Excluded),
        required_obligations: tally.required_obligations + form.required_obligations,
        satisfied_obligations: tally.satisfied_obligations + form.satisfied_obligations,
    }
}
