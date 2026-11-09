//! Inputs the building blocks share.

/// What a text conversion does with text it cannot read, and which Snowflake error it raises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFailure {
    /// A TRY_ form: NULL.
    Null,
    /// A cast or conversion of text: the text conversion's own Snowflake error.
    Text,
    /// A cast of a VARIANT string: Snowflake's failed-VARIANT-cast error.
    Variant,
}
