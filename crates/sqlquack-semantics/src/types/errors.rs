//! Schema failures.

use std::fmt::{self, Display};

/// Why a schema's text is not a schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    /// The text is not JSON of the schema's shape.
    Shape(String),
    /// A declaration the shape allows but SQLQuack does not, such as an unsupported offset.
    Declaration(String),
}

impl Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape(error) => write!(formatter, "not a schema: {error}"),
            Self::Declaration(reason) => formatter.write_str(reason),
        }
    }
}
