//! Statements: queries, FROM, DML, COMMENT, routines, and the statement dispatch with TRUNCATE and COPY.

pub(crate) mod alters;
pub(crate) mod definitions;
pub(crate) mod dml;
pub(crate) mod from;
pub(crate) mod objects;
pub(crate) mod queries;
pub(crate) mod routine_alters;
pub(crate) mod routines;
pub(crate) mod statements;
pub(crate) mod tables;
