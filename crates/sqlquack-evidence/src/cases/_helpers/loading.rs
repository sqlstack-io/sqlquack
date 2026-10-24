//! Validating one case file and expanding its cases.

use std::collections::BTreeSet;

use sqlquack_compare::comparison::models::RowOrder;
use sqlquack_policy::policy::models::Policy;

use crate::cases::_helpers::hashing::case_hash;
use crate::cases::_helpers::source_sql::source_sql;
use crate::cases::models::{CaseDefinition, CaseFile, HashedContent, LoadedCase, QueryOrder};
use crate::constants::{EVIDENCE_FORMAT, ORDINAL_COLUMN, SOURCE_DIALECT, VALUE_COLUMN};

pub(crate) fn load_file(
    relative: &str,
    file: &CaseFile,
    policy: &Policy,
) -> Result<Vec<LoadedCase>, Vec<String>> {
    let mut problems = file_problems(relative, file, policy);
    problems.extend(
        file.cases
            .iter()
            .flat_map(|case| case_problems(relative, case)),
    );
    let Some(profile) = policy.session_profiles.get(&file.session_profile) else {
        return Err(problems);
    };
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(file
        .cases
        .iter()
        .map(|case| {
            let sql = source_sql(case);
            let hash = case_hash(&HashedContent {
                format: EVIDENCE_FORMAT,
                source_dialect: SOURCE_DIALECT,
                session_parameters: &profile.snowflake,
                source_sql: &sql,
            });
            LoadedCase {
                reference: format!("{relative}#{}", case.id),
                category: file.category.clone(),
                construct: file.construct.clone(),
                session_profile: file.session_profile.clone(),
                session_parameters: profile.snowflake.clone(),
                source_sql: sql,
                order: row_order(case),
                hash,
            }
        })
        .collect())
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
            if !query
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("SELECT")
            {
                problems.push(format!("{label}: a query must start with SELECT"));
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

fn is_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}
