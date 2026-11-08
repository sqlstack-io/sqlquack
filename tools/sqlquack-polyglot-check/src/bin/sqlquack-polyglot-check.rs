//! `sqlquack-polyglot-check`: compares SQLQuack's parser with polyglot's on corpora.

use std::process::ExitCode;

use sqlquack_polyglot_check::cli::main::run_check::run_check;

fn main() -> ExitCode {
    run_check()
}
