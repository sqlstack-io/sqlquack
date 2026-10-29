use sqlquack_rules::conversion::models::ConversionOptions;

pub(crate) struct RewriteTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_sql: &'static str,
}

pub(crate) struct ConvertibleTestCase {
    pub(crate) description: &'static str,
    pub(crate) files: Vec<Option<Vec<usize>>>,
    pub(crate) order: Vec<usize>,
    pub(crate) points: Vec<usize>,
    pub(crate) expected_convertible: Vec<usize>,
}

pub(crate) struct GreedyTestCase {
    pub(crate) description: &'static str,
    pub(crate) files: Vec<Option<Vec<usize>>>,
    pub(crate) frequency_order: Vec<usize>,
    pub(crate) expected_order: Vec<usize>,
}

pub(crate) struct CensusTestCase {
    pub(crate) description: &'static str,
    pub(crate) fixtures: &'static [ShopFile],
    pub(crate) expected_summary: Result<CensusSummary, String>,
}

pub(crate) struct ShopFile {
    pub(crate) path: &'static str,
    pub(crate) contents: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CensusSummary {
    pub(crate) files: usize,
    pub(crate) parsed: usize,
    pub(crate) unparsed: usize,
    pub(crate) missing_rule: usize,
    pub(crate) unresolved_name: usize,
    pub(crate) parse_fidelity: usize,
    pub(crate) convertible: usize,
    pub(crate) most_used_files: usize,
    pub(crate) select_files: usize,
    pub(crate) qualify_files: usize,
    pub(crate) all_rules_convertible: usize,
    pub(crate) text_mentions_sql: bool,
}

/// Each root's construct with its direct, downstream, total and sole file counts.
pub(crate) type RootCounts = (String, [usize; 4]);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BlockerSummary {
    pub(crate) roots: Vec<RootCounts>,
    pub(crate) single_root_files: usize,
    pub(crate) possibly_incomplete_files: usize,
    /// Root pairs unblocking at least one file, with the files they unblock.
    pub(crate) pairs: Vec<(Vec<String>, usize)>,
    /// Each refused model, the refused models it reads and its unknown relations.
    pub(crate) files: Vec<(String, Vec<String>, usize)>,
}

pub(crate) struct BlockerTestCase {
    pub(crate) description: &'static str,
    pub(crate) fixtures: &'static [ShopFile],
    pub(crate) names: bool,
    pub(crate) expected_summary: BlockerSummary,
}

pub(crate) struct PlacedTestCase {
    pub(crate) description: &'static str,
    pub(crate) fixtures: &'static [ShopFile],
    pub(crate) names: bool,
    pub(crate) expected_placed: (Vec<(String, Vec<String>)>, Vec<String>),
}

pub(crate) struct FixedBlockerTestCase {
    pub(crate) description: &'static str,
    pub(crate) conversion: ConversionOptions,
    pub(crate) expected_summary: BlockerSummary,
}

pub(crate) struct DivergenceTallyTestCase {
    pub(crate) description: &'static str,
    pub(crate) fixtures: &'static [ShopFile],
    pub(crate) divergence: &'static str,
    /// The models relying on `divergence`, or None when none does.
    pub(crate) expected_models: Option<usize>,
}
