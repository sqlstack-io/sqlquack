//! Conversion inputs and results.

use crate::snowflake::conversion::constants::{
    DEFAULT_BUDGET_MILLISECONDS, DEFAULT_BUDGET_MINIMUM_BYTES, DEFAULT_BUDGET_SIZE_RATIO,
};
use std::collections::BTreeMap;

use serde::Serialize;
use sqlquack_diagnostics::failures::main::raise_id::raise_id;
use sqlquack_diagnostics::failures::models::FailureMode;
use sqlquack_diagnostics::provenance::models::{DivergenceUse, Provenance};
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;
use sqlquack_semantics::snowflake::typing::types::ColumnSpec;
use sqlquack_syntax::tokens::models::SourceLocation;

/// A known divergence the policy allows in the current mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AllowedDivergence {
    /// The construct keys it may apply to.
    pub constructs: Vec<String>,
    /// Why it is acceptable.
    pub reason: String,
    /// Whether it assumes a fact about the data or session that, if false, gives a wrong value.
    pub assumption: bool,
}

/// How conversion treats the policy's known divergences.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ConversionOptions {
    /// Divergence id to its allowance; any divergence not listed is refused.
    pub divergences: BTreeMap<String, AllowedDivergence>,
    /// A case's input fixture, whose conversion is plumbing rather than evidence for rules.
    pub fixture: Option<Fixture>,
    /// Limits beyond which converting one statement is refused rather than kept.
    pub budget: ConversionBudget,
    /// Whether the source quotes a name in an EXCLUDE list, a quoting the parsed tree does not keep.
    pub quoted_exclusions: bool,
    /// Whether quoted identifiers resolve to upper case, as `QUOTED_IDENTIFIERS_IGNORE_CASE = true`.
    pub quoted_identifiers_ignore_case: bool,
    /// The session's current database, resolved, which completes a relation name without one.
    pub current_database: Option<String>,
    /// The session's current schema, resolved, which completes a relation name without one.
    pub current_schema: Option<String>,
    /// User overrides by the name of the function each replaces, upper case.
    pub overrides: BTreeMap<String, UserOverride>,
    /// Override names to the SHA-256 of the body recorded cases verified, which strict mode allows.
    pub verified_overrides: BTreeMap<String, String>,
}

/// A user's DuckDB SQL standing in for a Snowflake function, written in SQLBuild's FUNCTION() format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UserOverride {
    /// The override's own name, upper case.
    pub name: String,
    /// The parameters, in order, with the names its body reads.
    pub arguments: Vec<ColumnSpec>,
    /// The result's Snowflake type.
    pub returns: SnowflakeType,
    /// The DuckDB expression over the parameters, as loaded.
    pub body: String,
    /// The SHA-256 of `body`, in lower-case hex, which verification is bound to.
    pub sha256: String,
}

/// How long converting one statement may take, and how large its DuckDB SQL may grow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ConversionBudget {
    /// Wall time for converting one statement, in milliseconds.
    pub milliseconds: u64,
    /// The emitted SQL may be this many times the source text, beyond `minimum_bytes`.
    pub size_ratio: u64,
    /// Emitted SQL up to this size is always within budget.
    pub minimum_bytes: u64,
}

impl Default for ConversionBudget {
    fn default() -> Self {
        Self {
            milliseconds: DEFAULT_BUDGET_MILLISECONDS,
            size_ratio: DEFAULT_BUDGET_SIZE_RATIO,
            minimum_bytes: DEFAULT_BUDGET_MINIMUM_BYTES,
        }
    }
}

/// The outermost CTE holding a case's input rows, and the column ordering them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fixture {
    /// The CTE name.
    pub relation: String,
    /// The column a trailing ORDER BY uses to keep rows in input order.
    pub ordering_column: String,
}

