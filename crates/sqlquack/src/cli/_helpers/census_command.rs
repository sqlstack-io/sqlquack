//! `sqlquack census`.

use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use sqlquack_core::parsing::models::SourceDialect;

use crate::analysis::main::with_overrides::with_overrides;
use crate::census::constants::{COVERAGE_POINTS, RULE_ID_PREFIX, SLOWEST_FILES};
use crate::census::main::census_with_progress::census_with_progress;
use crate::census::main::render_census_text::render_census_text;
use crate::census::models::CensusOptions;
use crate::cli::_helpers::evidence::{chosen_policy, divergence_options};
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::CensusArguments;
use sqlquack_rules::catalogue::constants::RULES;

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
    let schema = match arguments.schema.as_deref().map(load_schema).transpose() {
        Ok(schema) => schema.unwrap_or_default(),
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let conversion = match chosen_policy(&arguments.policy, arguments.builtin_policy) {
        Ok(policy) => divergence_options(
            policy.as_ref(),
            (
                arguments.strict,
                arguments.utc_session,
                (arguments.confirm_declared_offsets, arguments.python_runtime),
            ),
        ),
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::from(2);
        }
    };
    let conversion = match with_overrides(
        conversion,
        (
            arguments.overrides.as_deref(),
            &arguments.verified_overrides,
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
        names: arguments.names,
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
            return ExitCode::FAILURE;
        }
    };
    let report = run.report;
    for (name, constructs) in &assumed {
        if constructs
            .iter()
            .all(|construct| report.plan.unmatched.contains(construct))
        {
            let _ = writeln!(
                stderr,
                "sqlquack census: warning: --assume-supported '{name}' matches no collected \
                 refusal, so it changes no bound"
            );
        }
    }
    let rendered = if arguments.json {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => json + "\n",
            Err(error) => {
                let _ = writeln!(stderr, "sqlquack census: failed: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        render_census_text(&report, arguments.limit)
    };
    if stdout.write_all(rendered.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }
    let _ = writeln!(stderr, "sqlquack census: slowest files:");
    for (path, elapsed) in run.timings.iter().take(SLOWEST_FILES) {
        let seconds = elapsed.as_secs_f64();
        let _ = match arguments.names {
            true => writeln!(stderr, "  {seconds:>8.2} s  {}", path.display()),
            false => writeln!(stderr, "  {seconds:>8.2} s"),
        };
    }
    let _ = writeln!(
        stderr,
        "sqlquack census: done, {} files in {:.1} s",
        report.totals.files,
        started.elapsed().as_secs_f64()
    );
    ExitCode::SUCCESS
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
