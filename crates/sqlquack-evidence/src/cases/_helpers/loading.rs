//! Validating one case file and expanding its cases.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_compare::comparison::models::RowOrder;
use sqlquack_policy::policy::models::{Policy, SessionValue};

use crate::cases::_helpers::functions::{create_function, function_problems};
use crate::cases::_helpers::hashing::case_hash;
use crate::cases::_helpers::source_sql::{row_sources, source_sql};
use crate::cases::models::{
    CaseDefinition, CaseFile, CaseInput, CaseRole, EvidenceKind, HashedContent, LoadedCase,
    QueryOrder, Spelling,
};
use crate::constants::{
    CANONICAL_SUFFIX, EVIDENCE_FORMAT, ORDINAL_COLUMN, SOURCE_DIALECT, VALUE_COLUMN,
};

pub(crate) fn load_file(
    relative: &str,
    file: &CaseFile,
    policy: &Policy,
) -> Result<Vec<LoadedCase>, Vec<String>> {
    let mut problems = file_problems(relative, file, policy);
    for case in &file.cases {
        if let Some(id) = &case.expected_divergence
            && !policy.divergences.contains_key(id)
        {
            problems.push(format!("{relative}#{}: unknown divergence `{id}`", case.id));
        }
    }
    problems.extend(
        file.cases
            .iter()
            .flat_map(|case| case_problems(relative, case)),
    );
    problems.extend(
        file.cases
            .iter()
            .flat_map(|case| spelling_problems(relative, file, case)),
    );
    problems.extend(
        file.cases
            .iter()
            .flat_map(|case| evidence_problems(relative, file, case)),
    );
    problems.extend(
        file.cases
            .iter()
            .flat_map(|case| function_problems(&format!("{relative}#{}", case.id), case)),
    );
    let Some(profile) = policy.session_profiles.get(&file.session_profile) else {
        return Err(problems);
    };
    if !problems.is_empty() {
        return Err(problems);
    }
    let mut loaded: Vec<LoadedCase> = Vec::new();
    for case in &file.cases {
        let reference = format!("{relative}#{}", case.id);
        let parameters = (&profile.snowflake, &profile.pinned);
        let source = expand(file, case, Spelling::Source, parameters);
        let Some(equivalence) = &file.equivalence else {
            let role = match evidence(file, case) {
                EvidenceKind::Conversion => CaseRole::Conversion,
                EvidenceKind::Typing => CaseRole::Typing,
            };
            loaded.push(LoadedCase {
                reference,
                role,
                ..source
            });
            continue;
        };
        let canonical = expand(file, case, Spelling::Canonical, parameters);
        loaded.push(LoadedCase {
            reference: reference.clone(),
            role: CaseRole::EquivalenceSource {
                equivalence: equivalence.clone(),
                canonical_hash: canonical.hash.clone(),
            },
            ..source
        });
        loaded.push(LoadedCase {
            reference: format!("{reference}{CANONICAL_SUFFIX}"),
            role: CaseRole::EquivalenceCanonical {
                equivalence: equivalence.clone(),
            },
            ..canonical
        });
    }
    Ok(loaded)
}

fn expand(
    file: &CaseFile,
    case: &CaseDefinition,
    spelling: Spelling,
    (parameters, pinned): (
        &BTreeMap<String, SessionValue>,
        &BTreeMap<String, SessionValue>,
    ),
) -> LoadedCase {
    let sql = source_sql(case, spelling);
    let setup_sql: Vec<String> = case.functions.iter().map(create_function).collect();
    let values = case.values.unwrap_or(true);
    let hash = case_hash(&HashedContent {
        format: EVIDENCE_FORMAT,
        source_dialect: SOURCE_DIALECT,
        session_parameters: parameters,
        source_sql: &sql,
        values: (!values).then_some(false),
        setup_sql: &setup_sql,
    });
    LoadedCase {
        reference: String::new(),
        description: case.description.clone(),
        category: file.category.clone(),
        construct: file.construct.clone(),
        session_profile: file.session_profile.clone(),
        session_parameters: parameters.clone(),
        pinned_parameters: pinned.clone(),
        source_sql: sql,
        order: row_order(case),
        hash,
        role: CaseRole::Conversion,
        expected_divergence: case.expected_divergence.clone(),
        values,
        compile_error: case.compile_error,
        functions: case.functions.clone(),
        overrides: case.overrides.clone(),
        setup_sql,
        inputs: inputs(case),
        row_sources: row_sources(case, spelling),
    }
}

/// Each input column of `case` with its cells in row order.
fn inputs(case: &CaseDefinition) -> Vec<CaseInput> {
    let mut inputs: Vec<CaseInput> = Vec::new();
    for (index, column) in case.columns.iter().enumerate() {
        let mut cells: Vec<String> = Vec::new();
        for row in &case.rows {
            cells.extend(row.get(index).cloned());
        }
        inputs.push(CaseInput {
            column: column.clone(),
            cells,
        });
    }
    inputs
}

