//! A case's outcome from what conversion reported.

use std::collections::BTreeSet;

use crate::translation::constants::{DIVERGENCE_REFUSED, UNRESOLVED_NAME};
use crate::translation::models::{CaseOutcome, ConvertOutput};

/// The outcome of `output` for a case read with a census schema or not, and an empty setup or not.
pub(crate) fn classify(
    output: Result<ConvertOutput, String>,
    by_design: &BTreeSet<String>,
    (census_schema, empty_setup): (bool, bool),
) -> CaseOutcome {
    let report: ConvertOutput = match output {
        Ok(report) => report,
        Err(cause) => return CaseOutcome::DidNotFinish { cause },
    };
    if report.converted {
        return CaseOutcome::Translated {
            binds: empty_setup.then_some(true),
        };
    }
    let only = |category: &str| {
        !report.refusals.is_empty()
            && report
                .refusals
                .iter()
                .all(|refusal| refusal.category == category)
    };
    let constructs: Vec<String> = report
        .refusals
        .iter()
        .map(|refusal| refusal.construct.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    match () {
        () if only(UNRESOLVED_NAME) && census_schema => CaseOutcome::NeedsSchema,
        () if only(UNRESOLVED_NAME) => CaseOutcome::NoRelationInScope,
        () if only(DIVERGENCE_REFUSED) => CaseOutcome::DeclaredValueDivergence { constructs },
        () if !constructs.is_empty() && constructs.iter().all(|key| by_design.contains(key)) => {
            CaseOutcome::ByDesign { constructs }
        }
        () => CaseOutcome::NotYetTranslated {
            refusals: match report.refusals.is_empty() {
                true => vec!["no refusal reported".to_owned()],
                false => report
                    .refusals
                    .iter()
                    .map(|refusal| format!("{}: {}", refusal.category, refusal.construct))
                    .collect::<BTreeSet<String>>()
                    .into_iter()
                    .collect(),
            },
        },
    }
}
