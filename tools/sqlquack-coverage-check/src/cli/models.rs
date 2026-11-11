//! Command-line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::depth::constants::ALLOWLIST_FILE;
use crate::formats::constants::COVERAGE_ROOT;
use crate::formats::models::LocalParser;
use crate::instructions::constants::BASELINE_FILE;
use crate::instructions::models::Workload;
use crate::verdicts::constants::{CORPORA_DIR, VERDICTS_FILE};

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
    /// Where the root's recorded `cases/` and `receipts/` are, when published apart from it.
    #[arg(long)]
    pub evidence: Option<PathBuf>,
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
    /// The `sqlquack.toml` conversion reads; refuse value differences there to measure only loud ones.
    #[arg(long, default_value = "sqlquack.toml")]
    pub config: PathBuf,
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
        /// A focus file (`coverage/syntax-<dialect>/focus.toml`): also report its forms; fails on any unknown id.
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
    ParseWorker {
        /// The dialect parser that gives the verdicts.
        #[arg(long, value_enum)]
        parser: LocalParser,
        /// Parse each text on a thread with this many bytes of stack.
        #[arg(long)]
        thread_stack: Option<usize>,
        /// Also analyse each parsed text, lineage and findings, under the dialect's profile.
        #[arg(long)]
        analyse: bool,
    },
    /// Deep nesting in every native parser; fails on an unlisted crash or hang, or a stale entry.
    DepthProbe {
        /// The Cargo profile this binary was built with; it selects the allowlist entries.
        #[arg(long)]
        profile: String,
        /// The known failures.
        #[arg(long, default_value = ALLOWLIST_FILE)]
        allowlist: PathBuf,
        /// Parse workers run at once.
        #[arg(long, default_value_t = 4)]
        jobs: usize,
        /// Print JSON on stdout alone.
        #[arg(long)]
        json: bool,
    },
    /// Callgrind's instruction counts of the parse and analysis workloads against the baseline.
    InstructionCounts {
        /// The committed counts and tolerance.
        #[arg(long, default_value = BASELINE_FILE)]
        baseline: PathBuf,
        /// Record the measured counts in the baseline instead of checking them.
        #[arg(long)]
        update: bool,
    },
    /// Internal: one instruction-count workload, run under callgrind.
    #[command(hide = true)]
    InstructionWorkload {
        #[arg(long, value_enum)]
        workload: Workload,
    },
    /// SQLQuack's parse verdict on every corpus statement against the recorded ones; fails on any change.
    CorpusVerdicts {
        /// The directory of vendored corpus files.
        #[arg(long, default_value = CORPORA_DIR)]
        corpora: PathBuf,
        /// The recorded verdicts.
        #[arg(long, default_value = VERDICTS_FILE)]
        verdicts: PathBuf,
        /// Record the current verdicts instead of comparing with them.
        #[arg(long)]
        update: bool,
    },
    /// What conversion makes of every case Snowflake parsed, by form and family.
    Translation {
        #[command(flatten)]
        arguments: TranslationArguments,
        #[command(flatten)]
        common: Common,
    },
    /// Recorded statements linked to forms, the result-matching measure; fails on any change.
    Results {
        /// The file `sqlquack verify --translation-statements` writes; not read with --files-only.
        #[arg(required_unless_present = "files_only")]
        statements: Option<PathBuf>,
        /// The recorded translation cases' root; with `--evidence`, the only place cases are read.
        #[arg(long, requires = "evidence")]
        cases: PathBuf,
        /// Check the translation files and the cases they cite, without the measure.
        #[arg(long, conflicts_with_all = ["statements", "write_baseline", "report", "against"])]
        files_only: bool,
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
