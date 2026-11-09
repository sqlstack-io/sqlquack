//! Folds per-file censuses into the report.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::models::SourceDialect;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::models::RefusalCategory;

use crate::census::_helpers::coverage::{CoverageInput, convertible_at, greedy_order};
use crate::census::_helpers::inspect::FileCensus;
use crate::census::models::{
    BlockerReport, CensusReport, CensusTotals, ConstructCount, CoveragePoint, PlanReport,
    ProvenanceCounts, RefusalCount, RefusalTotals,
};
use sqlquack_convert::snowflake::analysis::main::assumption_id::assumption_id;

pub(crate) fn aggregate(
    files: &[FileCensus],
    dialect: SourceDialect,
    coverage_points: &[usize],
    names: bool,
) -> CensusReport {
    let mut usage: BTreeMap<&Construct, usize> = BTreeMap::new();
    for file in files {
        for construct in &file.constructs {
            *usage.entry(construct).or_default() += 1;
        }
    }
    let ids: BTreeMap<&Construct, usize> = usage
        .keys()
        .enumerate()
        .map(|(id, construct)| (*construct, id))
        .collect();
    let mut ranked: Vec<(&Construct, usize)> = usage.iter().map(|(c, n)| (*c, *n)).collect();
    ranked.sort_by_key(|(construct, count)| (Reverse(*count), *construct));
    let frequency_order: Vec<usize> = ranked.iter().map(|(construct, _)| ids[construct]).collect();
    let file_ids: Vec<Option<Vec<usize>>> =
        files.iter().map(|file| construct_ids(file, &ids)).collect();
    CensusReport {
        dialect,
        totals: totals(files, &usage),
        refusals: refusal_totals(files),
        constructs: ranked
            .iter()
            .map(|(construct, count)| ConstructCount {
                construct: construct.key(),
                files: *count,
            })
            .collect(),
        coverage: coverage(&file_ids, &frequency_order, coverage_points),
        provenance: provenance_counts(files, names),
        blockers: BlockerReport::default(),
        plan: PlanReport::default(),
    }
}

fn totals(files: &[FileCensus], usage: &BTreeMap<&Construct, usize>) -> CensusTotals {
    let mut constructs_by_family: BTreeMap<String, usize> = BTreeMap::new();
    for construct in usage.keys() {
        *constructs_by_family
            .entry(construct.family.prefix().to_owned())
            .or_default() += 1;
    }
    let parsed = files.iter().filter(|file| file.parsed).count();
    CensusTotals {
        files: files.len(),
        parsed,
        unparsed: files.len() - parsed,
        empty: files
            .iter()
            .filter(|file| file.parsed && file.statements == 0)
            .count(),
        statements: files.iter().map(|file| file.statements).sum(),
        distinct_constructs: usage.len(),
        constructs_by_family,
    }
}

fn refusal_totals(files: &[FileCensus]) -> RefusalTotals {
    let count = |category: RefusalCategory| {
        files
            .iter()
            .filter(|file| file.refusal_categories.contains(&category))
            .count()
    };
    RefusalTotals {
        missing_rule: count(RefusalCategory::MissingRule),
        not_expressible: count(RefusalCategory::NotExpressible),
        parse_fidelity: count(RefusalCategory::ParseFidelity),
        type_condition: count(RefusalCategory::TypeCondition),
        unresolved_name: count(RefusalCategory::UnresolvedName),
        conversion_budget: count(RefusalCategory::ConversionBudget),
        divergence_refused: count(RefusalCategory::DivergenceRefused),
        by_construct: refusal_counts(files),
        convertible: files
            .iter()
            .filter(|file| file.parsed && file.refusal_categories.is_empty())
            .count(),
        convertible_strict: 0,
        convertible_no_assumptions: 0,
        convertible_strict_no_assumptions: 0,
        assumptions: BTreeMap::new(),
    }
}

fn provenance_counts(files: &[FileCensus], names: bool) -> ProvenanceCounts {
    let mut counts = ProvenanceCounts::default();
    for provenance in files.iter().filter_map(|file| file.provenance.as_ref()) {
        for (target, ids) in [
            (&mut counts.rules, &provenance.rules),
            (&mut counts.building_blocks, &provenance.building_blocks),
            (&mut counts.typing, &provenance.typing),
        ] {
            for id in ids {
                *target.entry(id.clone()).or_default() += 1;
            }
        }
        let mut labels: BTreeSet<String> = BTreeSet::new();
        for (divergence, constructs) in &provenance.divergences {
            let divergence = match names {
                true => divergence.as_str(),
                false => assumption_id(divergence),
            };
            for construct in constructs {
                labels.insert(format!("{divergence} at {construct}"));
            }
        }
        for label in labels {
            *counts.divergences.entry(label).or_default() += 1;
        }
    }
    counts
}

fn refusal_counts(files: &[FileCensus]) -> Vec<RefusalCount> {
    let mut counts: BTreeMap<(RefusalCategory, String), usize> = BTreeMap::new();
    for file in files {
        for key in &file.refusal_constructs {
            *counts.entry(key.clone()).or_default() += 1;
        }
    }
    let mut sorted: Vec<RefusalCount> = counts
        .into_iter()
        .map(|((category, construct), files)| RefusalCount {
            category: category.label().to_owned(),
            construct,
            files,
        })
        .collect();
    sorted.sort_by(|left, right| {
        right
            .files
            .cmp(&left.files)
            .then_with(|| left.construct.cmp(&right.construct))
    });
    sorted
}

fn coverage(
    files: &[Option<Vec<usize>>],
    frequency_order: &[usize],
    coverage_points: &[usize],
) -> Vec<CoveragePoint> {
    let construct_count = frequency_order.len();
    let mut points: BTreeSet<usize> = coverage_points
        .iter()
        .copied()
        .filter(|point| *point < construct_count)
        .collect();
    points.insert(construct_count);
    let points: Vec<usize> = points.into_iter().collect();
    let input = CoverageInput {
        files,
        construct_count,
        frequency_order,
    };
    let frequency = convertible_at(files, frequency_order, construct_count, &points);
    let greedy = convertible_at(files, &greedy_order(&input), construct_count, &points);
    let share = |count: usize| {
        if files.is_empty() {
            0.0
        } else {
            count as f64 / files.len() as f64
        }
    };
    points
        .iter()
        .zip(frequency.iter().zip(&greedy))
        .map(|(rules, (frequency, greedy))| CoveragePoint {
            rules: *rules,
            frequency_order: *frequency,
            greedy_order: *greedy,
            frequency_share: share(*frequency),
            greedy_share: share(*greedy),
        })
        .collect()
}

fn construct_ids(file: &FileCensus, ids: &BTreeMap<&Construct, usize>) -> Option<Vec<usize>> {
    if !file.parsed {
        return None;
    }
    Some(
        file.constructs
            .iter()
            .map(|construct| ids[construct])
            .collect(),
    )
}
