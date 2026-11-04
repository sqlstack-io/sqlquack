//! Loading one kind of coverage file; each returns its records, declared ids and problems.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::formats::_helpers::files::{
    family_files, listed, read_jsonl, read_jsonl_lines, read_toml, stray,
};
use crate::formats::_helpers::records::{
    check_alias, check_case, check_definition, check_form, check_index, check_obligation,
    check_receipt, check_restatements, check_review, check_source,
};
use crate::formats::_helpers::retired::{retire, withdrawn_receipts};
use crate::formats::constants::{
    ALIASES_FILE, CASES_DIR, CATALOGUE_DIR, INDEXES_FILE, OBLIGATIONS_DIR, RECEIPT_REPORT_SUFFIX,
    RECEIPTS_DIR, REVIEWS_DIR, SOURCES_FILE,
};
use crate::formats::models::{
    Alias, AliasesFile, Case, CatalogueFile, Index, IndexesFile, Obligation, ObligationsFile,
    Receipt, Review, ReviewsFile, Source, SourcesFile,
};
use crate::formats::types::FamilyCatalogue;

/// What one section of the tree yields.
pub(crate) struct Section<T> {
    pub(crate) records: T,
    pub(crate) declared: Vec<String>,
    pub(crate) hashes: BTreeMap<String, String>,
    pub(crate) problems: Vec<String>,
}

impl<T> Section<T> {
    fn of(records: T, declared: Vec<String>, problems: Vec<String>) -> Self {
        Self {
            records,
            declared,
            hashes: BTreeMap::new(),
            problems,
        }
    }
}

/// An importer run report, which sits beside its receipts file.
fn is_report(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(RECEIPT_REPORT_SUFFIX))
}

pub(crate) fn load_sources(root: &Path) -> Section<Vec<Source>> {
    match read_toml::<SourcesFile>(&root.join(SOURCES_FILE)) {
        Ok((file, hash)) => {
            let problems: Vec<String> = file.sources.iter().flat_map(check_source).collect();
            let declared: Vec<String> = file
                .sources
                .iter()
                .map(|source| source.id.clone())
                .collect();
            let mut section: Section<Vec<Source>> = Section::of(file.sources, declared, problems);
            section.hashes.insert(SOURCES_FILE.to_owned(), hash);
            section
        }
        Err(problem) => Section::of(Vec::new(), Vec::new(), vec![problem]),
    }
}

pub(crate) fn load_indexes(root: &Path) -> Section<Vec<Index>> {
    match read_toml::<IndexesFile>(&root.join(INDEXES_FILE)) {
        Ok((file, _)) => {
            let problems: Vec<String> = file.indexes.iter().flat_map(check_index).collect();
            Section::of(file.indexes, Vec::new(), problems)
        }
        Err(problem) => Section::of(Vec::new(), Vec::new(), vec![problem]),
    }
}

/// Renames, from a file that is absent until the first one.
pub(crate) fn load_aliases(root: &Path) -> Section<Vec<Alias>> {
    let path = root.join(ALIASES_FILE);
    if !path.exists() {
        return Section::of(Vec::new(), Vec::new(), Vec::new());
    }
    match read_toml::<AliasesFile>(&path) {
        Ok((file, hash)) => {
            let problems: Vec<String> = file.aliases.iter().flat_map(check_alias).collect();
            let mut section: Section<Vec<Alias>> = Section::of(file.aliases, Vec::new(), problems);
            section.hashes.insert(ALIASES_FILE.to_owned(), hash);
            section
        }
        Err(problem) => Section::of(Vec::new(), Vec::new(), vec![problem]),
    }
}

/// Every family's forms and definitions; a record citing an unpinned source is a problem.
pub(crate) fn load_catalogue(root: &Path, sources: &[Source]) -> Section<FamilyCatalogue> {
    let source_ids: BTreeSet<&str> = sources.iter().map(|source| source.id.as_str()).collect();
    let (files, mut problems) = family_files(root, CATALOGUE_DIR, "toml");
    let mut records: FamilyCatalogue = BTreeMap::new();
    let mut declared: Vec<String> = Vec::new();
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();
    for (family, path) in files {
        let (file, hash) = match read_toml::<CatalogueFile>(&path) {
            Ok(loaded) => loaded,
            Err(problem) => {
                problems.push(problem);
                continue;
            }
        };
        let defined: BTreeSet<(&str, &str)> = file
            .definitions
            .iter()
            .map(|definition| (definition.source.as_str(), definition.name.as_str()))
            .collect();
        for definition in &file.definitions {
            problems.extend(check_definition(definition, &source_ids));
            declared.push(definition.id.clone());
        }
        for form in &file.forms {
            problems.extend(check_form(form, &family, &source_ids, &defined));
            declared.push(form.id.clone());
        }
        hashes.insert(format!("{CATALOGUE_DIR}/{family}.toml"), hash);
        records.insert(family, (file.forms, file.definitions));
    }
    problems.extend(check_restatements(
        sources,
        records.values().flat_map(|(forms, _)| forms),
    ));
    Section {
        records,
        declared,
        hashes,
        problems,
    }
}