/// A converted statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Converted {
    /// The DuckDB SQL.
    pub sql: String,
    /// The result columns and their Snowflake types.
    pub columns: Vec<ColumnSpec>,
    /// The rules, building blocks and typing functions used.
    pub provenance: Provenance,
    /// Where each raise ID in the SQL comes from: every source location of its expression.
    pub raises: BTreeMap<String, Vec<SourceLocation>>,
    /// Each reliance on a known divergence, placed in the source where the expression is known.
    #[serde(skip)]
    pub divergence_uses: Vec<DivergenceUse>,
}

impl Converted {
    /// The source locations of the expression whose raise produced the error `message`.
    pub fn raise_locations(&self, message: &str) -> &[SourceLocation] {
        raise_id(message)
            .and_then(|id| self.raises.get(id))
            .map_or(&[], Vec::as_slice)
    }
}

/// A converted SQL function: the DuckDB macro's parameters, in order, and its body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConvertedFunction {
    /// The macro's parameter names, one per declared parameter.
    pub parameters: Vec<String>,
    /// The body after `AS`: a scalar subquery, or `TABLE` and a query.
    pub body: String,
    /// The rules, building blocks and typing functions used.
    pub provenance: Provenance,
    /// Where each raise ID in the body comes from: every source location of its expression.
    pub raises: BTreeMap<String, Vec<SourceLocation>>,
}

/// A rule: what it converts and the conditions outside which it refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RuleSpec {
    /// The stable identifier, such as `rule.comparison`.
    pub id: &'static str,
    /// The construct keys the rule converts.
    pub constructs: &'static [&'static str],
    /// The type conditions it requires.
    pub conditions: &'static str,
    /// The ways its converted SQL may fail, each with the Snowflake errors meaning the same failure.
    pub failures: &'static [FailureMode],
}

/// How a kept call's arguments map onto the payload its rule reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallShape {
    /// One or more arguments, as `expressions`.
    Expressions,
    /// One or more arguments, as `expressions`, keeping the written name.
    Renamed,
    /// `(this, expression)`, keeping the written name.
    Fallback,
    /// `(condition, true_value, false_value)`.
    Conditional,
    /// `(this)`.
    Unary,
    /// `(this [, characters])`, trimming both ends.
    Trim,
    /// `(this, expression)`.
    Affix,
    /// `(this, start [, length])`.
    Substring,
    /// `(unit, start, end)`.
    Difference,
    /// `(*)` or `([DISTINCT] this)`.
    Count,
    /// `([DISTINCT] this)`.
    Aggregate,
    /// `()`, a ranking window function.
    Ranking,
    /// `(this [, offset [, default]])`, an offset window function.
    Offset,
    /// `(this)`, optionally `IGNORE NULLS`.
    FirstLast,
}

/// A call spelling with recorded evidence, and the rule node it is converted as.
#[derive(Clone, Copy, Debug)]
pub struct SpellingRule {
    /// The upper-case spelling.
    pub name: &'static str,
    /// The node kind of the rule that converts it.
    pub node: &'static str,
    /// How its arguments are read.
    pub shape: CallShape,
    /// The recorded spelling pair that proves it means what the rule's own spelling means.
    pub pair: Option<&'static str>,
}

/// Two spellings Snowflake evaluates identically, as recorded pairs of results show.
#[derive(Clone, Copy, Debug)]
pub struct SpellingPair {
    /// The stable identifier the case file declares as its `equivalence`.
    pub id: &'static str,
    /// The first spelling, as the cases write it.
    pub source: &'static str,
    /// The second spelling, recorded as each case's `canonical`.
    pub canonical: &'static str,
    /// The case file, relative to the cases directory, holding the recorded pairs.
    pub cases: &'static str,
}

/// Every refusal found in a statement by converting past each refused node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CollectedRefusals {
    /// The refusals, in the order found.
    pub refusals: Vec<Refusal>,
    /// Whether a refusal may be missing, or have been dropped as a possible knock-on.
    pub possibly_incomplete: bool,
    /// The statement's result columns where conversion reached them; refused values are NULL-typed.
    pub columns: Option<Vec<ColumnSpec>>,
}
