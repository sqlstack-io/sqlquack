//! One rule's coverage: its combinations, hazards and shapes outside its domain.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_core::provenance::models::Shape;

use crate::cases::models::LoadedCase;
use crate::coverage::_helpers::combinations::{combinations, render_shape, shape_combinations};
use crate::coverage::_helpers::matching::{rows_show, source_shows};
use crate::coverage::models::{
    CoverageTotals, Covering, Form, HazardCatalogue, RuleCoverage, RuleDomain,
};
use crate::coverage::types::{RuleCoverings, RuleShapes};
use crate::verify::models::{CaseVerdict, Verdict};

/// The coverage of a declared rule by its passing cases and every converted shape.
pub(crate) fn rule_coverage(
    domain: &RuleDomain,
    catalogue: &HazardCatalogue,
    covering: &[Covering<'_>],
    converted: Option<&BTreeSet<&Shape>>,
) -> RuleCoverage {
    let mut accepted: BTreeSet<String> = BTreeSet::new();
    for Covering { shape, .. } in covering {
        for form in &domain.forms {
            accepted.extend(shape_combinations(form, shape).into_iter().flatten());
        }
    }
    let all: BTreeSet<String> = domain.forms.iter().flat_map(combinations).collect();
    let mut undeclared_shapes: BTreeSet<String> = BTreeSet::new();
    for shape in converted.into_iter().flatten() {
        if !domain
            .forms
            .iter()
            .any(|form| shape_combinations(form, shape).is_some())
        {
            undeclared_shapes.insert(render_shape(shape));
        }
    }
    let mut hazards = 0;
    let mut hazard_gaps: Vec<String> = Vec::new();
    let mut unrecordable_hazards: Vec<String> = Vec::new();
    for (index, form) in domain.forms.iter().enumerate() {
        for (unit, covered) in hazard_units(index, form, catalogue, covering) {
            match covered {
                Some(true) => hazards += 1,
                Some(false) => {
                    hazards += 1;
                    hazard_gaps.push(unit);
                }
                None => unrecordable_hazards.push(unit),
            }
        }
    }
    RuleCoverage {
        rule: domain.id.clone(),
        declared: true,
        undeclarable: domain.undeclarable.clone(),
        combinations: all.len(),
        shape_gaps: all.difference(&accepted).cloned().collect(),
        undeclared_shapes: undeclared_shapes.into_iter().collect(),
        hazards,
        hazard_gaps,
        unrecordable_hazards,
    }
}

/// Each hazard `form` needs and whether a passing case covers it; `None` when unrecordable.
fn hazard_units(
    index: usize,
    form: &Form,
    catalogue: &HazardCatalogue,
    covering: &[Covering<'_>],
) -> Vec<(String, Option<bool>)> {
    let fitting: Vec<&Covering<'_>> = covering
        .iter()
        .filter(|covering| shape_combinations(form, covering.shape).is_some())
        .collect();
    let mut units: Vec<(String, Option<bool>)> = Vec::new();
    for (position, argument) in form.arguments.iter().flatten().enumerate() {
        if argument.grammar {
            continue;
        }
        for family in &argument.families {
            let Some(hazards) = catalogue.families.get(family) else {
                continue;
            };
            for hazard in &hazards.hazards {
                let covered = fitting.iter().any(|covering| {
                    covering.shape.arguments.get(position).is_some_and(|fed| {
                        fed.family == *family
                            && source_shows(
                                hazard,
                                fed.source.as_deref(),
                                (covering.case, covering.errored && covering.alongside > 1),
                                covering.errored,
                            )
                    })
                });
                units.push((
                    format!(
                        "{}[{index}] argument {position} {family}: {}",
                        form.construct, hazard.id
                    ),
                    (hazard.unrecordable.is_none()
                        && !form
                            .unrecordable
                            .contains_key(&format!("{position}.{family}.{}", hazard.id)))
                    .then_some(covered),
                ));
            }
        }
    }
    if let Some(rows) = form.rows.as_deref() {
        for hazard in catalogue
            .families
            .get(rows)
            .map(|family| family.hazards.as_slice())
            .unwrap_or_default()
        {
            let covered = fitting.iter().any(|covering| {
                !covering.errored && rows_show(hazard, covering.shape, covering.case)
            });
            units.push((
                format!("{}[{index}] {rows}: {}", form.construct, hazard.id),
                hazard.unrecordable.is_none().then_some(covered),
            ));
        }
    }
    units
}

/// The coverage of a rule no domain declares: its converted shapes.
pub(crate) fn undeclared_rule(rule: &str, converted: Option<&BTreeSet<&Shape>>) -> RuleCoverage {
    let mut shapes: BTreeSet<String> = BTreeSet::new();
    for shape in converted.into_iter().flatten() {
        shapes.insert(render_shape(shape));
    }
    RuleCoverage {
        rule: rule.to_owned(),
        undeclared_shapes: shapes.into_iter().collect(),
        ..RuleCoverage::default()
    }
}

/// Each rule's passing shapes and every converted shape, from the verdicts of `cases`.
pub(crate) fn rule_shapes<'a>(
    cases: &'a [LoadedCase],
    verdicts: &'a [CaseVerdict],
) -> (RuleCoverings<'a>, RuleShapes<'a>) {
    let by_reference: BTreeMap<&str, &LoadedCase> = cases
        .iter()
        .map(|case| (case.reference.as_str(), case))
        .collect();
    let mut covering: RuleCoverings<'a> = BTreeMap::new();
    let mut converted: RuleShapes<'a> = BTreeMap::new();
    for verdict in verdicts {
        let (Some(provenance), Some(case)) = (
            verdict.provenance.as_ref(),
            by_reference.get(verdict.reference.as_str()),
        ) else {
            continue;
        };
        let passing = matches!(verdict.verdict, Verdict::Passed | Verdict::Diverged { .. });
        let errored = match &verdict.verdict {
            Verdict::Passed => verdict.error_check.is_some(),
            Verdict::Diverged { rows_proven, .. } => !rows_proven,
            _ => false,
        };
        for (rule, shapes) in &provenance.shapes {
            for shape in shapes {
                converted.entry(rule.as_str()).or_default().insert(shape);
                if passing {
                    covering.entry(rule.as_str()).or_default().push(Covering {
                        shape,
                        case,
                        errored,
                        alongside: shapes.len(),
                    });
                }
            }
        }
    }
    (covering, converted)
}

/// `totals` with `coverage` counted.
pub(crate) fn add(totals: CoverageTotals, coverage: &RuleCoverage) -> CoverageTotals {
    CoverageTotals {
        rules: totals.rules + 1,
        undeclared_rules: totals.undeclared_rules + usize::from(!coverage.declared),
        undeclarable_rules: totals.undeclarable_rules
            + usize::from(coverage.undeclarable.is_some()),
        combinations: totals.combinations + coverage.combinations,
        shape_gaps: totals.shape_gaps + coverage.shape_gaps.len(),
        undeclared_shapes: totals.undeclared_shapes
            + match coverage.declared {
                true => coverage.undeclared_shapes.len(),
                false => 0,
            },
        hazards: totals.hazards + coverage.hazards,
        hazard_gaps: totals.hazard_gaps + coverage.hazard_gaps.len(),
        unrecordable_hazards: totals.unrecordable_hazards + coverage.unrecordable_hazards.len(),
    }
}
