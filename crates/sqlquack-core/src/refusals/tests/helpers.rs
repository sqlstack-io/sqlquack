use crate::classification::main::classify::classify;
use crate::classification::models::Occurrence;
use crate::parsing::main::parse_source::parse_source;
use crate::parsing::models::SourceDialect;
use crate::refusals::main::refuse_constructs::refuse_constructs;
use crate::refusals::main::refuse_unparsed::refuse_unparsed;
use crate::refusals::models::{Refusal, RefusalCategory};
use crate::refusals::tests::test_types::DeclaringPolicy;

fn occurrences(sql: &str) -> Vec<Occurrence> {
    parse_source(sql, SourceDialect::Snowflake)
        .expect("test SQL parses")
        .iter()
        .flat_map(|statement| classify(statement, sql, SourceDialect::Snowflake))
        .collect()
}

fn refusal_matches(refusal: &Refusal, occurrence: &Occurrence) -> bool {
    refusal.construct.as_ref() == Some(&occurrence.construct)
        && refusal.location == occurrence.location
        && refusal.reason.contains(&occurrence.construct.key())
}

pub(crate) fn unmatched_refusals(sql: &str, inexpressible: &'static [&'static str]) -> usize {
    let found = occurrences(sql);
    let refusals = refuse_constructs(&found, &DeclaringPolicy { inexpressible });
    let matched = refusals
        .iter()
        .zip(&found)
        .filter(|(refusal, occurrence)| refusal_matches(refusal, occurrence))
        .count();
    refusals.len().max(found.len()) - matched
}

pub(crate) fn not_expressible_reasons(
    sql: &str,
    inexpressible: &'static [&'static str],
) -> Vec<String> {
    refuse_constructs(&occurrences(sql), &DeclaringPolicy { inexpressible })
        .into_iter()
        .filter(|refusal| refusal.category == RefusalCategory::NotExpressible)
        .map(|refusal| refusal.reason)
        .collect()
}

pub(crate) fn unparsed_refusal(sql: &str) -> Refusal {
    refuse_unparsed(
        &parse_source(sql, SourceDialect::Snowflake).expect_err("test SQL does not parse"),
    )
}
