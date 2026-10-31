//! `parse_session`: a caller's `NAME=VALUE` session settings.

use std::collections::BTreeMap;

/// `NAME=VALUE` settings as upper-case names to values; empty or conflicting names are an error.
pub(crate) fn parse_session(settings: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut parameters: BTreeMap<String, String> = BTreeMap::new();
    for setting in settings {
        let (name, value) = setting
            .split_once('=')
            .ok_or_else(|| format!("--session needs NAME=VALUE, not `{setting}`"))?;
        let name = name.trim().to_ascii_uppercase();
        let value = value.trim().to_owned();
        if name.is_empty() {
            return Err(format!("--session `{setting}` names no parameter"));
        }
        if let Some(earlier) = parameters.get(&name)
            && *earlier != value
        {
            return Err(format!(
                "--session gives {name} twice, as `{earlier}` and as `{value}`"
            ));
        }
        parameters.insert(name, value);
    }
    Ok(parameters)
}
