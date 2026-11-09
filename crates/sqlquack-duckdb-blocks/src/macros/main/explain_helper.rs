//! `explain_helper`: what `sqlquack explain` prints for a helper macro.

use crate::macros::_helpers::registry::helper_macros;
use crate::macros::constants::HELPER_CALL_PREFIX;
use crate::macros::main::helper_statements::helper_statements;

/// The call, meaning and definitions of the helper macro `key`, with or without `sqlquack.`.
pub fn explain_helper(key: &str) -> Option<String> {
    let name = key.strip_prefix(HELPER_CALL_PREFIX).unwrap_or(key);
    let helper = helper_macros()
        .into_iter()
        .find(|helper| helper.name == name)?;
    let call = format!(
        "{HELPER_CALL_PREFIX}{}({})",
        helper.name,
        helper.parameters.join(", ")
    );
    let definition = helper_statements(&[&call])
        .iter()
        .map(|statement| format!("    {statement};"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "{call}: helper macro\n\n{explain}\n\ndefinition:\n{definition}\n",
        explain = helper.explain,
    ))
}
