//! `run_check`: the whole command.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::_helpers::corpora::named_corpora;
use crate::cli::_helpers::report::{report, spelling_counts, timing};
use crate::cli::models::Arguments;
use crate::expected::main::load_expected::load_expected;
use crate::runs::main::run_corpus::run_corpus;
use crate::runs::main::time_parsers::time_parsers;

/// Compares every named corpus; succeeds only when every entry agrees and keeps its tokens.
pub fn run_check() -> ExitCode {
    let arguments = Arguments::parse();
    std::panic::set_hook(Box::new(|_| {}));
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let expected = match load_expected(&arguments.expected) {
        Ok(expected) => expected,
        Err(error) => {
            let _ = writeln!(stderr, "sqlquack-polyglot-check: failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut exact = true;
    for (name, entries) in named_corpora(&arguments.corpus) {
        let label = match arguments.counts_only {
            true => "corpus".to_owned(),
            false => name,
        };
        let _ = writeln!(stderr, "sqlquack-polyglot-check: comparing {label}…");
        let entries = match entries {
            Ok(entries) => entries,
            Err(error) => {
                let _ = writeln!(stderr, "sqlquack-polyglot-check: failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        let summary = run_corpus(&entries, &expected);
        let _ = write!(
            stdout,
            "{}",
            report(&label, &summary, arguments.counts_only)
        );
        exact &= summary.exact();
        if arguments.spellings && !arguments.counts_only {
            let _ = write!(stdout, "{}", spelling_counts(&summary));
        }
        if arguments.timing_rounds > 0 {
            let _ = writeln!(stderr, "sqlquack-polyglot-check: timing {label}…");
            let times = time_parsers(&entries, arguments.timing_rounds);
            let _ = writeln!(stdout, "{}", timing(&label, arguments.timing_rounds, times));
        }
    }
    let (message, code) = match exact {
        true => ("done, every entry identical", ExitCode::SUCCESS),
        false => ("failed, see the differences above", ExitCode::FAILURE),
    };
    let _ = writeln!(stderr, "sqlquack-polyglot-check: {message}");
    code
}
