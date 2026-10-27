use serde_json::Value;

use crate::expressions::types::models::DataType;
use crate::guard::models::ParseOptions;

use crate::parser::tests::helpers::FragmentParser;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;

use crate::tokens::models::DialectType;

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
