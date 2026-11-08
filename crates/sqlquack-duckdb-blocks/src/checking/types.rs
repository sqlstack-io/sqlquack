//! How the check finds the columns of the relations a statement reads.

use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::checking::models::FunctionStub;

/// The columns of a relation by name, in order, or `None` when the name is no known relation.
pub type RelationLookup<'a> = &'a dyn Fn(&str) -> Option<Vec<(String, SnowflakeType)>>;

/// The project function called by a name, or `None` when the name is no project function.
pub type FunctionLookup<'a> = &'a dyn Fn(&str) -> Option<FunctionStub>;
