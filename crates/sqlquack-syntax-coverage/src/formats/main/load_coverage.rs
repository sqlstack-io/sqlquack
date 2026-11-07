//! `load_coverage`: every syntax-coverage file under a root, format-checked.

use std::collections::BTreeMap;
use std::path::Path;

use crate::formats::_helpers::dialect::{load_dialect, set_dialect};
use crate::formats::_helpers::sections::{Sections, load_sections};
use crate::formats::constants::MARK_DOCUMENTED;
use crate::formats::models::{Case, Definition, Form, SyntaxCoverage};

/// Every file under `root`, or every format problem; the root's dialect becomes this thread's.
pub fn load_coverage(root: &Path) -> Result<SyntaxCoverage, Vec<String>> {
    set_dialect(load_dialect(root)?);
    let sections: Sections = load_sections(root);
    let mut problems: Vec<String> = sections.problems();
    problems.extend(unmatched_marks(&sections));
    if !problems.is_empty() {
        return Err(problems);
    }
    let mut forms: BTreeMap<String, Vec<Form>> = BTreeMap::new();
    let mut definitions: BTreeMap<String, Vec<Definition>> = BTreeMap::new();
    for (family, (family_forms, family_definitions)) in sections.catalogue.records {
        forms.insert(family.clone(), family_forms);
        definitions.insert(family, family_definitions);
    }
    let mut file_hashes: BTreeMap<String, String> = sections.catalogue.hashes;
    file_hashes.extend(sections.obligations.hashes);
    file_hashes.extend(sections.sources.hashes);
    file_hashes.extend(sections.aliases.hashes);
    Ok(SyntaxCoverage {
        sources: sections.sources.records,
        indexes: sections.indexes.records,
        aliases: sections.aliases.records,
        catalogue: forms,
        definitions,
        obligations: sections.obligations.records,
        cases: sections.cases.records,
        receipts: sections.receipts.records.0,
        retired_receipts: sections.receipts.records.1,
        receipt_sql: sections.receipts.records.2,
        reviews: sections.reviews.records,
        unobservable: sections.unobservable.records.0,
        diagnostics: sections.unobservable.records.1,
        readings: sections.readings.records,
        file_hashes,
        documented_cases: sections
            .marks
            .records
            .into_iter()
            .filter(|mark| mark.mark == MARK_DOCUMENTED)
            .map(|mark| mark.case_id)
            .collect(),
    })
}

/// Marks naming no case, or a case whose SQL or obligations differ from the mark's.
fn unmatched_marks(sections: &Sections) -> Vec<String> {
    let cases: BTreeMap<&str, &Case> = sections
        .cases
        .records
        .values()
        .flatten()
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    sections
        .marks
        .records
        .iter()
        .filter(|mark| {
            cases.get(mark.case_id.as_str()).is_none_or(|case| {
                case.sql_sha256 != mark.sql_sha256 || case.obligations != mark.obligations
            })
        })
        .map(|mark| {
            format!(
                "mark {}: matches no case's SQL and obligations",
                mark.case_id
            )
        })
        .collect()
}
