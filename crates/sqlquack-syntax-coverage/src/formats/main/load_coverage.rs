//! `load_coverage`: every syntax-coverage file under a root, format-checked.

use std::collections::BTreeMap;
use std::path::Path;

use crate::formats::_helpers::sections::{Sections, load_sections};
use crate::formats::models::{Definition, Form, SyntaxCoverage};

/// Every file under `root`, or every format problem found; nothing is skipped or defaulted.
pub fn load_coverage(root: &Path) -> Result<SyntaxCoverage, Vec<String>> {
    let sections: Sections = load_sections(root);
    let problems: Vec<String> = sections.problems();
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
        reviews: sections.reviews.records,
        file_hashes,
    })
}