pub(crate) fn load_obligations(root: &Path) -> Section<BTreeMap<String, Vec<Obligation>>> {
    let (files, mut problems) = family_files(root, OBLIGATIONS_DIR, "toml");
    let mut records: BTreeMap<String, Vec<Obligation>> = BTreeMap::new();
    let mut declared: Vec<String> = Vec::new();
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();
    for (family, path) in files {
        let (file, hash) = match read_toml::<ObligationsFile>(&path) {
            Ok(loaded) => loaded,
            Err(problem) => {
                problems.push(problem);
                continue;
            }
        };
        for obligation in &file.obligations {
            problems.extend(check_obligation(obligation, &family));
            declared.push(obligation.id.clone());
        }
        hashes.insert(format!("{OBLIGATIONS_DIR}/{family}.toml"), hash);
        records.insert(family, file.obligations);
    }
    Section {
        records,
        declared,
        hashes,
        problems,
    }
}

pub(crate) fn load_cases(root: &Path) -> Section<BTreeMap<String, Vec<Case>>> {
    let (files, mut problems) = family_files(root, CASES_DIR, "jsonl");
    let mut records: BTreeMap<String, Vec<Case>> = BTreeMap::new();
    let mut declared: Vec<String> = Vec::new();
    for (family, path) in files {
        let (cases, read_problems) = read_jsonl::<Case>(&path);
        problems.extend(read_problems);
        for case in &cases {
            problems.extend(check_case(case, &family));
            declared.push(case.case_id.clone());
        }
        records.insert(family, cases);
    }
    Section::of(records, declared, problems)
}

/// Every receipts file, less the receipts superseded or withdrawn by the run reports beside them.
pub(crate) fn load_receipts(root: &Path) -> Section<(Vec<Receipt>, Vec<String>)> {
    let (paths, others) = listed(&root.join(RECEIPTS_DIR), "jsonl");
    let mut problems: Vec<String> = others
        .iter()
        .filter(|path| !is_report(path))
        .map(|path| stray(path, "jsonl"))
        .collect();
    let reports: Vec<PathBuf> = others.into_iter().filter(|path| is_report(path)).collect();
    let mut published: Vec<(Receipt, String)> = Vec::new();
    for path in paths {
        let (receipts, read_problems) = read_jsonl_lines::<Receipt>(&path);
        problems.extend(read_problems);
        problems.extend(
            receipts
                .iter()
                .flat_map(|(receipt, _)| check_receipt(receipt)),
        );
        published.extend(receipts);
    }
    let (withdrawn, report_problems) = withdrawn_receipts(&reports);
    problems.extend(report_problems);
    let (kept, retired, retire_problems) = retire(published, &withdrawn);
    problems.extend(retire_problems);
    Section::of((kept, retired), Vec::new(), problems)
}

pub(crate) fn load_reviews(root: &Path) -> Section<Vec<Review>> {
    let (paths, others) = listed(&root.join(REVIEWS_DIR), "toml");
    let mut problems: Vec<String> = others.iter().map(|path| stray(path, "toml")).collect();
    let mut records: Vec<Review> = Vec::new();
    for path in paths {
        match read_toml::<ReviewsFile>(&path) {
            Ok((file, _)) => {
                problems.extend(file.reviews.iter().flat_map(check_review));
                records.extend(file.reviews);
            }
            Err(problem) => problems.push(problem),
        }
    }
    let declared: Vec<String> = records.iter().map(|review| review.id.clone()).collect();
    Section::of(records, declared, problems)
}

/// Every section of one tree.
pub(crate) struct Sections {
    pub(crate) sources: Section<Vec<Source>>,
    pub(crate) indexes: Section<Vec<Index>>,
    pub(crate) aliases: Section<Vec<Alias>>,
    pub(crate) catalogue: Section<FamilyCatalogue>,
    pub(crate) obligations: Section<BTreeMap<String, Vec<Obligation>>>,
    pub(crate) cases: Section<BTreeMap<String, Vec<Case>>>,
    pub(crate) receipts: Section<(Vec<Receipt>, Vec<String>)>,
    pub(crate) reviews: Section<Vec<Review>>,
}

impl Sections {
    /// Every problem of every section, then every id declared twice.
    pub(crate) fn problems(&self) -> Vec<String> {
        let mut problems: Vec<String> = [
            &self.sources.problems,
            &self.indexes.problems,
            &self.aliases.problems,
            &self.catalogue.problems,
            &self.obligations.problems,
            &self.cases.problems,
            &self.receipts.problems,
            &self.reviews.problems,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
        let mut seen: BTreeSet<&String> = BTreeSet::new();
        problems.extend(
            [
                &self.sources.declared,
                &self.catalogue.declared,
                &self.obligations.declared,
                &self.cases.declared,
                &self.reviews.declared,
            ]
            .into_iter()
            .flatten()
            .filter(|id| !seen.insert(*id))
            .map(|id| format!("{id}: declared twice")),
        );
        problems
    }
}

pub(crate) fn load_sections(root: &Path) -> Sections {
    let sources: Section<Vec<Source>> = load_sources(root);
    let catalogue: Section<FamilyCatalogue> = load_catalogue(root, &sources.records);
    Sections {
        indexes: load_indexes(root),
        aliases: load_aliases(root),
        catalogue,
        obligations: load_obligations(root),
        cases: load_cases(root),
        receipts: load_receipts(root),
        reviews: load_reviews(root),
        sources,
    }
}
