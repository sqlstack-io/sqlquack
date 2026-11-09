//! Data models of functions names.

/// Metadata describing parser-specific function behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionSpec {
    pub name: &'static str,
    pub no_paren: bool,
    pub aggregate: bool,
}
