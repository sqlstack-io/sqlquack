//! Snowflake's ALTER TABLE ADD of a column or an out-of-line constraint, read with the CREATE TABLE grammar.

use crate::parser::_helpers::ddl::snowflake_create_table::tuple_names;
use crate::parser::models::{Constraint, Parser};
use sqlquack_syntax::expressions::columns::models::{
    AlterTableAction, ColumnDef, ConstraintModifiers, ForeignKeyRef, TableConstraint,
};
use sqlquack_syntax::expressions::tree::models::Expression;
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

/// An out-of-line constraint as the table constraint ALTER TABLE ADD holds, its property words kept in order.
fn table_constraint(constraint: Constraint) -> Result<TableConstraint> {
    let Constraint {
        kind,
        name,
        operand,
        references,
        properties,
    } = constraint;
    let has_constraint_keyword = name.is_some();
    let modifiers = ConstraintModifiers {
        properties,
        ..ConstraintModifiers::default()
    };
    Ok(match kind {
        "UNIQUE" => TableConstraint::Unique {
            name,
            columns: tuple_names(operand),
            key_parts: Vec::new(),
            columns_parenthesized: true,
            modifiers,
            has_constraint_keyword,
            nulls_not_distinct: false,
        },
        "PRIMARY KEY" => TableConstraint::PrimaryKey {
            name,
            columns: tuple_names(operand),
            key_parts: Vec::new(),
            timeseries_columns: Vec::new(),
            include_columns: Vec::new(),
            modifiers,
            has_constraint_keyword,
        },
        "CHECK" => TableConstraint::Check {
            name,
            expression: operand.ok_or_else(|| {
                sqlquack_syntax::tokens::errors::Error::internal(
                    "the parser reached a state its checks rule out",
                )
            })?,
            modifiers,
        },
        _ => {
            let (table, columns) = references.ok_or_else(|| {
                sqlquack_syntax::tokens::errors::Error::internal(
                    "the parser reached a state its checks rule out",
                )
            })?;
            let Expression::Table(table) = table else {
                return Err(sqlquack_syntax::tokens::errors::Error::internal(
                    "the parser reached a state its checks rule out",
                ));
            };
            TableConstraint::ForeignKey {
                name,
                columns: tuple_names(operand),
                references: Some(Box::new(ForeignKeyRef {
                    table: *table,
                    columns: tuple_names(columns),
                    on_delete: None,
                    on_update: None,
                    on_update_first: false,
                    match_type: None,
                    match_after_actions: false,
                    constraint_name: None,
                    deferrable: None,
                    has_foreign_key_keywords: true,
                })),
                on_delete: None,
                on_update: None,
                modifiers,
            }
        }
    })
}
