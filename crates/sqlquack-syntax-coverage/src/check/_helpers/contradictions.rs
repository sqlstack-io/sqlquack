//! Every current receipt against the parser, whether or not a catalogued case cites it.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::_helpers::receipts::snowflake_syntax;
use crate::check::models::{CaseResult, LocalVerdict, SnowflakeSyntax};
use crate::formats::models::{Receipt, SyntaxCoverage};

/// Whether the parser's verdict contradicts what Snowflake's parser did with the same statement.
fn contradicts(snowflake: SnowflakeSyntax, local: &LocalVerdict) -> bool {
    match snowflake {
        SnowflakeSyntax::Parsed => !matches!(local, LocalVerdict::Accepted { .. }),
        SnowflakeSyntax::Refused => matches!(local, LocalVerdict::Accepted { .. }),
    }
}

/// The receipt case ids the parser contradicts by SQL, setup and session, and those without SQL.
pub(crate) fn receipt_contradictions(
    coverage: &SyntaxCoverage,
    cases: &BTreeMap<&str, CaseResult>,
    parse: &dyn Fn(&str) -> LocalVerdict,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut statements: BTreeMap<(&str, &str, &str), Vec<&Receipt>> = BTreeMap::new();
    for receipt in &coverage.receipts {
        statements
            .entry((
                receipt.sql_sha256.as_str(),
                receipt.setup_sha256.as_str(),
                receipt.session_sha256.as_str(),
            ))
            .or_default()
            .push(receipt);
    }
    let mut judged: BTreeMap<&str, LocalVerdict> = cases
        .values()
        .map(|case| (case.sql_sha256.as_str(), case.local.clone()))
        .collect();
    let case_sql: BTreeMap<&str, &str> = coverage
        .cases
        .values()
        .flatten()
        .map(|case| (case.sql_sha256.as_str(), case.sql.as_str()))
        .collect();
    let mut contradicted: BTreeSet<String> = BTreeSet::new();
    let mut unjudged: BTreeSet<String> = BTreeSet::new();
    for ((sql_sha256, _, _), receipts) in &statements {
        let Some(snowflake) = snowflake_syntax(receipts) else {
            continue;
        };
        let sql: Option<&str> = case_sql
            .get(sql_sha256)
            .copied()
            .or_else(|| coverage.receipt_sql.get(*sql_sha256).map(String::as_str));
        let Some(sql) = sql else {
            unjudged.extend(receipts.iter().map(|receipt| receipt.case_id.clone()));
            continue;
        };
        let local: &LocalVerdict = judged.entry(sql_sha256).or_insert_with(|| parse(sql));
        if contradicts(snowflake, local) {
            contradicted.extend(receipts.iter().map(|receipt| receipt.case_id.clone()));
        }
    }
    (contradicted, unjudged)
}
