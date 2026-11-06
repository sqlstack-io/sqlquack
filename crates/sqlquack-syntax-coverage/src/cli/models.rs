//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::formats::constants::COVERAGE_ROOT;

#[derive(Debug, Parser)]
#[command(
    name = "syntax-coverage",
    about = "Checks documented syntax coverage, Snowflake's unless the root's dialect.toml says otherwise."
)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

/// Where the files are, and how to print.
#[derive(Debug, Args)]
pub struct Common {
    /// The coverage root.
    #[arg(long, default_value = COVERAGE_ROOT)]
    pub root: PathBuf,
    /// Print JSON on stdout alone.
    #[arg(long)]
    pub json: bool,
}

/// What the translation measure converts with, and how much of the catalogue.
#[derive(Debug, Args)]
pub struct TranslationArguments {
    /// The `sqlquack` binary whose `convert` is measured.
    #[arg(long)]
    pub sqlquack: PathBuf,
    /// The policy conversion relies on.
    #[arg(long, default_value = "sqlquack.toml")]
    pub policy: PathBuf,
    #[arg(long)]
    pub family: Option<String>,
    /// Conversions run at once.
    #[arg(long, default_value_t = 4)]
    pub jobs: usize,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load every coverage file and report format problems.
    Validate {
        #[command(flatten)]
        common: Common,
    },
    /// Counts for the whole catalogue and per family; fails on any unresolved reference.
    Summary {
        /// A focus file (`focus/<dialect>.toml`): also report its forms; fails on any unknown id.
        #[arg(long)]
        focus: Option<PathBuf>,
        #[command(flatten)]
        common: Common,
    },
    /// Every form of one family with its obligation completion and reasons.
    Family {
        family: String,
        #[command(flatten)]
        common: Common,
    },
    /// Every unsatisfied required obligation, with its reason.
    Missing {
        #[arg(long)]
        family: Option<String>,
        #[command(flatten)]
        common: Common,
    },
    /// Every form whose reviews are all stale, with its fingerprint and what each review recorded.
    Stale {
        #[arg(long)]
        family: Option<String>,
        #[command(flatten)]
        common: Common,
    },
    /// Counts per documentation page, a secondary view of the same forms.
    Pages {
        #[arg(long)]
        family: Option<String>,
        #[command(flatten)]
        common: Common,
    },
    /// The boundary and nesting-depth obligations of one form, with every case and verdict.
    Boundaries {
        #[arg(long)]
        form: String,
        #[command(flatten)]
        common: Common,
    },
    /// The current claims as JSON, bound to the frozen catalogue commit, as baseline.json holds.
    Snapshot {
        /// The commit whose catalogue must equal the working tree's.
        #[arg(long)]
        catalogue_commit: String,
        #[command(flatten)]
        common: Common,
    },
    /// Internal: one verdict line per SQL line on stdin, for the isolated local parse.
    #[command(hide = true)]
    ParseWorker,
    /// What conversion makes of every case Snowflake parsed, by form and family (decision D107).
    Translation {
        #[command(flatten)]
        arguments: TranslationArguments,
        #[command(flatten)]
        common: Common,
    },
    /// Recorded statements linked to forms, the result-matching measure; fails on any change (D139).
    Results {
        /// The file `sqlquack verify --translation-statements` writes.
        statements: PathBuf,
        /// Record the measure as the new baseline instead of comparing with it.
        #[arg(long)]
        write_baseline: bool,
        /// Write the full report, with the worklist and capture queue, as JSON to this file.
        #[arg(long)]
        report: Option<PathBuf>,
        /// The target branch's baseline: print the diff against it and fail on any regression.
        #[arg(long)]
        against: Option<PathBuf>,
        #[command(flatten)]
        common: Common,
    },
    /// A snapshot against a later one or the current tree; fails on any regression.
    Diff {
        base: PathBuf,
        /// A later snapshot; the current tree when omitted.
        head: Option<PathBuf>,
        #[command(flatten)]
        common: Common,
    },
}
