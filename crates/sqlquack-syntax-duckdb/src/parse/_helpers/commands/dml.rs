//! `InsertStatement`, `UpdateStatement` and `DeleteStatement` (insert.gram, update.gram, delete.gram).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::OnConflict;
use sqlquack_syntax::expressions::queries::models::{Select, Where, With};
use sqlquack_syntax::expressions::statements::models::{Delete, Insert, Truncate, Update};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    binary, delete, insert, joined, joined_table, tuple, update,
};
use crate::parse::constants::{INSERT_OR_ACTIONS, QUALIFIED_SET, TARGET_ALIAS_STOPS};

/// The transformer's refusal of `OR REPLACE` or `OR IGNORE` with an `ON CONFLICT` clause.
const OR_ACTION_AND_ON_CONFLICT: &str = "You can not provide both OR REPLACE|IGNORE and an ON CONFLICT clause, please remove the first if you want to have more granular control";
/// The transformer's refusal of a column list before `DEFAULT VALUES`.
const COLUMNS_AND_DEFAULT_VALUES: &str =
    "You can not provide both a column list and DEFAULT VALUES, please remove one";

/// `DO NOTHING`, as the action of an `OnConflict` node.
const NOTHING: &str = "NOTHING";
/// The function a parenthesised row or `ROW(...)` calls.
const ROW_FUNCTION: &str = "row";
/// The transformer's refusal of `TRUNCATE ... CASCADE`.
const TRUNCATE_CASCADE: &str = "TRUNCATE ... CASCADE is not implemented";

/// The items of a row, or of any call whose last name part is `row`, which `SET (columns) =` assigns in order.
fn row_items(value: &Expression) -> Option<&[Expression]> {
    match value {
        Expression::Paren(paren) => row_items(&paren.this),
        Expression::Tuple(row) => Some(&row.expressions),
        Expression::Function(call) => {
            let name: &str = call
                .qualified_name
                .last()
                .map_or(call.name.as_str(), |part| part.name.as_str());
            name.eq_ignore_ascii_case(ROW_FUNCTION)
                .then_some(call.args.as_slice())
        }
        _ => None,
    }
}

