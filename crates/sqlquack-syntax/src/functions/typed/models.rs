//! Data models of functions typed.

/// High-level parse category for canonical typed function dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypedParseKind {
    AggregateLike,
    Unary,
    Binary,
    Conditional,
    CastLike,
    Variadic,
}

/// Typed-function metadata for canonicalization and future dispatch organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypedFunctionSpec {
    pub canonical_name: &'static str,
    /// Alias spellings that canonicalize to `canonical_name`.
    pub aliases: &'static [&'static str],
    pub parse_kind: TypedParseKind,
}

/// Registry metadata group for specialized typed-function parser families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypedDispatchGroup {
    Aggregate,
    Window,
    Json,
    TranslateTeradata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypedDispatchGroupSpec {
    pub name: &'static str,
    pub group: TypedDispatchGroup,
}
