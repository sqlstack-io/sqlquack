pub(crate) struct CallDepthTestCase {
    pub(crate) description: &'static str,
    /// The caller's limit, or `None` for `parse_source`'s default.
    pub(crate) limit: Option<usize>,
    pub(crate) levels: usize,
    /// How many statements parse, or how the parse error ends.
    pub(crate) expected_outcome: Result<usize, &'static str>,
}

pub(crate) struct CheckedDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) depth: usize,
    pub(crate) expected_result: Result<usize, &'static str>,
}

/// Reads one dialect's statements with their source ranges.
pub(crate) type LocatedParser = fn(&str) -> Option<Vec<sqlquack_syntax::Expression>>;

pub(crate) struct OperatorSpanTestCase {
    pub(crate) description: &'static str,
    pub(crate) parse: LocatedParser,
    pub(crate) sql: &'static str,
    /// The operators found at the recorded ranges, in source order.
    pub(crate) expected_operators: &'static [&'static str],
}

pub(crate) struct JoinSpanTestCase {
    pub(crate) description: &'static str,
    pub(crate) parse: LocatedParser,
    pub(crate) sql: &'static str,
    /// For each join: its first keyword, its JOIN keyword or comma, and its ON, empty when none.
    pub(crate) expected_joins: &'static [[&'static str; 3]],
    /// Where the commas between the FROM clause's items are.
    pub(crate) expected_commas: &'static [usize],
}

pub(crate) struct DistinctSpanTestCase {
    pub(crate) description: &'static str,
    pub(crate) parse: LocatedParser,
    pub(crate) sql: &'static str,
    /// Where each SELECT DISTINCT's keyword starts.
    pub(crate) expected_starts: &'static [usize],
}

pub(crate) struct WrittenCallTestCase {
    pub(crate) description: &'static str,
    pub(crate) parse: LocatedParser,
    pub(crate) sql: &'static str,
    /// Each call in source order: its name as written and its qualifier joined by dots.
    pub(crate) expected_calls: &'static [(&'static str, &'static str)],
}

pub(crate) struct RowLimitSpanTestCase {
    pub(crate) description: &'static str,
    pub(crate) parse: LocatedParser,
    pub(crate) sql: &'static str,
    /// Where each query's first LIMIT, OFFSET or FETCH keyword starts.
    pub(crate) expected_starts: &'static [usize],
}
