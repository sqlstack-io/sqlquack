//! Constructs and their occurrences.

use std::fmt::{self, Display};

use serde::{Serialize, Serializer};

use sqlquack_syntax::tokens::models::SourceLocation;

/// The kind of thing a construct names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructFamily {
    /// An AST node kind, such as `select`, `eq` or `coalesce`.
    Node,
    /// A function the parser kept as a generic call, by upper-case name.
    Function,
    /// The base target type of a cast.
    Cast,
    /// A join kind.
    Join,
    /// A query clause, such as `qualify` or `group_by`.
    Clause,
    /// A spelling with its own meaning, such as `::` casts.
    Syntax,
}

impl ConstructFamily {
    /// The prefix used in construct keys.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Function => "fn",
            Self::Cast => "cast",
            Self::Join => "join",
            Self::Clause => "clause",
            Self::Syntax => "syntax",
        }
    }
}

/// One source construct that needs a conversion rule, keyed as `<family>:<name>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Construct {
    /// What the construct names.
    pub family: ConstructFamily,
    /// The node kind, function name, type, join kind, clause or syntax.
    pub name: String,
}

impl Construct {
    /// A construct of `family` called `name`.
    pub fn new(family: ConstructFamily, name: impl Into<String>) -> Self {
        Self {
            family,
            name: name.into(),
        }
    }

    /// The `<family>:<name>` key used in reports.
    pub fn key(&self) -> String {
        self.to_string()
    }
}

impl Display for Construct {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.family.prefix(), self.name)
    }
}

impl Serialize for Construct {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// One use of a construct in a statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Occurrence {
    /// The construct used.
    pub construct: Construct,
    /// Where it is, when the parser kept a position for the node or one of its neighbours.
    pub location: Option<SourceLocation>,
}
