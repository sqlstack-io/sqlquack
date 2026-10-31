//! `sqlquack index`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_policy::policy::models::Policy;
use walkdir::WalkDir;

use crate::analysis::models::PipelineConverter;
use crate::census::main::inspect_corpus::inspect_corpus;
use crate::census::models::CensusOptions;
use crate::cli::_helpers::evidence::{chosen_policy, divergence_options};
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::IndexArguments;
use crate::index::main::build_index::build_index;
use crate::index::main::case_records::case_records;
use crate::index::main::corpus_records::corpus_records;
use crate::index::main::render_index_text::render_index_text;
use crate::index::models::{ConversionRecord, ProvenanceIndex};

/// The case directory indexed when no path is given.
const DEFAULT_CASES: &str = "cases";

pub(crate) fn index_command(
    arguments: &IndexArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let _ = writeln!(stderr, "sqlquack index: converting cases and SQL files…");
    let policy = match chosen_policy(&arguments.policy, arguments.builtin_policy) {
        Ok(policy) => policy,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack index: failed: {error}");
            return ExitCode::from(2);
        }
    };
    let index = match index(arguments, policy.as_ref()) {
        Ok(index) => index,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack index: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let rendered = match arguments.json {
        true => serde_json::to_string_pretty(&index).unwrap_or_default() + "\n",
        false => render_index_text(&index),
    };
    let _ = stdout.write_all(rendered.as_bytes());
    let _ = writeln!(
        stderr,
        "sqlquack index: done, {} cases and {} files: {} converted, {} refused",
        index.totals.cases, index.totals.files, index.totals.converted, index.totals.refused
    );
    ExitCode::SUCCESS
}

fn index(arguments: &IndexArguments, policy: Option<&Policy>) -> Result<ProvenanceIndex, String> {
    let paths: Vec<PathBuf> = match arguments.paths.is_empty() {
        true => vec![PathBuf::from(DEFAULT_CASES)],
        false => arguments.paths.clone(),
    };
    let mut records: Vec<ConversionRecord> = Vec::new();
    let mut corpus: Vec<PathBuf> = Vec::new();
    let conversion = divergence_options(policy, (arguments.strict, arguments.utc_session));
    let converter = PipelineConverter {
        options: conversion.clone(),
    };
    for path in &paths {
        let (cases, sql) = contents(path)?;
        if cases {
            let policy = policy.ok_or(
                "indexing cases needs a policy file, which declares their session profiles",
            )?;
            let loaded = load_cases(path, policy).map_err(|error| error.to_string())?;
            records.extend(case_records(&loaded, &converter));
        }
        if sql {
            corpus.push(path.clone());
        }
    }
    if !corpus.is_empty() {
        let schema = match &arguments.schema {
            Some(path) => load_schema(path)?,
            None => Default::default(),
        };
        let options = CensusOptions {
            dialect: SourceDialect::Snowflake,
            coverage_points: Vec::new(),
            schema,
            chain_models: arguments.chain_models,
            conversion,
            names: false,
            assume_supported: Vec::new(),
            cursor_type: None,
        };
        let files = inspect_corpus(&corpus, &options).map_err(|error| error.to_string())?;
        records.extend(corpus_records(files));
    }
    Ok(build_index(&records))
}

/// Whether a path holds case files, SQL files, or both.
fn contents(path: &Path) -> Result<(bool, bool), String> {
    let extension_of = |path: &Path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
    };
    if path.is_file() {
        return match extension_of(path).as_deref() {
            Some("sql") => Ok((false, true)),
            _ => Err(format!(
                "{} is neither a SQL file nor a directory",
                path.display()
            )),
        };
    }
    if !path.is_dir() {
        return Err(format!(
            "cannot read {}: no such file or directory",
            path.display()
        ));
    }
    let mut found = (false, false);
    for entry in WalkDir::new(path).follow_links(true).into_iter().flatten() {
        match extension_of(entry.path()).as_deref() {
            Some("toml") => found.0 = true,
            Some("sql") => found.1 = true,
            _ => {}
        }
    }
    match found {
        (false, false) => Err(format!("{} holds no case or SQL files", path.display())),
        found => Ok(found),
    }
}
