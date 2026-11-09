//! The shared schema and scope types over Snowflake's column types.

use crate::snowflake::typing::models::SnowflakeType;

/// One column of a relation: its Snowflake name, as resolved, and Snowflake type.
pub type ColumnSpec = crate::schemas::models::ColumnSpec<SnowflakeType>;
/// The relations and project functions a Snowflake statement may use, keyed by resolved name.
pub type Schema = crate::schemas::models::Schema<SnowflakeType>;
/// A Snowflake project function's signature.
pub type FunctionSpec = crate::schemas::models::FunctionSpec<SnowflakeType>;
/// What a Snowflake project function returns.
pub type FunctionReturns = crate::schemas::models::FunctionReturns<SnowflakeType>;
/// A relation visible in a FROM clause, its columns of Snowflake types.
pub type ScopeRelation = crate::scopes::models::ScopeRelation<SnowflakeType>;
/// What a Snowflake query level can see, select-list aliases bound to values of type `A`.
pub type Scope<'a, A> = crate::scopes::models::Scope<'a, SnowflakeType, A>;
/// What a name resolves to in a Snowflake scope.
pub type Resolution<'s, A> = crate::scopes::models::Resolution<'s, SnowflakeType, A>;
