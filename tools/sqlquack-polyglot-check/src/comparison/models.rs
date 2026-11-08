//! What a comparison found.

use std::time::Duration;

/// How SQLQuack's parse compares with polyglot's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Both parsed it into trees that serialise identically.
    SameTree,
    /// Both refused it with identical errors.
    SameError,
    /// The results differ; each side rendered as text.
    Differs {
        /// polyglot's result.
        polyglot: String,
        /// SQLQuack's result.
        sqlquack: String,
    },
    /// The results agree, but SQLQuack's lossless tokens do not reproduce the input.
    TokensLost {
        /// Why.
        reason: String,
    },
}

/// The outcome and how long each parser took.
#[derive(Clone, Debug)]
pub struct Comparison {
    /// How the results compare.
    pub outcome: Outcome,
    /// polyglot's parse time.
    pub polyglot: Duration,
    /// SQLQuack's parse time.
    pub sqlquack: Duration,
}

/// A call spelling SQLQuack keeps where polyglot lowers it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spelling {
    /// The call's upper-case name.
    pub name: String,
    /// The node polyglot lowers it to, or `error` when polyglot rejects it.
    pub polyglot: String,
}

impl Spelling {
    /// Whether polyglot rejects the call rather than lowering it.
    pub fn is_rejection(&self) -> bool {
        self.polyglot == crate::comparison::constants::ERROR_KIND
    }
}

/// What the faithful parse keeps, and what nothing explains.
#[derive(Clone, Debug)]
pub struct FaithfulComparison {
    /// The kept spellings, one per call.
    pub spellings: Vec<Spelling>,
    /// A difference from the lowered parse the kept spellings do not explain.
    pub unexplained: Option<String>,
}
