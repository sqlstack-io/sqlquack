use std::collections::BTreeSet;

use crate::census::models::RootKey;
use crate::planning::_helpers::steps::greedy_steps;
use crate::planning::models::PlanInput;
use crate::planning::tests::test_types::{PlannedFile, StepSummary};

/// The greedy steps over unconnected files with the given roots, nothing assumed.
pub(crate) fn steps_of(files: &[PlannedFile]) -> Vec<StepSummary> {
    let input = PlanInput {
        own: files
            .iter()
            .map(|(roots, _)| {
                roots
                    .iter()
                    .map(|(category, construct)| RootKey {
                        category: (*category).to_owned(),
                        construct: (*construct).to_owned(),
                    })
                    .collect()
            })
            .collect(),
        needs: vec![BTreeSet::new(); files.len()],
        reads: vec![BTreeSet::new(); files.len()],
        parsed: vec![true; files.len()],
        incomplete: files.iter().map(|(_, incomplete)| *incomplete).collect(),
        missing: vec![false; files.len()],
    };
    greedy_steps(&input, &BTreeSet::new())
        .into_iter()
        .map(|step| {
            (
                step.roots.into_iter().map(|root| root.construct).collect(),
                [
                    step.unit_testable_upper_bound,
                    step.strict_upper_bound,
                    step.unit_testable_possibly_incomplete,
                ],
            )
        })
        .collect()
}
