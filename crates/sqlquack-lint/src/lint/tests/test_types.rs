pub(crate) struct LintTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub expected_codes: &'static [&'static str],
    pub expected_anchors: &'static [(&'static str, &'static str)],
}

pub(crate) struct EmptyFixtureLintTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: &'static str,
    pub(crate) in_fixture_harness: bool,
    pub(crate) expected_diagnostic_count: usize,
}

pub(crate) struct BatchLintTestCase {
    pub(crate) description: &'static str,
    pub(crate) queries: &'static [&'static str],
    pub(crate) expected_responses: usize,
}

pub(crate) struct LintMetadataTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub expected_code: &'static str,
    pub expected_message: &'static str,
    pub expected_remediation: &'static str,
}

pub(crate) struct LintFixTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub expected_code: &'static str,
    pub expected_replacement: Option<&'static str>,
}

pub(crate) struct AdditionalLintRuleTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub rule: &'static str,
    pub expected_anchor: Option<&'static str>,
    pub expected_replacement: Option<&'static str>,
}

pub(crate) struct CeremonialCteLintTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub expected_count: usize,
    pub expected_start: Option<u64>,
}

pub(crate) struct PlainSqlLintTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub rule: &'static str,
    pub expected_count: usize,
}

pub(crate) struct SqlMacroOutputLintTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    /// The text a `@sql.` call generated, and the names the call passed.
    pub(crate) output: &'static str,
    pub(crate) relations: &'static [&'static str],
    pub(crate) expected_count: usize,
}

pub(crate) struct DynamicOutputStarLintTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_count: usize,
    pub(crate) expected_prefix: &'static str,
}

pub(crate) struct DependencyImportLintTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dependency_identifiers: &'static [&'static str],
    pub(crate) expected_count: usize,
}

pub(crate) struct DialectLintRuleTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub dialect: &'static str,
    pub rule: &'static str,
    pub expected_count: usize,
    pub expected_fix: bool,
    pub expected_reason: &'static str,
}

pub(crate) struct InvalidRuleTestCase {
    pub description: &'static str,
    pub rule: &'static str,
    pub expected_message: &'static str,
}

pub(crate) struct FunctionDepthTestCase {
    pub description: &'static str,
    pub depth: usize,
    pub expected_diagnostic_count: usize,
}

pub(crate) struct FunctionDepthFailureTestCase {
    pub description: &'static str,
    pub depth: usize,
    pub expected_code: &'static str,
    pub expected_limit: &'static str,
}

pub(crate) struct NestingRefusalTestCase {
    pub description: &'static str,
    pub dialect: &'static str,
    pub depth: usize,
    pub expected_code: &'static str,
}

pub(crate) struct CrossJoinTestCase {
    pub description: &'static str,
    pub sql: &'static str,
    pub dialect: &'static str,
    pub one_row_relations: &'static [&'static str],
    pub expected_messages: &'static [&'static str],
}

pub(crate) struct CrossJoinSubjectTestCase {
    pub description: &'static str,
    pub message: &'static str,
    pub expected_subject: Option<(&'static str, Option<&'static str>)>,
}

pub(crate) struct UnavailableDialectTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: &'static str,
    pub(crate) expected_unavailable: Option<&'static str>,
    pub(crate) expected_diagnostics: usize,
}

pub(crate) struct UnknownDialectTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: &'static str,
    pub(crate) expected_message: &'static str,
}

pub(crate) struct LongChainTestCase {
    pub description: &'static str,
    pub dialect: &'static str,
    /// The statement, with `{chain}` standing for `terms` copies of `term` joined by `separator`.
    pub template: &'static str,
    pub term: &'static str,
    pub separator: &'static str,
    pub terms: usize,
    pub expected_lints: bool,
}

pub(crate) struct JoinPredicateTestCase {
    pub description: &'static str,
    pub dialect: &'static str,
    pub predicate: &'static str,
    pub expected_findings: usize,
}
