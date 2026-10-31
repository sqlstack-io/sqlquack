//! `render_census_text`: the census as a plain-text report.

use std::fmt::Write;

use crate::census::constants::PLACED_PER_FILE;
use crate::census::models::{BlockedFile, BlockerReport, CensusReport, RefusalCount};
use crate::planning::main::render_plan_text::render_plan_text;

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
    let refusals = &report.refusals;
    let _ = writeln!(
        text,
        "  files refused: missing rule {}, outside type conditions {}, unresolved name {}, \
         not expressible {}, parse fidelity {}, conversion budget {}",
        refusals.missing_rule,
        refusals.type_condition,
        refusals.unresolved_name,
        refusals.not_expressible,
        refusals.parse_fidelity,
        refusals.conversion_budget,
    );
    let _ = writeln!(
        text,
        "  convertible: strict {} (every model upstream converts too), unit-testable {} \
         (the file's own SQL converts; {} with an upstream model refused or not scanned)",
        refusals.convertible_strict,
        refusals.convertible,
        report.blockers.upstream_refused.len()
    );
    let relied: Vec<String> = refusals
        .assumptions
        .iter()
        .map(|(id, files)| format!("{id} in {files}"))
        .collect();
    let _ = writeln!(
        text,
        "  with no named assumption: strict {}, unit-testable {} (relying on one: {})",
        refusals.convertible_strict_no_assumptions,
        refusals.convertible_no_assumptions,
        match relied.is_empty() {
            true => "none".to_owned(),
            false => relied.join(", "),
        }
    );
    for (divergence, files) in &report.provenance.divergences {
        let _ = writeln!(
            text,
            "  converted with a known divergence: {divergence} in {files} files"
        );
    }
    render_refusals(&mut text, &refusals.by_construct, limit);
    render_blockers(&mut text, &report.blockers, limit);
    render_plan_text(&mut text, &report.plan);
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

fn render_refusals(text: &mut String, counts: &[RefusalCount], limit: usize) {
    let _ = writeln!(text);
    let _ = writeln!(text, "Refusals by category and construct (top {limit})");
    for count in counts.iter().take(limit) {
        let _ = writeln!(
            text,
            "  {:>6}  {}: {}",
            count.files, count.category, count.construct
        );
    }
}

fn render_blockers(text: &mut String, blockers: &BlockerReport, limit: usize) {
    let _ = writeln!(text);
    let _ = writeln!(
        text,
        "Root blockers through the model graph, every refusal collected (top {limit}; {} of {} \
         refused files have one root, {} possibly incomplete)",
        blockers.single_root_files,
        blockers.files.len(),
        blockers.possibly_incomplete_files
    );
    let _ = writeln!(
        text,
        "  {:>6}  {:>9}  {:>6}  {:>6}  {:>10}  root",
        "sole", "uncertain", "total", "direct", "downstream"
    );
    for root in blockers.roots.iter().take(limit) {
        let _ = writeln!(
            text,
            "  {:>6}  {:>9}  {:>6}  {:>6}  {:>10}  {}: {}",
            root.sole,
            root.sole_uncertain,
            root.total,
            root.direct,
            root.downstream,
            root.category,
            root.construct
        );
    }
    let _ = writeln!(text);
    let _ = writeln!(
        text,
        "Root combinations by the files fixing them all unblocks"
    );
    let _ = writeln!(
        text,
        "  {:>6}  {:>6}  {:>10}  {:>9}  roots",
        "files", "direct", "downstream", "uncertain"
    );
    for combination in &blockers.combinations {
        let roots: Vec<String> = combination
            .roots
            .iter()
            .map(|root| format!("{}: {}", root.category, root.construct))
            .collect();
        let _ = writeln!(
            text,
            "  {:>6}  {:>6}  {:>10}  {:>9}  {}",
            combination.files,
            combination.direct,
            combination.downstream,
            combination.uncertain,
            roots.join(" + ")
        );
    }
    if !blockers.upstream_refused.is_empty() {
        let _ = writeln!(text);
        let _ = writeln!(
            text,
            "Unit-testable but not strict: files with an upstream model refused or not scanned (first {limit})"
        );
        for file in blockers.upstream_refused.iter().take(limit) {
            let _ = writeln!(text, "  {}  {}", file.model, file.labels.join(", "));
        }
    }
    let placed: Vec<&BlockedFile> = blockers
        .files
        .iter()
        .filter(|file| !file.placed.is_empty())
        .take(limit)
        .collect();
    if placed.is_empty() {
        return;
    }
    let _ = writeln!(text);
    let _ = writeln!(text, "Refusal positions (first {limit} refused files)");
    for file in placed {
        for refusal in file.placed.iter().take(PLACED_PER_FILE) {
            let position = match (refusal.line, refusal.column) {
                (Some(line), Some(column)) => format!("{line}:{column}"),
                _ => "-".to_owned(),
            };
            let _ = writeln!(
                text,
                "  {}  {position}  {}: {}",
                file.model, refusal.category, refusal.construct
            );
        }
        if file.placed.len() > PLACED_PER_FILE {
            let _ = writeln!(
                text,
                "  {}  +{} more",
                file.model,
                file.placed.len() - PLACED_PER_FILE
            );
        }
    }
}
