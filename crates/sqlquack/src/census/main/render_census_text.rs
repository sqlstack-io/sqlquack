//! `render_census_text`: the census as a plain-text report.

use std::fmt::Write;

use crate::census::models::CensusReport;

/// The report as text, listing at most `limit` constructs.
pub fn render_census_text(report: &CensusReport, limit: usize) -> String {
    let mut text = String::new();
    let totals = &report.totals;
    let families: Vec<String> = totals
        .constructs_by_family
        .iter()
        .map(|(family, count)| format!("{count} {family}"))
        .collect();
    let _ = writeln!(
        text,
        "Census of {} {} SQL files",
        totals.files,
        report.dialect.name()
    );
    let _ = writeln!(
        text,
        "  parsed {} ({} statements), unparsed {}, empty {}",
        totals.parsed, totals.statements, totals.unparsed, totals.empty
    );
    let _ = writeln!(
        text,
        "  distinct constructs {} ({})",
        totals.distinct_constructs,
        families.join(", ")
    );
    let fidelity = &report.fidelity;
    let _ = writeln!(
        text,
        "  print-back: identical {}, canonicalised {}, unstable {}, failed {}",
        fidelity.identical, fidelity.canonicalised, fidelity.unstable, fidelity.failed
    );
    let refusals = &report.refusals;
    let _ = writeln!(
        text,
        "  files refused: missing rule {}, not expressible {}, parse fidelity {}; convertible {}",
        refusals.missing_rule,
        refusals.not_expressible,
        refusals.parse_fidelity,
        refusals.convertible
    );
    let _ = writeln!(text);
    let _ = writeln!(text, "Convertible files as the top constructs get rules");
    let _ = writeln!(
        text,
        "  {:>6}  {:>16}  {:>16}",
        "rules", "frequency order", "greedy order"
    );
    for point in &report.coverage {
        let _ = writeln!(
            text,
            "  {:>6}  {:>8} {:>6.1}%  {:>8} {:>6.1}%",
            point.rules,
            point.frequency_order,
            point.frequency_share * 100.0,
            point.greedy_order,
            point.greedy_share * 100.0
        );
    }
    let _ = writeln!(text);
    let shown = report.constructs.len().min(limit);
    let _ = writeln!(
        text,
        "Constructs by files using them (top {shown} of {})",
        report.constructs.len()
    );
    for (rank, count) in report.constructs.iter().take(limit).enumerate() {
        let _ = writeln!(
            text,
            "  {:>4}  {:>6}  {}",
            rank + 1,
            count.files,
            count.construct
        );
    }
    text
}
