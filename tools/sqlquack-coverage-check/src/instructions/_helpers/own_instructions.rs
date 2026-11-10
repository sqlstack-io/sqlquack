//! The instructions a callgrind profile counts in one object's own code.

use std::collections::BTreeMap;
use std::path::Path;

/// An `ob=`/`cob=` value's id and name: `(3) /bin/a` defines 3, `(3)` reuses it, `/bin/a` has no id.
fn object_reference(value: &str) -> (Option<&str>, Option<&str>) {
    let Some(rest) = value.strip_prefix('(') else {
        return (None, Some(value));
    };
    let (id, name) = rest.split_once(')').unwrap_or((rest, ""));
    (Some(id), Some(name.trim()).filter(|name| !name.is_empty()))
}

/// The object an `ob=`/`cob=` value names, recording a new id's name in `names`.
fn object_name(value: &str, names: BTreeMap<String, String>) -> (String, BTreeMap<String, String>) {
    let mut names: BTreeMap<String, String> = names;
    let name: String = match object_reference(value) {
        (None, name) => name.unwrap_or_default().to_owned(),
        (Some(id), Some(name)) => {
            names.insert(id.to_owned(), name.to_owned());
            name.to_owned()
        }
        (Some(id), None) => names.get(id).cloned().unwrap_or_default(),
    };
    (name, names)
}

/// Self cost in `file_name`'s functions; a cost line after `calls=` is inclusive, so it is skipped.
pub(crate) fn own_instructions(profile: &str, file_name: &str) -> Result<u64, String> {
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut counted: bool = false;
    let mut after_call: bool = false;
    let mut total: u64 = 0;
    for line in profile.lines() {
        if let Some(value) = line.strip_prefix("ob=") {
            let name: String;
            (name, names) = object_name(value, names);
            counted = Path::new(&name)
                .file_name()
                .is_some_and(|name| name == file_name);
        } else if let Some(value) = line.strip_prefix("cob=") {
            (_, names) = object_name(value, names);
        } else if line.starts_with("calls=") {
            after_call = true;
        } else if line.starts_with(|first: char| first.is_ascii_digit() || "+-*".contains(first)) {
            if counted && !after_call {
                let cost: &str = line.split_whitespace().last().unwrap_or_default();
                total += cost
                    .parse::<u64>()
                    .map_err(|error| format!("callgrind cost line {line:?}: {error}"))?;
            }
            after_call = false;
        }
    }
    Ok(total)
}
