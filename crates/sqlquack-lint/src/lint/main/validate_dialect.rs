//! Check a dialect name against the dialects SQL lint accepts.

/// `Ok` when `name` names a known SQL dialect, such as `duckdb` or `snowflake`, parsed or not.
pub fn validate_dialect(name: &str) -> Result<(), String> {
    match crate::dialects::main::lint_dialect::lint_dialect(name) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("unknown SQL dialect '{name}': {error}")),
    }
}
