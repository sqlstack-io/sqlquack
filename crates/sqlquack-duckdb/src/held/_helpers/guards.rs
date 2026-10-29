//! Binding a fragment to a name, and json_fidelity raises.

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::JSON_FIDELITY;

/// `body` with `value` bound to `name`, evaluated once.
pub(crate) fn bound(value: &str, name: &str, body: &str) -> String {
    format!("list_transform([{value}], lambda {name}: {body})[1]")
}

/// A json_fidelity raise for `what`.
pub(crate) fn fidelity_error(what: &str) -> String {
    raised(&format!("{JSON_FIDELITY}: {what}"))
}
