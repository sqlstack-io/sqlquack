//! Run summaries as printed text.

use std::fmt::Write;

use crate::cli::constants::{MILLISECONDS, SHOWN_DETAIL, SHOWN_FINDINGS};
use crate::runs::models::{ParserTimes, RunSummary};

/// The first findings, unless only counts may be shown, then a line of counts and timings.
pub(crate) fn report(label: &str, summary: &RunSummary, counts_only: bool) -> String {
    let mut text = String::new();
    if !counts_only {
        for finding in summary
            .differences
            .iter()
            .chain(&summary.tokens_lost)
            .chain(&summary.type_names)
            .chain(&summary.unlisted)
            .chain(&summary.unexplained)
            .chain(&summary.located)
            .take(SHOWN_FINDINGS)
        {
            let detail: String = finding.detail.chars().take(SHOWN_DETAIL).collect();
            let _ = writeln!(text, "{}\n{detail}\n", finding.from);
        }
    }
    let _ = writeln!(
        text,
        "{label}: {} entries, {} same trees, {} same errors, {} fixed, {} rejected, {} kept quoting, {} structured, {} differences, {} without lossless \
         tokens, {} cast type names unlike polyglot's, {} faithful calls kept, {} unlisted, {} \
         unexplained, {} located parses unlike faithful; parse time polyglot {:.1} ms, sqlquack \
         {:.1} ms",
        summary.entries,
        summary.same_trees,
        summary.same_errors,
        summary.fixed.values().sum::<usize>(),
        summary.rejected.values().sum::<usize>(),
        summary.kept_quoting.values().sum::<usize>(),
        summary.structured.values().sum::<usize>(),
        summary.differences.len(),
        summary.tokens_lost.len(),
        summary.type_names.len(),
        summary.spellings.values().sum::<usize>(),
        summary.unlisted.len(),
        summary.unexplained.len(),
        summary.located.len(),
        summary.polyglot.as_secs_f64() * MILLISECONDS,
        summary.sqlquack.as_secs_f64() * MILLISECONDS,
    );
    text
}

/// A line of median parse times and their ratio.
pub(crate) fn timing(label: &str, rounds: usize, times: ParserTimes) -> String {
    let polyglot = times.polyglot.as_secs_f64() * MILLISECONDS;
    let sqlquack = times.sqlquack.as_secs_f64() * MILLISECONDS;
    format!(
        "{label}: median of {rounds} rounds, polyglot {polyglot:.1} ms, sqlquack {sqlquack:.1} ms \
         ({:.3}x)",
        sqlquack / polyglot.max(f64::MIN_POSITIVE)
    )
}

/// One line per kept spelling: name, polyglot's node and the number of calls.
pub(crate) fn spelling_counts(summary: &RunSummary) -> String {
    let mut text = String::new();
    for (spelling, count) in &summary.spellings {
        let _ = writeln!(
            text,
            "spelling\t{}\t{}\t{count}",
            spelling.name, spelling.polyglot
        );
    }
    text
}