impl Parser<'_> {
    /// `InsertStatement`: `INSERT [OR action] INTO target [(columns)] values [ON CONFLICT ...]`.
    pub(crate) fn insert_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("INSERT")?;
        let action: Option<String> = match self.kw("OR") {
            true => {
                self.at += 1;
                let word: String = self.word_at(0).unwrap_or_default();
                if !INSERT_OR_ACTIONS.contains(&word.as_str()) {
                    return Err(self.error());
                }
                self.at += 1;
                Some(word)
            }
            false => None,
        };
        self.expect_kw("INTO")?;
        let mut statement: Insert = insert(self.base_table_name()?);
        statement.with = with;
        statement.conflict_action = action;
        if self.eat_kw("AS") {
            statement.alias = Some(self.name(is_identifier)?);
            statement.alias_explicit_as = true;
        }
        if self.kw("BY") && self.kw_in(1, &["NAME", "POSITION"]) {
            statement.by_name = self.kw_at(1, "NAME");
            self.at += 2;
        }
        if self.is_char('(') && !self.starts_query_at(1) {
            statement.columns = self.parenthesized(|parser| parser.name(is_identifier))?;
        }
        if self.eat_kw("OVERRIDING") {
            let Some(kind) = self
                .word_at(0)
                .filter(|word| matches!(word.as_str(), "USER" | "SYSTEM"))
            else {
                return Err(self.error());
            };
            self.at += 1;
            self.expect_kw("VALUE")?;
            statement.overriding = Some(kind);
        }
        if self.kw("DEFAULT") && self.kw_at(1, "VALUES") {
            if !statement.columns.is_empty() {
                self.refuse_later(COLUMNS_AND_DEFAULT_VALUES, self.offset());
            }
            self.at += 2;
            statement.default_values = true;
        } else {
            statement.query = Some(self.query()?);
        }
        if self.kw("ON") && self.kw_at(1, "CONFLICT") {
            let start: usize = self.offset();
            statement.on_conflict = Some(Box::new(self.on_conflict()?));
            if statement.conflict_action.is_some() {
                self.refuse_later(OR_ACTION_AND_ON_CONFLICT, start);
            }
        }
        statement.returning = self.returning()?;
        Ok(Expression::Insert(Box::new(statement)))
    }

    /// `OnConflictClause`: `ON CONFLICT [(columns) [WHERE ...] | ON CONSTRAINT name] action`.
    fn on_conflict(&mut self) -> Parsing<Expression> {
        self.at += 2;
        let mut node: OnConflict = OnConflict {
            duplicate: None,
            expressions: Vec::new(),
            action: None,
            conflict_keys: None,
            index_predicate: None,
            constraint: None,
            where_: None,
        };
        if self.is_char('(') {
            let keys: Vec<Identifier> = self.parenthesized(|parser| parser.name(is_identifier))?;
            node.conflict_keys = Some(Box::new(tuple(
                keys.into_iter().map(Expression::Identifier).collect(),
            )));
            if self.eat_kw("WHERE") {
                node.index_predicate = Some(Box::new(self.expression()?));
            }
        } else if self.eat_kws(&["ON", "CONSTRAINT"]) {
            node.constraint = Some(Box::new(Expression::Identifier(self.col_id_or_string()?)));
        }
        self.expect_kw("DO")?;
        if self.eat_kw("NOTHING") {
            node.action = Some(Box::new(Expression::Identifier(Identifier::new(NOTHING))));
            return Ok(Expression::OnConflict(Box::new(node)));
        }
        self.expect_kw("UPDATE")?;
        self.expect_kw("SET")?;
        let assignments: Vec<Expression> = self
            .set_clauses()?
            .into_iter()
            .map(|(target, value)| binary(Expression::Eq, Expression::Identifier(target), value))
            .collect();
        node.action = Some(Box::new(tuple(assignments)));
        if self.eat_kw("WHERE") {
            node.where_ = Some(Box::new(self.expression()?));
        }
        Ok(Expression::OnConflict(Box::new(node)))
    }

    /// `UpdateSetClauseList` as column and value pairs, a `(columns) = value` tuple split as DuckDB's transformer splits it.
    pub(crate) fn set_clauses(&mut self) -> Parsing<Vec<(Identifier, Expression)>> {
        let clauses: Vec<Vec<(Identifier, Expression)>> = self.comma_list(|parser| {
            let start: usize = parser.offset();
            if parser.is_char('(') {
                let columns: Vec<Identifier> =
                    parser.parenthesized(|inner| inner.name(is_identifier))?;
                parser.expect_char('=')?;
                let value: Expression = parser.expression()?;
                let values: Vec<Expression> = match row_items(&value) {
                    Some(items) => {
                        if items.len() != columns.len() {
                            let message: String = format!(
                                "Could not perform assignment, expected {} values, got {}",
                                columns.len(),
                                items.len()
                            );
                            parser.refuse_later(&message, start);
                        }
                        items.to_vec()
                    }
                    None => vec![value; columns.len()],
                };
                return Ok(columns.into_iter().zip(values).collect());
            }
            let mut parts: Vec<Identifier> = vec![parser.name(is_identifier)?];
            while parser.eat_char('.') {
                parts.push(parser.name(is_identifier)?);
            }
            parser.expect_char('=')?;
            let value: Expression = parser.expression()?;
            if parts.len() > 1 {
                parser.refuse_later(QUALIFIED_SET, start);
            }
            Ok(vec![(joined(parts), value)])
        })?;
        Ok(clauses.into_iter().flatten().collect())
    }

    /// `ReturningClause`: `RETURNING targets`, or none.
    pub(crate) fn returning(&mut self) -> Parsing<Vec<Expression>> {
        match self.eat_kw("RETURNING") {
            true => self.comma_list(|parser| parser.named_expression()),
            false => Ok(Vec::new()),
        }
    }

    /// `[AS] name` after a DML target, unless the next word starts the statement's next clause.
    pub(crate) fn target_alias(&mut self) -> Parsing<Option<(Identifier, bool)>> {
        let explicit: bool = self.eat_kw("AS");
        let stops: bool = self.kw_in(0, &TARGET_ALIAS_STOPS);
        if !explicit && (stops || !self.is(is_identifier)) {
            return Ok(None);
        }
        Ok(Some((self.name(is_identifier)?, explicit)))
    }

    /// `TruncateStatement`: `TRUNCATE [TABLE] name [[AS] alias] [CASCADE | RESTRICT]`.
    pub(crate) fn truncate_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("TRUNCATE")?;
        let table_word: bool = self.kw("TABLE") && !self.ends_at(1);
        self.at += usize::from(table_word);
        let mut target: TableRef = self.base_table_name()?;
        let behaviour: bool = self.kw_in(0, &["CASCADE", "RESTRICT"]);
        if !behaviour && (self.kw("AS") || self.is(is_identifier)) {
            let explicit: bool = self.eat_kw("AS");
            target.alias = Some(self.name(is_identifier)?);
            target.alias_explicit_as = explicit;
        }
        let cascade: bool = self.eat_kw("CASCADE");
        let restrict: bool = !cascade && self.eat_kw("RESTRICT");
        if cascade {
            self.refuse_later(TRUNCATE_CASCADE, start);
        }
        let mut node: Truncate = Truncate::new(String::new());
        node.table = target;
        node.cascade = cascade;
        node.restrict = restrict;
        Ok(Expression::Truncate(Box::new(node)))
    }

    /// `WhereClause`: `WHERE condition`, or none.
    fn dml_where(&mut self) -> Parsing<Option<Where>> {
        match self.eat_kw("WHERE") {
            true => Ok(Some(Where {
                this: self.expression()?,
            })),
            false => Ok(None),
        }
    }

    /// `UpdateStatement`: `UPDATE target [[AS] alias] SET ... [FROM ...] [WHERE ...] [RETURNING ...]`.
    pub(crate) fn update_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("UPDATE")?;
        let mut target: TableRef = self.base_table_name()?;
        if !self.kw("SET") {
            let Some((name, explicit)) = self.target_alias()? else {
                return Err(self.error());
            };
            target.alias = Some(name);
            target.alias_explicit_as = explicit;
        }
        self.expect_kw("SET")?;
        let mut statement: Update = update(target);
        statement.with = with;
        statement.set = self.set_clauses()?;
        if self.kw("FROM") {
            let source: Select = self.select_from_clause(Select::new())?;
            statement.from_clause = source.from;
            statement.from_joins = source.joins;
        }
        statement.where_clause = self.dml_where()?;
        statement.returning = self.returning()?;
        Ok(Expression::Update(Box::new(statement)))
    }

    /// `DeleteStatement`: `DELETE FROM target [[AS] alias] [USING tables] [WHERE ...] [RETURNING ...]`.
    pub(crate) fn delete_statement(&mut self, with: Option<With>) -> Parsing<Expression> {
        self.expect_kw("DELETE")?;
        self.expect_kw("FROM")?;
        let mut statement: Delete = delete(self.base_table_name()?);
        statement.with = with;
        if let Some((name, explicit)) = self.target_alias()? {
            statement.alias = Some(name);
            statement.alias_explicit_as = explicit;
        }
        if self.eat_kw("USING") {
            let sources: Vec<Expression> = self.comma_list(|parser| {
                let (source, joins) = parser.table_ref()?;
                Ok(match joins.is_empty() {
                    true => source,
                    false => joined_table(source, joins, None),
                })
            })?;
            let tables: Option<Vec<TableRef>> = sources
                .iter()
                .map(|source| match source {
                    Expression::Table(table) => Some((**table).clone()),
                    _ => None,
                })
                .collect();
            match tables {
                Some(tables) => statement.using = tables,
                None => statement.using_items = sources,
            }
        }
        statement.where_clause = self.dml_where()?;
        statement.returning = self.returning()?;
        Ok(Expression::Delete(Box::new(statement)))
    }
}
