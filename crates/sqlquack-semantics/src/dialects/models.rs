//! What the registry hands out: the dialects a query is read in and resolved for.

use crate::dialects::types::DialectProfile;

/// The dialect SQL is written in, and the one it is resolved for, whose names may clash.
#[derive(Clone, Copy, Debug)]
pub struct Profiles {
    /// The dialect the SQL is written in, which decides what a name means.
    pub source: &'static dyn DialectProfile,
    /// The dialect the names must also stay apart in.
    pub target: &'static dyn DialectProfile,
}
