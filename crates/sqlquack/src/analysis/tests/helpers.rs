use std::collections::BTreeSet;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::analysis::main::analyze::analyze;
use crate::analysis::tests::test_types::AnalysisSummary;

pub(crate) fn summarise(sql: &str) -> AnalysisSummary {
    let analysis = analyze(sql, SourceDialect::Snowflake, &builtin_catalogue());
    let construct_refusals = analysis
        .refusals
        .iter()
        .filter(|refusal| refusal.construct.is_some())
        .count();
    AnalysisSummary {
        parsed: analysis.parsed,
        statements: analysis.statements,
        categories: analysis
            .refusals
            .iter()
            .map(|refusal| refusal.category)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        unrefused_constructs: analysis.occurrences.len().abs_diff(construct_refusals),
        unlocated_construct_refusals: analysis
            .refusals
            .iter()
            .filter(|refusal| refusal.construct.is_some() && refusal.location.is_none())
            .count(),
    }
}
