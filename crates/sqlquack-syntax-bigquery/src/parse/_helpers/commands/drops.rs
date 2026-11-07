//! The `DROP`, `UNDROP` and `TRUNCATE` statements (googlesql.tm); a drop `ON` a parent is a gap.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::{DropFunction, DropView};
use sqlquack_syntax::expressions::statements::models::{Truncate, TruncateTarget};
use sqlquack_syntax::expressions::tables::models::{DropObject, DropTable, Undrop};

use crate::parse::_helpers::commands::routines::parameter_type;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::constants::{
    AGGREGATE_FUNCTION_KIND, FUNCTION_KIND, INDEX_KINDS, MATERIALIZED_VIEW_KIND, SCHEMA_KIND,
};

/// A `DropObject` of `kind` named `name`.
fn dropped(kind: &str, name: TableRef, if_exists: bool) -> Expression {
    Expression::DropObject(Box::new(DropObject {
        concurrently: false,
        objects: Vec::new(),
        kind: kind.to_owned(),
        name,
        if_exists,
        cascade: false,
        restrict: false,
        force: false,
    }))
}

impl Parser<'_> {
    /// `DROP ...`.
    pub(crate) fn drop_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("DROP")?;
        if self.eat_kws(&["ROW", "ACCESS", "POLICY"]) {
            self.eat_kws(&["IF", "EXISTS"]);
            self.ident()?;
            self.expect_kw("ON")?;
            self.table_path()?;
            self.gap("drop_row_access_policy", start);
            return Ok(Expression::Null(
                sqlquack_syntax::expressions::names::models::Null,
            ));
        }
        if self.kw("ALL") && self.kw_at(1, "ROW") {
            self.at += 2;
            self.eat_kw("ACCESS");
            self.expect_kws(&["POLICIES", "ON"])?;
            self.table_path()?;
            self.gap("drop_all_row_access_policies", start);
            return Ok(Expression::Null(
                sqlquack_syntax::expressions::names::models::Null,
            ));
        }
        if self.eat_kws(&["TABLE", "FUNCTION"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: TableRef = self.table_path()?;
            if self.is_char('(') {
                return Err(self.error_with(
                    "Parameters are not supported for DROP TABLE FUNCTION because table \
                     functions don't support overloading",
                    self.offset(),
                    true,
                ));
            }
            return Ok(dropped("TABLE FUNCTION", name, if_exists));
        }
        if self.eat_kw("TABLE") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let mut node: DropTable = DropTable::new(String::new());
            node.names = vec![self.table_reference()?];
            node.if_exists = if_exists;
            return Ok(Expression::DropTable(Box::new(node)));
        }
        if self.eat_kws(&["SNAPSHOT", "TABLE"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            return Ok(dropped("SNAPSHOT TABLE", self.table_path()?, if_exists));
        }
        if self.eat_kw("DATA_POLICY") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            return Ok(dropped("DATA_POLICY", self.table_path()?, if_exists));
        }
        if let Some(entity) = self.generic_entity_type() {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            return Ok(dropped(&entity, self.table_path()?, if_exists));
        }
        self.drop_schema_object(start)
    }

    /// `DROP` of a schema object kind, from the kind on; `start` is where the `DROP` began.
    fn drop_schema_object(&mut self, start: usize) -> Parsing<Expression> {
        let kind_at: usize = self.offset();
        let Some(kind) = self.schema_object_kind() else {
            return Err(self.error());
        };
        if kind == AGGREGATE_FUNCTION_KIND {
            return Err(self.error_with(
                "DROP AGGREGATE FUNCTION is not supported, use DROP FUNCTION",
                kind_at,
                true,
            ));
        }
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        if matches!(kind, "VIEW" | "MATERIALIZED VIEW") {
            let mut node: DropView = DropView::new(String::new());
            node.name = self.table_reference()?;
            node.if_exists = if_exists;
            node.materialized = kind == MATERIALIZED_VIEW_KIND;
            return Ok(Expression::DropView(Box::new(node)));
        }
        let name: TableRef = self.table_path()?;
        let mut parameters: Option<Vec<DataType>> = None;
        if self.is_char('(') {
            if kind != FUNCTION_KIND {
                return Err(self.error_with(
                    "Parameters are only supported for DROP FUNCTION",
                    self.offset(),
                    true,
                ));
            }
            let declared: Vec<Expression> = self.function_parameters()?;
            parameters = Some(declared.iter().filter_map(parameter_type).collect());
        }
        let mut parent: bool = false;
        if self.kw("ON") {
            if !INDEX_KINDS.contains(&kind) {
                return Err(self.error_with(
                    &format!("DROP {kind} does not support the ON clause"),
                    self.offset(),
                    true,
                ));
            }
            self.at += 1;
            self.table_path()?;
            parent = true;
        }
        let mode_at: usize = self.offset();
        let cascade: bool = self.eat_kw("CASCADE");
        let restrict: bool = !cascade && self.eat_kw("RESTRICT");
        if (cascade || restrict) && kind != SCHEMA_KIND {
            let mode: &str = if cascade { "CASCADE" } else { "RESTRICT" };
            return Err(self.error_with(
                &format!("'{mode}' is not supported for DROP {kind}"),
                mode_at,
                true,
            ));
        }
        if parent {
            self.gap("drop_index_on_table", start);
            return Ok(Expression::Null(
                sqlquack_syntax::expressions::names::models::Null,
            ));
        }
        if kind == FUNCTION_KIND {
            return Ok(Expression::DropFunction(Box::new(DropFunction {
                name,
                parameters,
                if_exists,
                cascade: false,
            })));
        }
        Ok(Expression::DropObject(Box::new(DropObject {
            concurrently: false,
            objects: Vec::new(),
            kind: kind.to_owned(),
            name,
            if_exists,
            cascade,
            restrict,
            force: false,
        })))
    }

    /// `UNDROP SCHEMA path`; its optional clauses have no place in `Undrop` and are a gap.
    pub(crate) fn undrop_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("UNDROP")?;
        let kind_at: usize = self.offset();
        let Some(kind) = self.schema_object_kind() else {
            return Err(self.error());
        };
        if kind != SCHEMA_KIND {
            return Err(self.error_with(&format!("UNDROP {kind} is not supported"), kind_at, true));
        }
        let mut extra: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let name: TableRef = self.table_path()?;
        if self.eat_kw("FOR") {
            if !self.eat_kws(&["SYSTEM", "TIME"]) {
                self.expect_kw("SYSTEM_TIME")?;
            }
            self.expect_kws(&["AS", "OF"])?;
            self.expression()?;
            extra = true;
        }
        if self.eat_kw("OPTIONS") {
            self.options_list()?;
            extra = true;
        }
        if extra {
            self.gap("undrop_clauses", start);
        }
        Ok(Expression::Undrop(Box::new(Undrop {
            kind: kind.to_owned(),
            name,
            if_exists: false,
            rename_to: None,
        })))
    }

    /// `TRUNCATE TABLE path [WHERE expr]`; a `WHERE` has no place in `Truncate` and is a gap.
    pub(crate) fn truncate_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["TRUNCATE", "TABLE"])?;
        let table: TableRef = self.table_path()?;
        if self.kw("WHERE") {
            let start: usize = self.offset();
            self.at += 1;
            self.expression()?;
            self.gap("truncate_where", start);
        }
        Ok(Expression::Truncate(Box::new(Truncate {
            target: TruncateTarget::Table,
            if_exists: false,
            table,
            on_cluster: None,
            cascade: false,
            extra_tables: Vec::new(),
            identity: None,
            restrict: false,
            partition: None,
            error_table: false,
        })))
    }
}
