//! `CreateTableStmt`, `CreateViewStmt` and `DropStatement` (create_table.gram, drop.gram).

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, ComputedColumn, ConstraintModifiers, ForeignKeyRef,
    TableConstraint,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{CreateView, DropView, ViewColumn};
use sqlquack_syntax::expressions::tables::models::{CreateTable, DropObject, DropTable, OnCommit};

use crate::parse::_helpers::commands::options::{RelOption, RelValue};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::joined;
use crate::parse::constants::{
    AUTO_COMPRESSION, COLLATE_TWICE, COLUMN_CONSTRAINT_STARTS, COMPRESSION_AUTO, COMPRESSIONS,
    CTAS_NEEDS_SELECT, DEFAULT_TWICE, DEFER_BINDING, DROP_MATERIALIZED, DROP_ONE, FK_ACTIONS,
    FK_COLUMN_COUNT, GENERATED_ALWAYS, GENERATED_COLLATE, GENERATED_DEFAULT, GENERATED_QUALIFIED,
    NO_COLUMNS, ON_COMMIT_DELETE, PERSISTENT_ONLY_SECRET, RECURSIVE_LIMIT, RECURSIVE_ORDER,
    RECURSIVE_VIEW_COLUMNS, REPLACE_AND_IF_NOT_EXISTS, SECRET_KIND, STORED_GENERATED,
    SUBQUERY_IN_CHECK, TABLE_CONSTRAINT_STARTS, TEMPORARY_INDEX, TEMPORARY_TRIGGER,
    VARCHAR_COLLATION, VARCHAR_NAMES, VIEW_KIND, VIEW_OPTION, VIEW_OPTION_BOOLEAN,
    VIEW_OPTION_CONSTANT,
};
use crate::scan::models::Token;
use crate::scan::models::TokenKind;

/// The kind of a `DROP VIEW` with `CASCADE` or `RESTRICT`, which `DropView` cannot hold.
const DROP_VIEW_KIND: &str = "VIEW";

/// `CREATE [OR REPLACE] [TEMP]`: what comes before the created object's kind.
#[derive(Clone, Copy)]
struct Head {
    or_replace: bool,
    temporary: bool,
}

