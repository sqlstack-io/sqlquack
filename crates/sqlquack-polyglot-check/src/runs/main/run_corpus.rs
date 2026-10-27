//! `run_corpus`: every entry of a corpus compared.

use crate::comparison::main::compare_faithful::compare_faithful;
use crate::comparison::main::compare_parse::compare_parse;
use crate::comparison::main::compare_type_names::compare_type_names;
use crate::comparison::models::Outcome;
use crate::corpora::models::CorpusEntry;
use crate::expected::constants::OK_PREFIX;
use crate::expected::models::ExpectedDifferences;
use crate::runs::models::{Finding, RunSummary};

/// Compares every entry, with faithful calls differing only as `expected` allows.
pub fn run_corpus(entries: &[CorpusEntry], expected: &ExpectedDifferences) -> RunSummary {
    let mut summary = RunSummary {
        entries: entries.len(),
        ..RunSummary::default()
    };
    for entry in entries {
        let comparison = compare_parse(&entry.sql, entry.dialect);
        summary.polyglot += comparison.polyglot;
        summary.sqlquack += comparison.sqlquack;
        let from = format!("{} ({:?})", entry.from, entry.dialect);
        let faithful = compare_faithful(&entry.sql, entry.dialect);
        for spelling in faithful.spellings {
            if !expected.allows(&spelling.name, &spelling.polyglot) {
                summary.unlisted.push(Finding {
                    from: from.clone(),
                    detail: format!("unlisted: {} -> {}", spelling.name, spelling.polyglot),
                });
            }
            *summary.spellings.entry(spelling).or_default() += 1;
        }
        if let Some(detail) = faithful.unexplained {
            summary.unexplained.push(Finding {
                from: from.clone(),
                detail,
            });
        }
        for detail in compare_type_names(&entry.sql, entry.dialect) {
            summary.type_names.push(Finding {
                from: from.clone(),
                detail,
            });
        }
        match comparison.outcome {
            Outcome::SameTree => summary.same_trees += 1,
            Outcome::SameError => summary.same_errors += 1,
            Outcome::Differs { polyglot, sqlquack } => {
                let fix = expected
                    .fix_for(&entry.sql, &polyglot)
                    .filter(|_| sqlquack.starts_with(OK_PREFIX));
                let dialect = format!("{:?}", entry.dialect);
                let rejection = expected.rejection_for(&entry.sql, &dialect, &polyglot, &sqlquack);
                match (fix, rejection) {
                    (Some(fix), _) => *summary.fixed.entry(fix.id.clone()).or_default() += 1,
                    (None, Some(rejection)) => {
                        *summary.rejected.entry(rejection.id.clone()).or_default() += 1;
                    }
                    (None, None) => summary.differences.push(Finding {
                        from,
                        detail: format!("polyglot: {polyglot}\nsqlquack: {sqlquack}"),
                    }),
                }
            }
            Outcome::TokensLost { reason } => summary.tokens_lost.push(Finding {
                from,
                detail: reason,
            }),
        }
    }
    summary
}
