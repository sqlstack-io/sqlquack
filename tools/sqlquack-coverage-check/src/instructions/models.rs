//! The committed instruction counts and the workloads they measure.

use std::collections::BTreeMap;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// A fixed workload whose instructions in one function the guard counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Workload {
    /// Strict Snowflake parsing of every workload statement.
    Parse,
    /// Lineage and findings, in one analysis, of every parsed workload statement.
    Analysis,
}

impl Workload {
    /// Every workload, in the order the guard runs them.
    pub const ALL: [Self; 2] = [Self::Parse, Self::Analysis];

    /// The workload's name, as the baseline keys its count.
    pub fn name(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Analysis => "analysis",
        }
    }
}

/// `coverage/instruction-counts.toml`: each workload's count, and how far a run may move from it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionBaseline {
    /// The architecture the counts were recorded on; another counts different instructions.
    pub arch: String,
    /// The compiler the counts were recorded with, as `rustc --version` names it; another compiler
    /// emits different instructions, so a toolchain bump records new counts.
    pub rustc: String,
    /// How far, in percent of the baseline, a count may move either way.
    pub tolerance_percent: u64,
    /// Each workload's count in the program's own code, shared libraries excluded.
    pub counts: BTreeMap<String, u64>,
}
