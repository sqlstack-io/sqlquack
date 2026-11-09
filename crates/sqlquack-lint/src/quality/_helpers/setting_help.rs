//! Remediation text that shows the exact configuration setting to write.

use crate::constants::SETTING_SNIPPET_INDENT;

/// One setting a remediation tells the user to write.
pub(crate) struct Setting<'a> {
    pub(crate) purpose: &'a str,
    pub(crate) config_file: &'a str,
    pub(crate) section: &'a str,
    pub(crate) key: &'a str,
    pub(crate) value: &'a str,
}

/// `<purpose>, set this in <config_file>:` followed by the section and `key = value`.
pub(crate) fn setting_help(setting: &Setting<'_>) -> String {
    let Setting {
        purpose,
        config_file,
        section,
        key,
        value,
    } = setting;
    format!(
        "{purpose}, set this in {config_file}:\n{SETTING_SNIPPET_INDENT}[{section}]\n{SETTING_SNIPPET_INDENT}{key} = {value}"
    )
}
