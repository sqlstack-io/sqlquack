//! SQLQuack converts Snowflake SQL into DuckDB SQL and refuses whatever it cannot prove correct.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod census;
pub mod cli;
pub mod index;
pub(crate) mod planning;
pub mod project;
pub mod review;
pub mod run;
pub mod statements;
pub mod terminal;
