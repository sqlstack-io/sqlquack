//! `create_table_statement` and `create_view_statement` (googlesql.tm), materialized views too.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    ConstraintModifiers, ForeignKeyRef, TableConstraint,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, CreateView, ViewColumn};
use sqlquack_syntax::expressions::tables::models::{
    ClusterByColumnsProperty, ClusteredByProperty, CreateTable, LikeProperty, OptionEntry,
    OptionsProperty, PartitionedByProperty,
};

use crate::parse::_helpers::commands::alters::{clause, path_operand};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::constants::{COPY_SOURCE, CREATE_KINDS};

/// What `CREATE` heads: `OR REPLACE`, the scope (`TEMP`, `PUBLIC`, `PRIVATE`), `MATERIALIZED`.
pub(crate) struct Head {
    pub(crate) or_replace: bool,
    pub(crate) temporary: bool,
    pub(crate) scope: Option<&'static str>,
    pub(crate) materialized: bool,
}

/// `name = value` pairs from `options_list` as the shared tree's option entries.
fn option_entries(options: Vec<Expression>) -> Vec<OptionEntry> {
    options
        .into_iter()
        .filter_map(|option| match option {
            Expression::Eq(pair) => match &pair.left {
                Expression::Identifier(key) => Some(OptionEntry {
                    key: key.clone(),
                    value: pair.right.clone(),
                }),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A `CLUSTER BY` list: its columns when each item is a bare column, else its expressions.
fn cluster_by(expressions: Vec<Expression>) -> Expression {
    let bare = |expression: &Expression| match expression {
        Expression::Column(column) => column.table.is_none(),
        _ => false,
    };
    if !expressions.iter().all(bare) {
        return Expression::ClusteredByProperty(Box::new(ClusteredByProperty {
            expressions,
            sorted_by: None,
            buckets: None,
        }));
    }
    let columns: Vec<Identifier> = expressions
        .into_iter()
        .filter_map(|expression| match expression {
            Expression::Column(column) => Some(column.name),
            _ => None,
        })
        .collect();
    Expression::ClusterByColumnsProperty(Box::new(ClusterByColumnsProperty { columns }))
}

impl Parser<'_> {
    /// `CREATE ...`; a `CREATE` of another object kind is an opaque gap.
    pub(crate) fn create_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        self.expect_kw("CREATE")?;
        let mut head = Head {
            or_replace: false,
            temporary: false,
            scope: None,
            materialized: false,
        };
        if self.kw("OR") && self.kw_at(1, "REPLACE") {
            self.at += 2;
            head.or_replace = true;
        }
        head.scope = match self.word_at(0).as_deref() {
            Some("TEMP") => Some("TEMP"),
            Some("TEMPORARY") => Some("TEMPORARY"),
            Some("PUBLIC") => Some("PUBLIC"),
            Some("PRIVATE") => Some("PRIVATE"),
            _ => None,
        };
        if head.scope.is_some() {
            self.at += 1;
        }
        head.temporary = matches!(head.scope, Some("TEMP" | "TEMPORARY"));
        head.materialized = self.eat_kw("MATERIALIZED");
        let other_table: bool = self.kw("TABLE") && self.kw_at(1, "FUNCTION");
        if !head.materialized
            && !self.kw("VIEW")
            && !(self.kw("TABLE") && !self.kw_at(1, "FUNCTION"))
            && let Some(created) = self.create_object(&head)
        {
            return created;
        }
        match self.word_at(0).as_deref() {
            Some("TABLE") if !other_table && !head.materialized => self.create_table(head),
            Some("VIEW") => self.create_view(head),
            Some(kind) if CREATE_KINDS.contains(&kind) || other_table => {
                self.at = start_at;
                self.opaque_statement()
            }
            _ => Err(self.error()),
        }
    }

    /// `IF NOT EXISTS`, when written.
    fn if_not_exists(&mut self) -> bool {
        self.eat_kws(&["IF", "NOT", "EXISTS"])
    }

    /// `DEFAULT COLLATE`, `PARTITION BY`, `CLUSTER BY`, a connection if `connects`, `OPTIONS`.
    fn table_clauses(&mut self, connects: bool) -> Parsing<(Vec<Expression>, Option<TableRef>)> {
        let mut properties: Vec<Expression> = Vec::new();
        let mut connection: Option<TableRef> = None;
        if self.kw("DEFAULT") && self.kw_at(1, "COLLATE") {
            self.at += 2;
            self.string_lit()?;
        }
        if self.kw("PARTITION") && self.kw_at(1, "BY") {
            self.at += 2;
            let this: Expression = self.expression()?;
            properties.push(Expression::PartitionedByProperty(Box::new(
                PartitionedByProperty {
                    this: Box::new(this),
                },
            )));
        }
        if self.kw("CLUSTER") && self.kw_at(1, "BY") {
            self.at += 2;
            let columns: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
            properties.push(cluster_by(columns));
        }
        if connects && self.kw("WITH") && self.kw_at(1, "CONNECTION") {
            self.at += 2;
            connection = Some(self.table_path()?);
        }
        if self.kw("OPTIONS") && self.char_at(1, '(') {
            self.at += 1;
            let entries: Vec<OptionEntry> = option_entries(self.options_list()?);
            properties.push(Expression::OptionsProperty(Box::new(OptionsProperty {
                entries,
            })));
        }
        Ok((properties, connection))
    }

    fn create_table(&mut self, head: Head) -> Parsing<Expression> {
        self.expect_kw("TABLE")?;
        let if_not_exists: bool = self.if_not_exists();
        let mut node: CreateTable = CreateTable::new(String::new());
        node.name = self.table_reference()?;
        node.if_not_exists = if_not_exists;
        node.or_replace = head.or_replace;
        node.temporary = head.temporary;
        if self.is_char('(') && !self.starts_query_at(1) {
            self.at += 1;
            loop {
                if self.kw_in(0, &["CONSTRAINT", "FOREIGN", "CHECK"])
                    || (self.kw("PRIMARY") && self.kw_at(1, "KEY"))
                {
                    node.constraints.push(self.table_constraint()?);
                } else {
                    node.columns.push(self.table_column_definition()?);
                }
                if !self.eat_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
        }
        node = self.table_sources(node)?;
        let (properties, connection) = self.table_clauses(true)?;
        node.properties.extend(properties);
        node.with_connection = connection;
        if self.eat_kw("AS") {
            let parenthesized: bool = self.is_char('(');
            node.as_select = Some(self.query()?);
            node.as_select_parenthesized = parenthesized;
        }
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// `LIKE path`, then a `CLONE` or `COPY` source; a source's `WHERE`, or both, are gaps.
    fn table_sources(&mut self, mut node: CreateTable) -> Parsing<CreateTable> {
        if self.eat_kw("LIKE") {
            let like: TableRef = self.table_path()?;
            node.properties
                .push(Expression::LikeProperty(Box::new(LikeProperty {
                    this: Box::new(Expression::Table(Box::new(like))),
                    expressions: Vec::new(),
                })));
        }
        for keyword in ["CLONE", "COPY"] {
            if !self.kw(keyword) {
                continue;
            }
            let start: usize = self.offset();
            self.at += 1;
            let source: TableRef = self.table_path()?;
            let mut at: Option<Expression> = None;
            if self.eat_kw("FOR") {
                if !self.eat_kws(&["SYSTEM", "TIME"]) {
                    self.expect_kw("SYSTEM_TIME")?;
                }
                self.expect_kws(&["AS", "OF"])?;
                at = Some(clause("FOR SYSTEM_TIME AS OF", vec![self.expression()?]));
            }
            if self.kw("WHERE") {
                let where_at: usize = self.offset();
                self.at += 1;
                self.expression()?;
                self.gap("table_source_where", where_at);
            }
            if node.clone_source.is_some() {
                self.gap("table_clone_and_copy", start);
                continue;
            }
            node.clone_source = Some(source);
            node.clone_at_clause = at;
            node.is_copy = keyword == COPY_SOURCE;
        }
        Ok(node)
    }

    /// `table_constraint_definition`: a primary key, a named foreign key or `CHECK`, then `[[NOT] ENFORCED]`.
    fn table_constraint(&mut self) -> Parsing<TableConstraint> {
        if self.kw("PRIMARY") {
            let columns: Vec<Identifier> = self.primary_key_columns()?;
            let modifiers: ConstraintModifiers = self.constraint_modifiers();
            self.constraint_options()?;
            return Ok(TableConstraint::PrimaryKey {
                name: None,
                columns,
                key_parts: Vec::new(),
                timeseries_columns: Vec::new(),
                include_columns: Vec::new(),
                modifiers,
                has_constraint_keyword: false,
            });
        }
        let name: Option<Identifier> = match self.eat_kw("CONSTRAINT") {
            true => Some(self.ident()?),
            false => None,
        };
        let constraint: TableConstraint = match self.eat_kw("CHECK") {
            true => TableConstraint::Check {
                name,
                expression: self.parenthesized_expression()?,
                modifiers: self.constraint_modifiers(),
            },
            false => {
                self.expect_kws(&["FOREIGN", "KEY"])?;
                let columns: Vec<Identifier> = self.identifier_list()?;
                let references: ForeignKeyRef = self.foreign_key_reference()?;
                TableConstraint::ForeignKey {
                    name,
                    columns,
                    references: Some(Box::new(references)),
                    on_delete: None,
                    on_update: None,
                    modifiers: self.constraint_modifiers(),
                }
            }
        };
        self.constraint_options()?;
        Ok(constraint)
    }

    /// `[NOT] ENFORCED` as a constraint's modifiers.
    fn constraint_modifiers(&mut self) -> ConstraintModifiers {
        ConstraintModifiers {
            enforced: self.enforced(),
            ..ConstraintModifiers::default()
        }
    }

    /// A constraint's `OPTIONS (...)`, read as a gap.
    fn constraint_options(&mut self) -> Parsing<()> {
        if self.kw("OPTIONS") && self.char_at(1, '(') {
            let start: usize = self.offset();
            self.at += 1;
            self.options_list()?;
            self.gap("constraint_options", start);
        }
        Ok(())
    }

    fn create_view(&mut self, head: Head) -> Parsing<Expression> {
        self.expect_kw("VIEW")?;
        let if_not_exists: bool = self.if_not_exists();
        let name: TableRef = self.table_reference()?;
        let mut columns: Vec<ViewColumn> = Vec::new();
        if self.is_char('(') && !self.starts_query_at(1) {
            columns = self.parenthesized(|parser| {
                let mut column: ViewColumn = ViewColumn::new(String::new());
                column.name = parser.ident()?;
                if parser.kw("OPTIONS") && parser.char_at(1, '(') {
                    parser.at += 1;
                    column.options = parser.options_list()?;
                }
                Ok(column)
            })?;
        }
        let (properties, _) = self.table_clauses(false)?;
        self.expect_kw("AS")?;
        if head.materialized && self.kw("REPLICA") && self.kw_at(1, "OF") {
            self.at += 2;
            let mut operands: Vec<Expression> = vec![path_operand(&self.table_path()?)];
            if !properties.is_empty() {
                operands.push(clause("OPTIONS", properties));
            }
            return Ok(Expression::CreateObject(Box::new(CreateObject {
                kind: "MATERIALIZED VIEW".to_owned(),
                modifier: None,
                or_replace: head.or_replace,
                or_alter: false,
                if_not_exists,
                name,
                source: "REPLICA".to_owned(),
                operands,
                properties: Vec::new(),
            })));
        }
        let parenthesized: bool = self.is_char('(');
        let query: Expression = self.query()?;
        let mut node: CreateView = CreateView::new(String::new(), query);
        node.name = name;
        node.columns = columns;
        node.or_replace = head.or_replace;
        node.if_not_exists = if_not_exists;
        node.temporary = head.temporary;
        node.materialized = head.materialized;
        node.options = properties;
        node.query_parenthesized = parenthesized;
        Ok(Expression::CreateView(Box::new(node)))
    }
}
