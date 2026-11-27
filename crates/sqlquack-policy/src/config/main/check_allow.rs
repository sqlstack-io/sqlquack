//! `check_allow`: known differences a caller allows or refuses, checked against the evidence policy.

use std::collections::BTreeMap;

use crate::policy::constants::PROJECT_UDF;
use crate::policy::main::nearest_name::nearest_name;
use crate::policy::models::Policy;

/// Refuse names in `allow`, written in `table`, that `evidence` doesn't know, and `project_udf`, which `runtime_fix` allows.
pub fn check_allow(
    allow: &BTreeMap<String, bool>,
    evidence: &Policy,
    (table, runtime_fix): (&str, &str),
) -> Result<(), String> {
    let runtime = allow
        .contains_key(PROJECT_UDF)
        .then(|| format!("{table} cannot allow `{PROJECT_UDF}`: {runtime_fix}"));
    let problems: Vec<String> = allow
        .keys()
        .filter(|name| !evidence.divergences.contains_key(*name))
        .map(|name| {
            let known = evidence.divergences.keys().map(String::as_str);
            match nearest_name(name, known) {
                Some(near) => {
                    format!("{table} names no known difference `{name}`; did you mean `{near}`?")
                }
                None => format!(
                    "{table} names no known difference `{name}`; `sqlquack explain \
                     --differences` lists them"
                ),
            }
        })
        .chain(runtime)
        .collect();
    match problems.is_empty() {
        true => Ok(()),
        false => Err(problems.join("; ")),
    }
}