/// What `case` is evidence for: its own `evidence`, else its file's, else conversion.
fn evidence(file: &CaseFile, case: &CaseDefinition) -> EvidenceKind {
    case.evidence
        .or(file.evidence)
        .unwrap_or(EvidenceKind::Conversion)
}

/// Typing cases cannot prove equivalences or divergences; only they may drop values.
fn evidence_problems(relative: &str, file: &CaseFile, case: &CaseDefinition) -> Vec<String> {
    let label = format!("{relative}#{}", case.id);
    let mut problems: Vec<String> = Vec::new();
    let typing = evidence(file, case) == EvidenceKind::Typing;
    if typing && file.equivalence.is_some() {
        problems.push(format!(
            "{label}: an equivalence file cannot hold typing cases"
        ));
    }
    if typing && case.expected_divergence.is_some() {
        problems.push(format!(
            "{label}: a typing case has no DuckDB result to diverge"
        ));
    }
    if !typing && case.values == Some(false) {
        problems.push(format!("{label}: only a typing case may record no values"));
    }
    problems
}

fn row_order(case: &CaseDefinition) -> RowOrder {
    match (&case.expression, case.order) {
        (None, Some(QueryOrder::Bag)) => RowOrder::Bag,
        _ => RowOrder::Ordered,
    }
}

fn file_problems(relative: &str, file: &CaseFile, policy: &Policy) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    if !policy.session_profiles.contains_key(&file.session_profile) {
        problems.push(format!(
            "{relative}: unknown session profile `{}`",
            file.session_profile
        ));
    }
    if file.cases.is_empty() {
        problems.push(format!("{relative}: declares no case"));
    }
    let ids: BTreeSet<&str> = file.cases.iter().map(|case| case.id.as_str()).collect();
    if ids.len() != file.cases.len() {
        problems.push(format!("{relative}: repeats a case id"));
    }
    match policy.edge_cases.get(&file.category) {
        None => problems.push(format!("{relative}: unknown category `{}`", file.category)),
        Some(requirement) => {
            let covered: BTreeSet<&str> = file
                .cases
                .iter()
                .flat_map(|case| case.tags.iter().map(String::as_str))
                .collect();
            let missing: Vec<&str> = requirement
                .required
                .iter()
                .map(String::as_str)
                .filter(|tag| !covered.contains(tag))
                .collect();
            if !missing.is_empty() {
                problems.push(format!(
                    "{relative}: category `{}` requires inputs no case covers: {}",
                    file.category,
                    missing.join(", ")
                ));
            }
        }
    }
    problems
}

fn case_problems(relative: &str, case: &CaseDefinition) -> Vec<String> {
    let label = format!("{relative}#{}", case.id);
    let mut problems: Vec<String> = Vec::new();
    if !is_identifier(&case.id) {
        problems.push(format!("{label}: the id must be a lower-case identifier"));
    }
    if case.description.trim().is_empty() {
        problems.push(format!("{label}: has no description"));
    }
    if case.rows.is_empty() {
        problems.push(format!("{label}: needs at least one input row"));
    }
    if case.rows.iter().any(|row| row.len() != case.columns.len()) {
        problems.push(format!("{label}: every row needs one value per column"));
    }
    let names: Vec<&str> = case
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    if names
        .iter()
        .any(|name| !is_identifier(name) || [ORDINAL_COLUMN, VALUE_COLUMN].contains(name))
    {
        problems.push(format!(
            "{label}: column names must be lower-case identifiers other than `{ORDINAL_COLUMN}` and `{VALUE_COLUMN}`"
        ));
    }
    match (&case.expression, &case.query, case.order) {
        (Some(_), None, None) => {}
        (None, Some(query), Some(_)) => {
            let upper = query.trim_start().to_ascii_uppercase();
            if !upper.starts_with("SELECT") && !upper.starts_with("WITH ") {
                problems.push(format!("{label}: a query must start with SELECT or WITH"));
            }
        }
        (Some(_), None, Some(_)) => problems.push(format!(
            "{label}: expressions are always in input order; drop `order`"
        )),
        (None, Some(_), None) => problems.push(format!(
            "{label}: a query needs `order = \"ordered\"` or `\"bag\"`"
        )),
        _ => problems.push(format!(
            "{label}: needs exactly one of `expression` and `query`"
        )),
    }
    problems
}

fn spelling_problems(relative: &str, file: &CaseFile, case: &CaseDefinition) -> Vec<String> {
    let label = format!("{relative}#{}", case.id);
    match (&file.equivalence, &case.canonical) {
        (Some(_), None) => vec![format!(
            "{label}: an equivalence case needs its `canonical` spelling"
        )],
        (None, Some(_)) => vec![format!(
            "{label}: `canonical` belongs only in equivalence files"
        )],
        (Some(_), Some(canonical))
            if Some(canonical) == case.expression.as_ref()
                || Some(canonical) == case.query.as_ref() =>
        {
            vec![format!(
                "{label}: the canonical spelling repeats the source spelling"
            )]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn is_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}
