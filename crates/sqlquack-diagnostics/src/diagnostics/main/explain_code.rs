//! `explain_code`: a registered code's title, explanation, help and example, as text.

use crate::diagnostics::_helpers::code_entry::code_entry;

/// The explanation of the code whose code, name or alias is `key`, ignoring case; `None` when none is.
pub fn explain_code(key: &str) -> Option<String> {
    let entry = code_entry(key)?;
    let alias = match entry.alias {
        Some(alias) => format!(" (formerly {alias})"),
        None => String::new(),
    };
    let help = match entry.help {
        Some(help) => format!("help: {help}\n\n"),
        None => String::new(),
    };
    Some(format!(
        "{code} {name}{alias}: {title}\n\n{explanation}\n\n{help}example:\n    {example}\n",
        code = entry.code,
        name = entry.name,
        title = entry.title,
        explanation = entry.explanation,
        example = entry.example,
    ))
}
