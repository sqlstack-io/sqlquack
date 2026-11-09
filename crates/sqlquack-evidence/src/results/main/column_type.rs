//! `column_type`: a recorded column's Snowflake type name.

use crate::results::_helpers::type_names::snowflake_type;
use crate::results::models::WarehouseColumn;

/// The Snowflake type name of a column as the warehouse described it, such as `NUMBER(38,0)`.
pub fn column_type(column: &WarehouseColumn) -> String {
    snowflake_type(column)
}
