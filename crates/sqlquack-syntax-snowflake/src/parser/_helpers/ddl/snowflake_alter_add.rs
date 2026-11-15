//! Snowflake's ALTER TABLE ADD of a column or an out-of-line constraint, read with the CREATE TABLE grammar.

use crate::parser::_helpers::ddl::snowflake_create_table::table_constraint;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::{AlterTableAction, ColumnDef};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A column after ADD [ COLUMN ] [ IF NOT EXISTS ]: Snowflake's needs a type and takes the CREATE TABLE column options.
    pub(crate) fn parse_added_column_def(&mut self) -> Result<ColumnDef> {
        if !self.is_snowflake() {
            return self.parse_column_def();
        }
        let column = self.parse_checked_table_column()?;
        if column.no_type {
            return Err(self.parse_error("Expected a data type"));
        }
        Ok(column)
    }

    /// Snowflake's `ADD <out-of-line constraint>`, or None (position unchanged) when no constraint follows ADD.
    pub(crate) fn parse_snowflake_added_constraint(&mut self) -> Result<Option<AlterTableAction>> {
        let starts = self.check(TokenType::Constraint)
            || self.check(TokenType::Unique)
            || self.check(TokenType::PrimaryKey)
            || self.check(TokenType::ForeignKey)
            || self.check(TokenType::Check);
        if !self.is_snowflake() || !starts {
            return Ok(None);
        }
        let constraint = self.parse_checked_constraint(false)?;
        Ok(Some(AlterTableAction::AddConstraint(table_constraint(
            constraint,
        )?)))
    }
}
