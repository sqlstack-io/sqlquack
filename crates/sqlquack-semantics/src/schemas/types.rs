//! The questions shared semantics asks of a dialect's column types.

/// A dialect's column type, which keeps the dialect's exact types and answers shared questions.
pub trait DialectType {
    /// Whether the type holds semi-structured values, such as a VARIANT or JSON.
    fn is_semi_structured(&self) -> bool;
}
