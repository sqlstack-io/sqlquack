//! `sqlquack census`.

use std::fmt::Write as _;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use crate::analysis::models::SourceDialect;

use crate::analysis::main::with_overrides::with_overrides;
use crate::census::constants::{COVERAGE_POINTS, RULE_ID_PREFIX, SLOWEST_FILES};
use crate::census::main::census_diagnostics::census_diagnostics;
use crate::census::main::census_with_progress::census_with_progress;
use crate::census::main::render_census_diagnostics::render_census_diagnostics;
use crate::census::main::render_census_text::render_census_text;
use crate::census::models::{CensusOptions, CensusRun};
use crate::cli::_helpers::evidence::divergence_options;
use crate::cli::_helpers::project::project_choices;
use crate::cli::models::CensusArguments;
use sqlquack_convert::snowflake::analysis::models::ConversionChoices;
use sqlquack_convert::snowflake::catalogue::constants::RULES;

pub(crate) fn census_command(
    arguments: &CensusArguments,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let Some(dialect) = SourceDialect::from_name(&arguments.dialect) else {
        let _ = writeln!(
            stderr,
            "sqlquack census: unsupported dialect '{}'; supported: snowflake",
            arguments.dialect
        );
        return ExitCode::from(2);
    };
    let project = match project_choices(
        &arguments.conversion,
        (None, &[], arguments.overrides.as_deref()),
    ) {
        Ok(project) => project,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::from(2);
        }
    };
    for warning in &project.warnings {
        let _ = writeln!(stderr, "sqlquack census: warning: {warning}");
    }
    let schema = project.schema;
    let conversion = match with_overrides(
        divergence_options(&project.choices),
        (
            project.overrides.as_deref(),
            &[
                project.verified_overrides.clone(),
                arguments.verified_overrides.clone(),
            ]
            .concat(),
        ),
    ) {
        Ok(conversion) => conversion,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::from(2);
        }
    };
    if !arguments.assume_supported.is_empty() && !arguments.chain_models {
        let _ = writeln!(
            stderr,
            "sqlquack census: --assume-supported needs --chain-models, since a reader of a \
             refused model converts only once the model is typed"
        );
        return ExitCode::from(2);
    }
    let named: Vec<String> = arguments
        .assume_supported
        .iter()
        .map(|name| name.trim().to_owned())
        .collect();
    if named.iter().any(String::is_empty) {
        let _ = writeln!(
            stderr,
            "sqlquack census: warning: --assume-supported has an empty key, which is ignored"
        );
    }
    let named: Vec<String> = named.into_iter().filter(|name| !name.is_empty()).collect();
    let assumed = match assumed_constructs(&named) {
        Ok(assumed) => assumed,
        Err(unknown) => {
            let _ = writeln!(
                stderr,
                "sqlquack census: --assume-supported names unknown rule '{unknown}'"
            );
            return ExitCode::from(2);
        }
    };
    let options = CensusOptions {
        dialect,
        coverage_points: COVERAGE_POINTS.to_vec(),
        schema,
        chain_models: arguments.chain_models,
        conversion,
        names: arguments.detailed && !arguments.anonymous,
        assume_supported: assumed
            .iter()
            .flat_map(|(_, constructs)| constructs.iter().cloned())
            .collect(),
        cursor_type: arguments.cursor_type,
    };
    let started = Instant::now();
    let _ = writeln!(stderr, "sqlquack census: reading SQL files…");
    let run = match census_with_progress(&arguments.paths, &options, &mut |line| {
        let _ = writeln!(stderr, "{line}");
    }) {
        Ok(run) => run,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::from(2);
        }
    };
    let outcome = match run.files.iter().any(|file| file.blocked) {
        true => ExitCode::FAILURE,
        false => ExitCode::SUCCESS,
    };
    let shown = match arguments.detailed || arguments.anonymous {
        true => coverage_report(&run, arguments, &assumed),
        false => {
            default_report(&run, arguments, &project.choices).map(|text| (text, String::new()))
        }
    };
    let (text, notes) = match shown {
        Ok(shown) => shown,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::from(2);
        }
    };
    if stdout.write_all(text.as_bytes()).is_err() {
        return ExitCode::from(2);
    }
    let _ = write!(stderr, "{notes}");
    let _ = writeln!(
        stderr,
        "sqlquack census: done, {} files in {:.1} s",
        run.files.len(),
        started.elapsed().as_secs_f64()
    );
    outcome
}

/// Each blocked file's diagnostics and the summary, as text or JSON.
fn default_report(
    run: &CensusRun,
    arguments: &CensusArguments,
    choices: &ConversionChoices,
) -> Result<String, String> {
    let census = census_diagnostics(&run.files, choices).map_err(|error| error.to_string())?;
    match arguments.output.json() {
        true => serde_json::to_string_pretty(&census)
            .map(|json| json + "\n")
            .map_err(|error| error.to_string()),
        false => Ok(render_census_diagnostics(&census)),
    }
}

/// The coverage report, as text or JSON, and its stderr notes: unmatched assumptions, slowest files.
fn coverage_report(
    run: &CensusRun,
    arguments: &CensusArguments,
    assumed: &[(String, Vec<String>)],
) -> Result<(String, String), String> {
    let report = &run.report;
    let mut notes = String::new();
    for (name, constructs) in assumed {
        if constructs
            .iter()
            .all(|construct| report.plan.unmatched.contains(construct))
        {
            let _ = writeln!(
                notes,
                "sqlquack census: warning: --assume-supported '{name}' matches no collected \
                 refusal, so it changes no bound"
            );
        }
    }
    let text = match arguments.output.json() {
        true => serde_json::to_string_pretty(report)
            .map(|json| json + "\n")
            .map_err(|error| error.to_string())?,
        false => render_census_text(report, arguments.limit),
    };
    if arguments.detailed {
        let _ = writeln!(notes, "sqlquack census: slowest files:");
        for (path, elapsed) in run.timings.iter().take(SLOWEST_FILES) {
            let seconds = elapsed.as_secs_f64();
            let _ = match arguments.anonymous {
                false => writeln!(notes, "  {seconds:>8.2} s  {}", path.display()),
                true => writeln!(notes, "  {seconds:>8.2} s"),
            };
        }
    }
    Ok((text, notes))
}

/// Each name with the constructs it stands for: a rule's for `rule.<id>`, else the construct key.
fn assumed_constructs(names: &[String]) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut assumed: Vec<(String, Vec<String>)> = Vec::new();
    for name in names {
        if !name.starts_with(RULE_ID_PREFIX) {
            assumed.push((name.clone(), vec![name.clone()]));
            continue;
        }
        let rule = RULES
            .iter()
            .find(|rule| rule.id == name)
            .ok_or_else(|| name.clone())?;
        assumed.push((
            name.clone(),
            rule.constructs
                .iter()
                .map(|construct| (*construct).to_owned())
                .collect(),
        ));
    }
    Ok(assumed)
}
