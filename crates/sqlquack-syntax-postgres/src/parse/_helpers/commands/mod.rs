//! Statements: queries, FROM, DML, COMMENT, routines, and the statement dispatch with TRUNCATE and COPY.

pub(crate) mod alters;
pub(crate) mod catalog_alters;
pub(crate) mod catalog_creates;
pub(crate) mod creates;
pub(crate) mod definitions;
pub(crate) mod dml;
pub(crate) mod drops;
pub(crate) mod from;
pub(crate) mod object_alters;
pub(crate) mod objects;
pub(crate) mod queries;
pub(crate) mod role_creates;
pub(crate) mod routine_alters;
pub(crate) mod routines;
pub(crate) mod statements;
pub(crate) mod tables;
