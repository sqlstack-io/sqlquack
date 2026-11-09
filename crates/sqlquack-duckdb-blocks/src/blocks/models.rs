//! Inputs the building blocks share.

use sqlquack_snowflake_types::typing::models::SnowflakeType;

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

/// What text read as a NUMBER is read as, and what its failures yield.
#[derive(Clone, Debug)]
pub enum NumberText {
    /// A NUMBER type, failing as the text failure says.
    Typed(SnowflakeType, TextFailure),
    /// A target its caller spells, such as a macro's typed NULL parameter.
    Spelled(NumberTarget),
}

/// How a reading spells its target type and failures, as SQL.
#[derive(Clone, Debug)]
pub struct NumberTarget {
    /// The SQL before and after a value that casts it to the target.
    pub cast: (String, String),
    /// The SQL before and after a value that casts it to the target or NULL.
    pub try_cast: (String, String),
    /// The largest value the target holds.
    pub largest: String,
    /// What text beyond the target's range yields.
    pub overflow: String,
    /// What text Snowflake does not read as a number yields.
    pub unrecognised: String,
}