impl Parser<'_> {
    /// `CreateStatement`: tables, views and macros as trees, every other kind a gap.
    pub(crate) fn create_statement(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let start: usize = self.offset();
        let mut offset: usize = 1;
        let or_replace: bool = self.kw_at(offset, "OR") && self.kw_at(offset + 1, "REPLACE");
        if or_replace {
            offset += 2;
        }
        let persistent: bool = self.kw_at(offset, "PERSISTENT");
        let temporary: bool = self.kw_in(offset, &["TEMP", "TEMPORARY"]);
        if persistent || temporary {
            offset += 1;
        }
        let head: Head = Head {
            or_replace,
            temporary,
        };
        let kind: String = self.word_at(offset).unwrap_or_default();
        let created: Expression = match kind.as_str() {
            "MACRO" | "FUNCTION" => self.create_macro()?,
            "TABLE" => {
                self.at = first + offset + 1;
                self.create_table(head)?
            }
            "VIEW" | "SECURE" | "RECURSIVE" => {
                self.at = first + offset;
                self.create_view(head)?
            }
            "SEQUENCE" => {
                self.at = first + offset + 1;
                self.create_sequence(or_replace, temporary, start)?
            }
            "TYPE" => {
                self.at = first + offset + 1;
                self.create_type(or_replace, temporary, start)?
            }
            "SCHEMA" => {
                self.at = first + offset + 1;
                self.create_schema(or_replace, start)?
            }
            "INDEX" | "UNIQUE" => {
                self.at = first + offset;
                self.create_index(or_replace, start)?
            }
            "TRIGGER" => {
                self.at = first + offset + 1;
                self.create_trigger(or_replace, start)?
            }
            SECRET_KIND => {
                self.at = first + offset + 1;
                self.create_secret(or_replace, start)?
            }
            "EXTENSION" if !persistent && !temporary => {
                self.at = first + offset + 1;
                self.create_extension_repository(or_replace, start)?
            }
            "EXTERNAL" if offset == 1 => {
                self.at = first + 2;
                self.create_external_resource(start)?
            }
            _ => {
                self.at = first + offset;
                return Err(self.error());
            }
        };
        let temporary_refusal: Option<&str> = match kind.as_str() {
            "INDEX" | "UNIQUE" => Some(TEMPORARY_INDEX),
            "TRIGGER" => Some(TEMPORARY_TRIGGER),
            _ => None,
        };
        if let (true, Some(message)) = (temporary, temporary_refusal) {
            self.refuse_later(message, start);
        }
        if persistent && kind != SECRET_KIND {
            self.refuse_later(PERSISTENT_ONLY_SECRET, start);
        }
        Ok(created)
    }

    /// `IF NOT EXISTS`, refused with `OR REPLACE`.
    pub(crate) fn create_if_not_exists(&mut self, or_replace: bool, start: usize) -> Parsing<bool> {
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        if if_not_exists && or_replace {
            self.refuse_later(REPLACE_AND_IF_NOT_EXISTS, start);
        }
        Ok(if_not_exists)
    }

    /// `CreateTableStmt` after `TABLE`: `[IF NOT EXISTS] name {CreateTableAs | CreateColumnList}`.
    fn create_table(&mut self, head: Head) -> Parsing<Expression> {
        let start: usize = self.offset();
        let if_not_exists: bool = self.create_if_not_exists(head.or_replace, start)?;
        let mut node: CreateTable = CreateTable::new(String::new());
        node.name = self.base_table_name()?;
        node.or_replace = head.or_replace;
        node.temporary = head.temporary;
        node.if_not_exists = if_not_exists;
        let after: usize = self.past_parentheses();
        if !self.is_char('(') || self.kw_at(after, "AS") {
            return Ok(Expression::CreateTable(Box::new(
                self.create_table_as(node)?,
            )));
        }
        if !self.kw_in(after, &["PARTITIONED", "SORTED", "WITH"]) {
            return Ok(Expression::CreateTable(Box::new(self.column_list(node)?)));
        }
        let template: CreateTable = node.clone();
        let created: CreateTable = match self.attempt(|parser| parser.create_table_as(template)) {
            Ok(created) => created,
            Err((error, _)) if !self.is_char('(') => return Err(error),
            Err((as_error, as_reached)) => match self.attempt(|parser| parser.column_list(node)) {
                Ok(created) => created,
                Err((list_error, list_reached)) => {
                    return Err(match as_reached > list_reached {
                        true => as_error,
                        false => list_error,
                    });
                }
            },
        };
        Ok(Expression::CreateTable(Box::new(created)))
    }

    /// How many tokens ahead the token after the parenthesised group at the cursor is.
    fn past_parentheses(&self) -> usize {
        let mut depth: usize = 0;
        for (offset, token) in self.tokens[self.at..].iter().enumerate() {
            match token.kind {
                TokenKind::Char('(') => depth += 1,
                TokenKind::Char(')') if depth <= 1 => return offset + 1,
                TokenKind::Char(')') => depth -= 1,
                _ => {}
            }
        }
        self.tokens.len() - self.at
    }

    /// `CreateTableAs`: `[(names)] [options] AS query [WITH [NO] DATA]`.
    fn create_table_as(&mut self, mut node: CreateTable) -> Parsing<CreateTable> {
        if self.is_char('(') {
            let names: Vec<Identifier> = self.parenthesized(|parser| parser.name(is_identifier))?;
            node.columns = names
                .into_iter()
                .map(|name| {
                    let mut column: ColumnDef = ColumnDef::new(String::new(), DataType::Unknown);
                    column.name = name;
                    column
                })
                .collect();
        }
        node.with_properties = properties(self.table_options()?);
        let start: usize = self.offset();
        self.expect_kw("AS")?;
        if !self.starts_query_at(0) {
            self.statement()?;
            return Err(self.error_with(CTAS_NEEDS_SELECT, start, false));
        }
        node.as_select_parenthesized = self.is_char('(');
        node.as_select = Some(self.query()?);
        if self.eat_kws(&["WITH", "DATA"]) {
            node.with_data = Some(true);
        } else if self.eat_kws(&["WITH", "NO", "DATA"]) {
            node.with_data = Some(false);
        }
        node.on_commit = self.commit_action()?;
        Ok(node)
    }

    /// `CreateColumnList`: `(columns and constraints) [options] [ON COMMIT ...]`.
    fn column_list(&mut self, mut node: CreateTable) -> Parsing<CreateTable> {
        let start: usize = self.offset();
        self.expect_char('(')?;
        if !self.is_char(')') {
            loop {
                match self.kw_in(0, &TABLE_CONSTRAINT_STARTS) {
                    true => node.constraints.push(self.table_constraint()?),
                    false => node.columns.push(self.column_definition()?),
                }
                if !self.eat_char(',') || self.is_char(')') {
                    break;
                }
            }
        }
        self.expect_char(')')?;
        node.with_properties = properties(self.table_options()?);
        node.on_commit = self.commit_action()?;
        if node.columns.is_empty() {
            self.refuse_later(NO_COLUMNS, start);
        }
        Ok(node)
    }

    /// `PartitionSortedOptions`, as a gap, and `WithList`, whose options are returned.
    pub(crate) fn table_options(&mut self) -> Parsing<Vec<RelOption>> {
        let start: usize = self.offset();
        let mut found: bool = false;
        for _ in 0..2 {
            if self.kw_in(0, &["PARTITIONED", "SORTED"]) && self.kw_at(1, "BY") {
                self.at += 2;
                self.expect_char('(')?;
                self.expression_list(')')?;
                self.expect_char(')')?;
                found = true;
            }
        }
        if found {
            self.gap("table_layout", start);
        }
        let mut options: Vec<RelOption> = Vec::new();
        if self.kw("WITH") && (self.char_at(1, '(') || self.kw_in(1, &["WITH", "WITHOUT"])) {
            options = self.with_list()?;
        }
        Ok(options)
    }

    /// `CommitAction`: `ON COMMIT PRESERVE ROWS`; `DELETE ROWS` is not implemented.
    fn commit_action(&mut self) -> Parsing<Option<OnCommit>> {
        if !(self.kw("ON") && self.kw_at(1, "COMMIT")) {
            return Ok(None);
        }
        let start: usize = self.offset();
        self.at += 2;
        let preserve: bool = self.eat_kw("PRESERVE");
        if !preserve {
            self.expect_kw("DELETE")?;
        }
        self.expect_kw("ROWS")?;
        match preserve {
            true => Ok(Some(OnCommit::PreserveRows)),
            false => Err(self.error_with(ON_COMMIT_DELETE, start, false)),
        }
    }

    /// `ColumnDefinition`: `name [type] [generated] constraint...`.
    fn column_definition(&mut self) -> Parsing<ColumnDef> {
        let start: usize = self.offset();
        let name: Identifier = self.name(is_identifier)?;
        let typed: bool = !(self.terminal_at(0)
            || self.is_char(',')
            || self.is_char(')')
            || self.kw_in(0, &COLUMN_CONSTRAINT_STARTS));
        let data_type: DataType = match typed {
            true => self.data_type()?,
            false => DataType::Unknown,
        };
        let name_text: String = name.name.clone();
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        let generated: Option<ColumnConstraint> = self.generated_column(&name_text, start)?;
        let typed_or_generated: bool = typed || generated.is_some();
        column.constraints.extend(generated);
        if !typed_or_generated {
            let message: String = format!(
                "Column {} must have a type or be defined as a GENERATED column.",
                name_text
            );
            self.refuse_later(&message, start);
        }
        let column: ColumnDef = self.column_constraints(column)?;
        let generated: bool = column
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, ColumnConstraint::ComputedColumn(_)));
        let collations: usize = column
            .constraints
            .iter()
            .filter(|constraint| matches!(constraint, ColumnConstraint::Collate(_)))
            .count();
        let refusal: Option<&str> = match (collations, generated) {
            _ if column.encoding.as_deref() == Some(AUTO_COMPRESSION) => Some(COMPRESSION_AUTO),
            (2.., _) => Some(COLLATE_TWICE),
            (1, true) => Some(GENERATED_COLLATE),
            (1, false) if !is_varchar(&column.data_type) => Some(VARCHAR_COLLATION),
            (_, true) if column.default.is_some() => Some(GENERATED_DEFAULT),
            _ => None,
        };
        if let Some(message) = refusal {
            self.refuse_later(message, start);
        }
        Ok(column)
    }

    /// `GeneratedColumn`: `[GENERATED {ALWAYS | BY DEFAULT}] AS (expression) [VIRTUAL | STORED]`.
    pub(crate) fn generated_column(
        &mut self,
        name: &str,
        start: usize,
    ) -> Parsing<Option<ColumnConstraint>> {
        let mut by_default: bool = false;
        if self.eat_kw("GENERATED") {
            if !self.eat_kw("ALWAYS") {
                self.expect_kw("BY")?;
                self.expect_kw("DEFAULT")?;
                by_default = true;
            }
            self.expect_kw("AS")?;
        } else if !self.eat_kw("AS") {
            return Ok(None);
        }
        self.expect_char('(')?;
        let (first, subqueries) = (self.at, self.subqueries);
        let expression: Expression = self.expression()?;
        let qualified: bool = self.has_qualified_column(first);
        let subquery: bool = self.subqueries > subqueries;
        self.expect_char(')')?;
        let refusal: Option<&str> = match (by_default, qualified, self.kw("STORED")) {
            (true, _, _) => Some(GENERATED_ALWAYS),
            (_, true, _) => Some(GENERATED_QUALIFIED),
            (_, _, true) => Some(STORED_GENERATED),
            _ => None,
        };
        if let Some(message) = refusal {
            self.refuse_later(message, start);
        }
        if subquery {
            let message: String = format!(
                "Expression of generated column \"{name}\" contains a subquery, which isn't allowed"
            );
            self.refuse_later(&message, start);
        }
        let virtual_kind: bool = self.eat_kw("VIRTUAL");
        Ok(Some(ColumnConstraint::ComputedColumn(ComputedColumn {
            expression: Box::new(expression),
            persisted: false,
            not_null: false,
            persistence_kind: virtual_kind.then(|| "VIRTUAL".to_owned()),
            data_type: None,
        })))
    }

    /// `ColumnConstraint*`: nullability, keys, defaults, checks, references, collation.
    pub(crate) fn column_constraints(&mut self, mut column: ColumnDef) -> Parsing<ColumnDef> {
        let start: usize = self.offset();
        loop {
            let named: bool = self.eat_kw("CONSTRAINT");
            if named {
                self.col_id_or_string()?;
            }
            if self.eat_kw("NULL") {
                column.nullable = Some(true);
                column.constraints.push(ColumnConstraint::Null);
            } else if self.eat_kws(&["NOT", "NULL"]) {
                column.nullable = Some(false);
                column.constraints.push(ColumnConstraint::NotNull);
            } else if self.eat_kw("UNIQUE") {
                column.unique = true;
            } else if self.eat_kws(&["PRIMARY", "KEY"]) {
                column.primary_key = true;
            } else if self.eat_kw("DEFAULT") {
                let default: Expression = self.column_default()?;
                if column.default.replace(default).is_some() {
                    self.refuse_later(DEFAULT_TWICE, start);
                }
            } else if self.kw("CHECK") {
                let condition: Expression = self.check_condition()?;
                column.constraints.push(ColumnConstraint::Check(condition));
            } else if self.kw("REFERENCES") {
                let reference: ForeignKeyRef = self.reference()?;
                column
                    .constraints
                    .push(ColumnConstraint::References(Box::new(reference)));
            } else if self.kw("USING") && self.kw_at(1, "COMPRESSION") {
                let start: usize = self.offset();
                self.at += 2;
                let method: String = self.col_id_or_string()?.name.to_ascii_lowercase();
                if method != AUTO_COMPRESSION && !COMPRESSIONS.contains(&method.as_str()) {
                    let message: String = format!(
                        "Enum value: unrecognized value \"{method}\" for enum \"CompressionType\""
                    );
                    self.refuse_later(&message, start);
                }
                self.gap("column_compression", start);
                column.encoding = Some(method);
            } else if !named && self.eat_kw("COLLATE") {
                let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
                while self.eat_char('.') {
                    parts.push(self.name(is_col_label)?);
                }
                column
                    .constraints
                    .push(ColumnConstraint::Collate(joined(parts)));
            } else if named {
                return Err(self.error());
            } else {
                return Ok(column);
            }
        }
    }

    /// Whether a `name.name` not followed by `(` was read since token `first`: a qualified column.
    fn has_qualified_column(&self, first: usize) -> bool {
        let is_name = |token: &Token| matches!(token.kind, TokenKind::Word | TokenKind::Quoted);
        let read: &[Token] = &self.tokens[first..self.at];
        read.windows(3).enumerate().any(|(index, window)| {
            let after_dot: bool = index > 0 && read[index - 1].kind == TokenKind::Char('.');
            let call_follows: bool = read
                .get(index + 3)
                .is_some_and(|token| token.kind == TokenKind::Char('('));
            is_name(&window[0])
                && window[1].kind == TokenKind::Char('.')
                && is_name(&window[2])
                && !after_dot
                && !call_follows
        })
    }

    /// `CheckConstraint`: `CHECK (expression)`, which may hold no subquery.
    fn check_condition(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("CHECK")?;
        self.expect_char('(')?;
        let subqueries: usize = self.subqueries;
        let condition: Expression = self.expression()?;
        self.expect_char(')')?;
        if self.subqueries > subqueries {
            self.refuse_later(SUBQUERY_IN_CHECK, start);
        }
        Ok(condition)
    }

    /// `ForeignKeyConstraint`: `REFERENCES table [(columns)] [ON UPDATE|DELETE action ...]`.
    fn reference(&mut self) -> Parsing<ForeignKeyRef> {
        let start: usize = self.offset();
        self.expect_kw("REFERENCES")?;
        let table: TableRef = self.base_table_name()?;
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.name(is_identifier))?,
            false => Vec::new(),
        };
        let mut refused: bool = false;
        let mut seen: Vec<String> = Vec::new();
        while self.kw("ON") && self.kw_in(1, &["UPDATE", "DELETE"]) {
            let event: String = self.word_at(1).unwrap_or_default();
            if seen.contains(&event) {
                break;
            }
            seen.push(event);
            self.at += 2;
            if self.eat_kws(&["NO", "ACTION"]) || self.eat_kw("RESTRICT") {
                continue;
            }
            refused = true;
            if !self.eat_kw("CASCADE") {
                self.expect_kw("SET")?;
                if !(self.eat_kw("NULL") || self.eat_kw("DEFAULT")) {
                    return Err(self.error());
                }
            }
        }
        if refused {
            self.refuse_later(FK_ACTIONS, start);
        }
        Ok(ForeignKeyRef {
            table,
            columns,
            on_delete: None,
            on_update: None,
            on_update_first: false,
            match_type: None,
            match_after_actions: false,
            constraint_name: None,
            deferrable: None,
            has_foreign_key_keywords: false,
        })
    }

    /// `TopLevelConstraint`: `[CONSTRAINT name] {PRIMARY KEY | UNIQUE} (columns)`, a check or a key.
    pub(crate) fn table_constraint(&mut self) -> Parsing<TableConstraint> {
        let has_constraint_keyword: bool = self.eat_kw("CONSTRAINT");
        let name: Option<Identifier> = match has_constraint_keyword {
            true => Some(self.col_id_or_string()?),
            false => None,
        };
        let modifiers: ConstraintModifiers = ConstraintModifiers::default();
        if self.kw("CHECK") {
            let expression: Expression = self.check_condition()?;
            return Ok(TableConstraint::Check {
                name,
                expression,
                modifiers,
            });
        }
        if self.eat_kws(&["FOREIGN", "KEY"]) {
            let columns: Vec<Identifier> =
                self.parenthesized(|parser| parser.name(is_identifier))?;
            let start: usize = self.offset();
            let mut reference: ForeignKeyRef = self.reference()?;
            reference.has_foreign_key_keywords = true;
            if !reference.columns.is_empty() && reference.columns.len() != columns.len() {
                self.refuse_later(FK_COLUMN_COUNT, start);
            }
            return Ok(TableConstraint::ForeignKey {
                name,
                columns,
                references: Some(Box::new(reference)),
                on_delete: None,
                on_update: None,
                modifiers,
            });
        }
        let primary: bool = self.eat_kws(&["PRIMARY", "KEY"]);
        if !primary {
            self.expect_kw("UNIQUE")?;
        }
        let columns: Vec<Identifier> = self.parenthesized(|parser| parser.name(is_identifier))?;
        Ok(match primary {
            true => TableConstraint::PrimaryKey {
                name,
                columns,
                key_parts: Vec::new(),
                timeseries_columns: Vec::new(),
                include_columns: Vec::new(),
                modifiers,
                has_constraint_keyword,
            },
            false => TableConstraint::Unique {
                name,
                columns,
                key_parts: Vec::new(),
                columns_parenthesized: true,
                modifiers,
                has_constraint_keyword,
                nulls_not_distinct: false,
            },
        })
    }

    /// `CreateViewStmt`: `[SECURE] [RECURSIVE] VIEW [IF NOT EXISTS] name [(names)] AS query`.
    fn create_view(&mut self, head: Head) -> Parsing<Expression> {
        let start: usize = self.offset();
        let secure: bool = self.eat_kw("SECURE");
        let recursive: bool = self.eat_kw("RECURSIVE");
        self.expect_kw("VIEW")?;
        let if_not_exists: bool = self.create_if_not_exists(head.or_replace, start)?;
        let name: TableRef = self.base_table_name()?;
        let columns: Vec<ViewColumn> = match self.is_char('(') {
            true => self
                .parenthesized(|parser| parser.name(is_identifier))?
                .into_iter()
                .map(|column| {
                    let mut view_column: ViewColumn = ViewColumn::new(String::new());
                    view_column.name = column;
                    view_column
                })
                .collect(),
            false => Vec::new(),
        };
        let options_start: usize = self.offset();
        let options: Vec<RelOption> = self.table_options()?;
        if !options.is_empty() {
            self.gap("view_options", options_start);
        }
        for option in &options {
            let refusal: Option<&str> = match option.value {
                _ if !option.name.eq_ignore_ascii_case(DEFER_BINDING) => Some(VIEW_OPTION),
                RelValue::NonConstant => Some(VIEW_OPTION_CONSTANT),
                RelValue::Other => Some(VIEW_OPTION_BOOLEAN),
                RelValue::Absent | RelValue::Null | RelValue::Boolean => None,
            };
            if let Some(message) = refusal {
                self.refuse_later(message, start);
            }
        }
        self.expect_kw("AS")?;
        let parenthesized: bool = self.is_char('(');
        let pivots: usize = self.pivot_entries;
        let query: Expression = self.query()?;
        self.pivot_entry_check(pivots, VIEW_KIND, start);
        if recursive && columns.is_empty() {
            self.refuse_later(RECURSIVE_VIEW_COLUMNS, start);
        }
        if let (true, Expression::Union(union)) = (recursive, &query)
            && !union.by_name
        {
            let refusal: Option<&str> = match (&union.order_by, &union.limit, &union.offset) {
                (Some(_), _, _) => Some(RECURSIVE_ORDER),
                (_, Some(_), _) | (_, _, Some(_)) => Some(RECURSIVE_LIMIT),
                _ => None,
            };
            if let Some(message) = refusal {
                self.refuse_later(message, start);
            }
        }
        if recursive {
            self.gap("recursive_view", start);
        }
        let mut node: CreateView = CreateView::new(String::new(), query);
        node.name = name;
        node.columns = columns;
        node.or_replace = head.or_replace;
        node.if_not_exists = if_not_exists;
        node.temporary = head.temporary;
        node.secure = secure;
        node.query_parenthesized = parenthesized;
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// `DropStatement`: tables and views as trees, every other kind a gap.
    pub(crate) fn drop_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.at += 1;
        let materialized: bool = self.kw("MATERIALIZED") && self.kw_at(1, "VIEW");
        let view: bool = materialized || self.kw("VIEW");
        if !(view || self.kw("TABLE")) {
            return self.drop_object(start);
        }
        self.at += 1 + usize::from(materialized);
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let mut names: Vec<TableRef> = vec![self.base_table_name()?];
        while self.eat_char(',') {
            if self.terminal_at(0) || self.kw_in(0, &["CASCADE", "RESTRICT"]) {
                break;
            }
            names.push(self.base_table_name()?);
        }
        let cascade: bool = self.eat_kw("CASCADE");
        let restrict: bool = !cascade && self.eat_kw("RESTRICT");
        if materialized {
            self.refuse_later(DROP_MATERIALIZED, start);
        }
        if names.len() > 1 {
            self.refuse_later(DROP_ONE, start);
        }
        if !view {
            let mut node: DropTable = DropTable::new(String::new());
            node.names = names;
            node.if_exists = if_exists;
            node.cascade = cascade;
            node.restrict = restrict;
            return Ok(Expression::DropTable(Box::new(node)));
        }
        if cascade || restrict {
            return Ok(Expression::DropObject(Box::new(DropObject {
                kind: DROP_VIEW_KIND.to_owned(),
                name: names.remove(0),
                if_exists,
                cascade,
                restrict,
                force: false,
                objects: Vec::new(),
                concurrently: false,
            })));
        }
        let mut node: DropView = DropView::new(String::new());
        node.name = names.remove(0);
        node.if_exists = if_exists;
        Ok(Expression::DropView(Box::new(node)))
    }
}

/// Whether `data_type` names `VARCHAR` or an alias of it, the only type a collation applies to.
pub(crate) fn is_varchar(data_type: &DataType) -> bool {
    match data_type {
        DataType::VarChar { .. }
        | DataType::Char { .. }
        | DataType::Text
        | DataType::String { .. } => true,
        DataType::Custom { name } => {
            let last: &str = name.rsplit('.').next().unwrap_or_default();
            VARCHAR_NAMES
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(last))
        }
        _ => false,
    }
}

/// A table's `WITH` options as the tree keeps them: each name with its value as written.
fn properties(options: Vec<RelOption>) -> Vec<(String, String)> {
    options
        .into_iter()
        .map(|option| (option.name, option.text))
        .collect()
}
