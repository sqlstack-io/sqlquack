//! A case's outcome from its verify verdict, with its refusal codes and refused constructs.

use std::collections::BTreeSet;

use sqlquack_diagnostics::diagnostics::constants::{
    CODES, CONVERSION_BUDGET, DIVERGENCE_REFUSED, MISSING_RULE, NOT_EXPRESSIBLE, PARSE_FIDELITY,
    TYPE_CONDITION, UNRESOLVED_NAME,
};
use sqlquack_diagnostics::refusals::models::RefusalCategory;
use sqlquack_evidence::cases::models::{CaseRole, LoadedCase};
use sqlquack_evidence::verify::models::{CaseVerdict, Conversion, ErrorCheck, Verdict};
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_policy::policy::constants::HASH_DIVERGENCE;

use crate::statements::constants::{CATEGORIES, FUNCTION_REASON_PREFIX, UNCODED};
use crate::statements::models::Outcome;
use crate::statements::types::Judged;

/// The outcome, codes and refused constructs of one case.
pub(crate) fn outcome(
    case: &LoadedCase,
    verdict: &CaseVerdict,
    converter: &dyn CaseConverter,
) -> Judged {
    let none = (Vec::new(), Vec::new());
    let (outcome, (codes, refused)) = match &verdict.verdict {
        Verdict::Failed { .. } => (Outcome::Differs, none),
        Verdict::Typed => (Outcome::ShapeOnly, none),
        Verdict::Unproven { .. } => (Outcome::Unproven, none),
        Verdict::Diverged { divergence, .. } => {
            (Outcome::DeclaredDivergence, diverged(verdict, divergence))
        }
        Verdict::Refused {
            reasons,
            constructs,
            snowflake_rejected,
        } => (
            match snowflake_rejected {
                true => Outcome::MatchingRejection,
                false => Outcome::Refused,
            },
            (codes(reasons, constructs), constructs.clone()),
        ),
        Verdict::Pending => match converter.convert(case) {
            Conversion::Converted { .. } => (Outcome::UnrecordedTranslates, none),
            Conversion::Refused {
                reasons,
                constructs,
            } => (
                Outcome::UnrecordedRefused,
                (codes(&reasons, &constructs), constructs.clone()),
            ),
        },
        Verdict::Passed => return passed(case, verdict),
    };
    (outcome, codes, refused)
}

/// A passed case's outcome: an opt-in divergence or a Snowflake-only proof does not match.
fn passed(case: &LoadedCase, verdict: &CaseVerdict) -> Judged {
    let opt_in = verdict
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.divergences.get(HASH_DIVERGENCE));
    if let Some(constructs) = opt_in {
        return (
            Outcome::Refused,
            vec![DIVERGENCE_REFUSED.to_owned()],
            constructs.iter().cloned().collect(),
        );
    }
    if matches!(case.role, CaseRole::EquivalenceSource { .. }) {
        return (Outcome::ShapeOnly, Vec::new(), Vec::new());
    }
    let outcome = match &verdict.error_check {
        None => Outcome::Matching,
        Some(ErrorCheck::Agrees { .. }) => Outcome::MatchingError,
        Some(ErrorCheck::DivergenceRaise { divergence }) => {
            let (codes, refused) = diverged(verdict, divergence);
            return (Outcome::DeclaredDivergence, codes, refused);
        }
        Some(_) => Outcome::Differs,
    };
    (outcome, Vec::new(), Vec::new())
}

/// A guard's raise as a code, and the constructs conversion relied on the divergence for.
fn diverged(verdict: &CaseVerdict, divergence: &str) -> (Vec<String>, Vec<String>) {
    let constructs = verdict
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.divergences.get(divergence))
        .map(|constructs| constructs.iter().cloned().collect())
        .unwrap_or_default();
    (vec![format!("divergence:{divergence}")], constructs)
}

/// The distinct codes of `reasons`, each read from the category its reason starts with.
fn codes(reasons: &[String], refused: &[String]) -> Vec<String> {
    let mut codes: BTreeSet<String> = BTreeSet::new();
    for reason in reasons {
        let reason = match reason.strip_prefix(FUNCTION_REASON_PREFIX) {
            Some(rest) => rest.split_once(": ").map_or(rest, |(_, rest)| rest),
            None => reason,
        };
        let category = CATEGORIES
            .iter()
            .find(|category| reason.starts_with(category.label()));
        codes.insert(
            category
                .map_or(UNCODED, |category| code_of(*category, refused))
                .to_owned(),
        );
    }
    codes.into_iter().collect()
}

/// The diagnostic code of `category`; a Snowflake rejection's is its refused construct's own.
fn code_of(category: RefusalCategory, refused: &[String]) -> &'static str {
    match category {
        RefusalCategory::ParseFidelity => PARSE_FIDELITY,
        RefusalCategory::MissingRule => MISSING_RULE,
        RefusalCategory::TypeCondition => TYPE_CONDITION,
        RefusalCategory::UnresolvedName => UNRESOLVED_NAME,
        RefusalCategory::NotExpressible => NOT_EXPRESSIBLE,
        RefusalCategory::ConversionBudget => CONVERSION_BUDGET,
        RefusalCategory::DivergenceRefused => DIVERGENCE_REFUSED,
        RefusalCategory::SnowflakeRejects => rejection_code(refused),
    }
}

/// The code a Snowflake rejection's refused construct names, or the missing-rule code.
fn rejection_code(refused: &[String]) -> &'static str {
    for key in refused {
        let name: &str = key.split_once(':').map_or(key.as_str(), |(_, name)| name);
        for entry in CODES {
            if entry.name == name || entry.alias == Some(name) {
                return entry.code;
            }
        }
    }
    MISSING_RULE
}
