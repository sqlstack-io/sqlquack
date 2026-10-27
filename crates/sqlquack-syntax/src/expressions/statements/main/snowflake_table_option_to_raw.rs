//! `snowflake_table_option_to_raw`.

use crate::expressions::statements::models::Raw;
use crate::expressions::tree::models::Expression;

pub(crate) fn snowflake_table_option_to_raw((key, value): (String, String)) -> Expression {
    Expression::Raw(Raw {
        sql: format!("{}={}", key, value),
    })
}
