//! `sqlquack census`.

use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use sqlquack_core::parsing::models::SourceDialect;

use crate::census::constants::COVERAGE_POINTS;
use crate::census::main::render_census_text::render_census_text;
use crate::census::main::run_census::run_census;
use crate::census::models::CensusOptions;
use crate::cli::_helpers::evidence::divergence_options;
use crate::cli::_helpers::schema::load_schema;
use crate::cli::models::CensusArguments;

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
    let conversion = match divergence_options(arguments.policy.as_deref(), arguments.strict) {
        Ok((conversion, true)) => conversion,
        Ok((conversion, false)) => {
            let _ = writeln!(
                stderr,
                "sqlquack census: no policy file found; every known divergence is refused"
            );
            conversion
        }
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let options = CensusOptions {
        dialect,
        coverage_points: COVERAGE_POINTS.to_vec(),
        schema,
        chain_models: arguments.chain_models,
        conversion,
    };
    let started = Instant::now();
    let _ = writeln!(stderr, "sqlquack census: reading SQL files…");
    let report = match run_census(&arguments.paths, &options) {
        Ok(report) => report,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack census: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
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
    let _ = writeln!(
        stderr,
        "sqlquack census: done, {} files in {:.1} s",
        report.totals.files,
        started.elapsed().as_secs_f64()
    );
    ExitCode::SUCCESS
}
