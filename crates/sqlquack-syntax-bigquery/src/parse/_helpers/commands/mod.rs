//! Statements: queries, DML, `CREATE TABLE`/`VIEW`, `DROP` and the opaque rest.

pub(crate) mod alters;
pub(crate) mod create_objects;
pub(crate) mod creates;
pub(crate) mod dml;
pub(crate) mod drops;
pub(crate) mod from;
pub(crate) mod grants;
pub(crate) mod queries;
pub(crate) mod routines;
pub(crate) mod statements;
pub(crate) mod tvf;
