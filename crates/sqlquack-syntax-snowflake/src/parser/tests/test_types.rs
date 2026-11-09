use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::errors::Error;
use sqlquack_syntax::traversal::models::ChildPathSegment;

use serde_json::Value;

use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::guard::models::ParseOptions;

use crate::parser::tests::helpers::FragmentParser;
use sqlquack_syntax::tokens::models::Token;
use sqlquack_syntax::tokens::models::TokenType;

use sqlquack_syntax::tokens::models::DialectType;

pub(crate) struct ParseKindTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: Option<DialectType>,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_kind: Result<&'static str, String>,
}

pub(crate) struct ParseValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: Option<DialectType>,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_value: Result<Value, String>,
}

pub(crate) struct ParseErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: Option<DialectType>,
    pub(crate) sql: &'static str,
    pub(crate) expected_error: &'static str,
}

pub(crate) struct ChainDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) connector: &'static str,
    pub(crate) terms: usize,
    pub(crate) expected_depth_below: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    Parsed,
    Rejected { by_depth_guard: bool },
    Panicked,
    OverBudget,
}

pub(crate) struct TerminationTestCase {
    pub(crate) description: &'static str,
    pub(crate) inputs: Vec<String>,
    pub(crate) expected_decisions: &'static [Decision],
}

pub(crate) struct DepthScopeTestCase {
    pub(crate) description: &'static str,
    pub(crate) max_parser_depth: usize,
    pub(crate) expected_trace: (usize, bool, usize, bool, bool, bool),
}

pub(crate) struct EmptyStreamTestCase {
    pub(crate) description: &'static str,
    pub(crate) parts: &'static [&'static str],
    pub(crate) expected_nonempty_fragments: Vec<String>,
}

pub(crate) struct FragmentEofTestCase {
    pub(crate) description: &'static str,
    pub(crate) fragment: FragmentParser,
    pub(crate) sql: &'static str,
    pub(crate) expected_changed_constructors: Vec<&'static str>,
}

pub(crate) struct TerminatorTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: (bool, bool),
}

pub(crate) struct TerminatedValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) terminated: bool,
    pub(crate) pointer: &'static str,
    pub(crate) expected_value: Value,
}

pub(crate) struct EntryPointTestCase {
    pub(crate) description: &'static str,
    pub(crate) parts: &'static [&'static str],
    pub(crate) entry: fn(Vec<Token>) -> String,
    pub(crate) expected_outcome_part: &'static str,
}

pub(crate) struct SourceEntryTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: &'static str,
}

pub(crate) struct TrailingKeywordTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_same: bool,
}

pub(crate) struct ParsePresenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: Option<DialectType>,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_present: Result<bool, String>,
}

pub(crate) struct DialectTokenTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: DialectType,
    pub(crate) sql: &'static str,
    pub(crate) token_type: TokenType,
    pub(crate) expected_text: &'static str,
}

pub(crate) struct DataTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: DialectType,
    pub(crate) sql: &'static str,
    pub(crate) options: ParseOptions,
    pub(crate) expected_data_type: DataType,
}

pub(crate) struct DataTypeErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: DialectType,
    pub(crate) sql: &'static str,
    pub(crate) options: ParseOptions,
    pub(crate) expected_error: &'static str,
}

pub(crate) struct StreamDataTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) parts: &'static [&'static str],
    pub(crate) expected_data_type: Result<DataType, String>,
}

pub(crate) struct GuardedParseTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) max_parser_depth: Option<usize>,
    pub(crate) outcome: fn(&str, Option<usize>) -> String,
    pub(crate) expected_outcome_part: &'static str,
}

pub(crate) struct AstDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: fn() -> String,
    /// None when the default guard accepts the query; otherwise a fragment of the error.
    pub(crate) expected_error: Option<&'static str>,
}

pub(crate) struct TypeDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: sqlquack_syntax::Dialect,
    /// The query with its type nested `n` levels deep.
    pub(crate) sql: fn(n: usize) -> String,
    /// The guard code at each of [`TYPE_DEPTHS`] levels, or None where the parse succeeds.
    pub(crate) expected_errors: [Option<&'static str>; 4],
}

pub(crate) struct TreeOperationTestCase {
    pub(crate) description: &'static str,
    /// The operation on the parsed tree; true when it ran to the end.
    pub(crate) operation: fn(Vec<sqlquack_syntax::expressions::tree::models::Expression>) -> bool,
    pub(crate) expected_completed: bool,
}

pub(crate) struct ChainStepTestCase {
    pub(crate) description: &'static str,
    /// Wraps the chain so far in one more node of the kind under test.
    pub(crate) step: fn(Expression) -> Expression,
    /// The chain's depth after 100,000 steps, which the drop must survive.
    pub(crate) expected_depth: usize,
}

pub(crate) struct OrderTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: Expression,
    pub(crate) expected_labels: &'static [&'static str],
}

pub(crate) struct SearchTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: Expression,
    pub(crate) kind: &'static str,
    pub(crate) expected_first: Option<&'static str>,
    pub(crate) expected_count: usize,
    pub(crate) expected_contains: bool,
}

pub(crate) struct DepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: Expression,
    pub(crate) expected_depth: usize,
}

pub(crate) struct ChildPathTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_paths: Vec<Vec<ChildPathSegment>>,
}

pub(crate) struct ErrorPositionTestCase {
    pub(crate) description: &'static str,
    pub(crate) error: Error,
    pub(crate) expected_position: (Option<usize>, Option<usize>, Option<usize>, Option<usize>),
    pub(crate) expected_message_parts: &'static [&'static str],
}

/// The nesting depths each [`TypeDepthTestCase`] is parsed at.
pub(crate) const TYPE_DEPTHS: [usize; 4] = [512, 513, 20_000, 100_000];
