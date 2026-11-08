//! Development check that SQLQuack's parser gives exactly polyglot's results; never a runtime dependency.

#![forbid(unsafe_code)]

pub mod cli;
pub mod comparison;
pub mod corpora;
pub mod expected;
pub mod runs;
