//! `render_census_diagnostics`: each blocked file's diagnostics, upstream warnings, and a summary.

use std::fmt::Write;

use sqlquack_diagnostics::diagnostics::main::render_diagnostics::render_diagnostics;

use crate::census::_helpers::files::model_name;
use crate::census::constants::SUMMARY_CAUSES;
use crate::census::models::CensusDiagnostics;

/// Diagnostics, then upstream warnings, then the non-zero counts, as text.
pub fn render_census_diagnostics(census: &CensusDiagnostics) -> String {
    let mut text = String::new();
    for file in &census.blocked {
        let rendered = render_diagnostics(
            &file.diagnostics,
            &file.source,
            &file.path.display().to_string(),
        );
        let _ = writeln!(text, "{rendered}");
    }
    for warning in &census.upstream {
        for upstream in &warning.reads {
            let _ = writeln!(
                text,
                "warning: {} converts, but reads {}, which doesn't\n --> {}\n",
                warning.path.display(),
                model_name(upstream),
                warning.path.display()
            );
        }
    }
    let share = match census.files {
        0 => 0,
        files => census.converted * 100 / files,
    };
    let _ = writeln!(
        text,
        "Converted {} of {} files ({share}%)",
        census.converted, census.files
    );
    if !census.blocked.is_empty() {
        let mut shown: Vec<String> = census
            .causes
            .iter()
            .take(SUMMARY_CAUSES)
            .map(|cause| format!("{} {}", cause.code, cause.cause))
            .collect();
        if census.causes.len() > SUMMARY_CAUSES {
            shown.push(format!("and {} more", census.causes.len() - SUMMARY_CAUSES));
        }
        let _ = writeln!(
            text,
            "Blocked: {} ({})",
            [
                counted(census.blocked.len(), "file"),
                counted(census.causes.len(), "cause"),
            ]
            .join(", "),
            shown.join(", ")
        );
    }
    if !census.upstream.is_empty() {
        let _ = writeln!(
            text,
            "Converts with a blocked upstream model: {}",
            census.upstream.len()
        );
    }
    text
}

fn counted(count: usize, noun: &str) -> String {
    match count {
        1 => format!("1 {noun}"),
        _ => format!("{count} {noun}s"),
    }
}
