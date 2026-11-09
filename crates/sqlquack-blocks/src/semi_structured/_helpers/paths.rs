//! JSON paths: FLATTEN's PATH text, and the DuckDB paths the read check scopes itself to.

use crate::semi_structured::_helpers::guards::fidelity_error;
use crate::semi_structured::constants::{BRACKETED_KEY, PLAIN_KEY_PATTERN};
use crate::semi_structured::models::PathStep;

/// The path step of the decoded key `key`: `.key`, `['key']` with `'` escaped, or a raise.
pub(crate) fn key_step(key: &str) -> String {
    format!(
        "CASE WHEN regexp_full_match({key}, '{PLAIN_KEY_PATTERN}') THEN '.' || {key} \
         WHEN regexp_full_match({key}, '{BRACKETED_KEY}') \
         THEN '[''' || replace({key}, '''', '\\''') || ''']' \
         ELSE CAST({} AS VARCHAR) END",
        fidelity_error("a FLATTEN path through a key holding a character no recording shows")
    )
}

/// The path `steps` spell, each step `.key` or `[index]`, without a leading dot.
pub(crate) fn joined_path(steps: &str) -> String {
    format!("regexp_replace(array_to_string({steps}, ''), '^\\.', '')")
}

/// Whether a step writes as it is in a DuckDB path: a key without a quote or backslash.
pub(crate) fn plain_step(step: &PathStep) -> bool {
    !matches!(step, PathStep::Key(key) if key.contains(['"', '\\']))
}

/// The objects whose keys a read steps through, and the values it reads.
pub(crate) fn scoped_paths(paths: &[Vec<PathStep>]) -> (Vec<String>, Vec<String>) {
    let mut objects: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();
    for steps in paths {
        let mut path = String::from("$");
        for step in steps {
            match step {
                PathStep::Key(key) => {
                    objects.push(path.clone());
                    path.push_str(&format!(".\"{key}\""));
                }
                PathStep::Index(index) => path.push_str(&format!("[{index}]")),
            }
        }
        values.push(path);
    }
    objects.sort();
    objects.dedup();
    values.sort();
    values.dedup();
    (objects, values)
}

/// A VARCHAR list of DuckDB JSON paths.
pub(crate) fn path_list(paths: &[String]) -> String {
    match paths.is_empty() {
        true => "CAST([] AS VARCHAR[])".to_owned(),
        false => format!(
            "[{}]",
            paths
                .iter()
                .map(|path| format!("'{}'", path.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
