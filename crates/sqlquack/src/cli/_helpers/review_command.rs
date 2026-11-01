//! `sqlquack review`.

use std::io::Write;
use std::process::ExitCode;

use crate::analysis::main::conversion_options::conversion_options;
use crate::analysis::models::PipelineConverter;
use crate::cli::_helpers::verify_command::run_verification;
use crate::cli::models::ReviewArguments;
use crate::review::main::export_review::export_review;
use crate::review::models::ReviewFile;

pub(crate) fn review_command(arguments: &ReviewArguments, stderr: &mut dyn Write) -> ExitCode {
    let _ = writeln!(
        stderr,
        "sqlquack review: verifying cases and writing review files…"
    );
    match review(arguments) {
        Ok(count) => {
            let _ = writeln!(
                stderr,
                "sqlquack review: done, {count} files in {}",
                arguments.out.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack review: failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn review(arguments: &ReviewArguments) -> Result<usize, String> {
    let (evidence, report) = run_verification(&arguments.paths, None, None)?;
    let converter = PipelineConverter {
        options: conversion_options(&evidence.policy, false),
        python: None,
    };
    let files: Vec<ReviewFile> = export_review(
        &evidence.cases,
        &report,
        &arguments.paths.results,
        &converter,
    );
    for file in &files {
        let path = arguments.out.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(&path, &file.text)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(files.len())
}
