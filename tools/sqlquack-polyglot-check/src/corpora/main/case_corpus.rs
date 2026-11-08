//! `case_corpus`: SQLQuack's cases, as Snowflake source SQL and converted DuckDB SQL.

use std::path::Path;

use sqlquack::analysis::models::PipelineConverter;
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_rules::session::main::conversion_options::conversion_options;
use sqlquack_syntax::Dialect;

use crate::corpora::models::CorpusEntry;

/// Every case's source SQL, and the DuckDB SQL of every case that converts.
pub fn case_corpus(cases: &Path, policy: &Path) -> Result<Vec<CorpusEntry>, String> {
    let policy = load_policy(policy).map_err(|error| error.to_string())?;
    let loaded = load_cases(cases, &policy).map_err(|error| error.to_string())?;
    let converter = PipelineConverter {
        options: conversion_options(&policy, false),
        python: None,
    };
    let mut entries = Vec::with_capacity(loaded.len() * 2);
    for case in &loaded {
        entries.push(CorpusEntry {
            dialect: Dialect::Snowflake,
            sql: case.source_sql.clone(),
            from: case.reference.clone(),
        });
        if let Conversion::Converted { sql, .. } = converter.convert(case) {
            entries.push(CorpusEntry {
                dialect: Dialect::DuckDB,
                sql,
                from: format!("{} (converted)", case.reference),
            });
        }
    }
    Ok(entries)
}
