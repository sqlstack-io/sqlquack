//! `from_clause` (gram.y): table references, joins, `LATERAL`, function tables and aliases.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{Join, JoinKind};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, join, joined_table, lateral, subquery, table,
};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::parse::constants::VALUE_FUNCTIONS;

/// `call` under its alias, when it has one.
fn aliased(call: Expression, alias: Option<TableAlias>) -> Expression {
    match alias {
        None => call,
        Some(alias) => {
            let mut node: Alias = Alias::new(call, alias.name);
            node.column_aliases = alias.columns;
            node.alias_explicit_as = alias.explicit;
            node.alias_keyword = alias.explicit.then(|| "AS".to_owned());
            Expression::Alias(Box::new(node))
        }
    }
}

impl Parser<'_> {
    /// `from_list`: the comma-separated items, and every join, in order.
    pub(crate) fn parse_from_list(&mut self) -> Parsing<(Vec<Expression>, Vec<Join>)> {
        let mut items: Vec<Expression> = Vec::new();
        let mut joins: Vec<Join> = Vec::new();
        loop {
            items.push(self.table_ref()?);
            joins.extend(self.joins()?);
            if !self.eat_char(',') {
                return Ok((items, joins));
            }
        }
    }

    /// Whether a join begins at the cursor.
    fn at_join(&self) -> bool {
        match self.keyword_at(0) {
            Some("join" | "cross" | "natural") => true,
            Some("inner") => self.kw_at(1, "join"),
            Some("left" | "right" | "full") => self.kw_at(1, "join") || self.kw_at(1, "outer"),
            _ => false,
        }
    }

    /// The joins after a table reference, left to right.
    pub(crate) fn joins(&mut self) -> Parsing<Vec<Join>> {
        let mut joins: Vec<Join> = Vec::new();
        while self.at_join() {
            let first: usize = self.at;
            let mut node: Join = self.deeper(|parser| parser.one_join())?;
            node.span = self.source_range(first);
            joins.push(node);
        }
        Ok(joins)
    }

    fn one_join(&mut self) -> Parsing<Join> {
        if self.eat_kw("cross") {
            self.expect_kw("join")?;
            return Ok(join(
                self.table_ref()?,
                JoinKind::Cross,
                JoinConstraint::None,
                (false, false),
            ));
        }
        let natural: bool = self.eat_kw("natural");
        let (mut inner, mut outer) = (false, false);
        let side: Option<&str> = match self.keyword_at(0) {
            Some(word @ ("left" | "right" | "full")) => {
                self.at += 1;
                outer = self.eat_kw("outer");
                Some(word)
            }
            Some("inner") => {
                self.at += 1;
                inner = true;
                None
            }
            _ => None,
        };
        self.expect_kw("join")?;
        let kind: JoinKind = match (natural, side) {
            (true, Some("left")) => JoinKind::NaturalLeft,
            (true, Some("right")) => JoinKind::NaturalRight,
            (true, Some("full")) => JoinKind::NaturalFull,
            (true, _) => JoinKind::Natural,
            (false, Some("left")) => JoinKind::Left,
            (false, Some("right")) => JoinKind::Right,
            (false, Some("full")) => JoinKind::Full,
            (false, _) => JoinKind::Inner,
        };
        let mut right: Expression = self.table_ref()?;
        if natural {
            return Ok(join(right, kind, JoinConstraint::None, (inner, outer)));
        }
        if self.at_join() {
            let nested: Vec<Join> = self.joins()?;
            right = joined_table(right, nested, None);
        }
        let start: usize = self.offset();
        if self.eat_kw("on") {
            let on: Expression = self.a_expr()?;
            return Ok(join(
                right,
                kind,
                JoinConstraint::On(Box::new(on)),
                (inner, outer),
            ));
        }
        self.expect_kw("using")?;
        let using: Vec<Identifier> = self.name_list()?;
        if self.eat_kw("as") {
            self.name(is_col_id)?;
            self.gap("join_using_alias", start);
        }
        Ok(join(
            right,
            kind,
            JoinConstraint::Using(using),
            (inner, outer),
        ))
    }

    /// `ROWS FROM (...)` with its alias, as a gap, or `XMLTABLE (...)` or `JSON_TABLE (...)` with its alias.
    fn gap_table(&mut self, start: usize, is_lateral: bool) -> Parsing<Option<Expression>> {
        match self.keyword_at(0) {
            Some("rows") if self.kw_at(1, "from") && self.char_at(2, '(') => {
                self.at += 2;
                self.skip_parenthesized()?;
                self.ordinality();
                self.function_alias()?;
                self.gap("rows_from", start);
                Ok(Some(Expression::Table(Box::new(TableRef::new(
                    "rows_from",
                )))))
            }
            Some(word @ ("xmltable" | "json_table")) if self.char_at(1, '(') => {
                self.at += 1;
                let call: Expression = match word {
                    "xmltable" => self.xml_table()?,
                    _ => self.json_table()?,
                };
                let alias: Option<TableAlias> = self.alias_clause(true)?;
                Ok(Some(match is_lateral {
                    true => {
                        let (name, columns) = match alias {
                            Some(alias) => (Some(alias.name), alias.columns),
                            None => (None, Vec::new()),
                        };
                        lateral(call, name, columns)
                    }
                    false => aliased(call, alias),
                }))
            }
            _ => Ok(None),
        }
    }

    /// `table_ref` without its joins, with its source range.
    pub(crate) fn table_ref(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let node: Expression = self.table_item()?;
        Ok(self.spanned(first, node))
    }

    /// `table_ref` without its joins.
    fn table_item(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        if self.eat_kw("lateral") {
            if self.is_char('(') {
                let query: Expression = self.select_with_parens()?;
                let alias: Option<TableAlias> = self.alias_clause(true)?;
                return Ok(self.subquery_ref(query, alias, true));
            }
            if let Some(gap) = self.gap_table(start, true)? {
                return Ok(gap);
            }
            return self.function_table(true, start);
        }
        if self.is_char('(') {
            return self.parenthesized_ref();
        }
        if let Some(gap) = self.gap_table(start, false)? {
            return Ok(gap);
        }
        if self.function_ahead() {
            return self.function_table(false, start);
        }
        let mut relation: TableRef = self.relation()?;
        if let Some(alias) = self.alias_clause(true)? {
            relation.alias = Some(alias.name);
            relation.alias_explicit_as = alias.explicit;
            relation.column_aliases = alias.columns;
        }
        if self.kw("tablesample") {
            let start: usize = self.offset();
            self.at += 1;
            self.name_expression_free()?;
            self.skip_parenthesized()?;
            if self.eat_kw("repeatable") {
                self.skip_parenthesized()?;
            }
            self.gap("tablesample", start);
        }
        Ok(Expression::Table(Box::new(relation)))
    }

    /// The function name of `TABLESAMPLE`, which may be qualified.
    fn name_expression_free(&mut self) -> Parsing<()> {
        self.qualified_name().map(|_| ())
    }

    /// Whether a function table starts here: a name and then `(`, or a value function.
    fn function_ahead(&self) -> bool {
        if let Some(word) = self.keyword_at(0)
            && VALUE_FUNCTIONS.iter().any(|(name, _)| *name == word)
        {
            return true;
        }
        let mut offset: usize = 0;
        while self.peek_at(offset).is_some_and(is_col_label) && self.char_at(offset + 1, '.') {
            offset += 2;
        }
        self.peek_at(offset).is_some_and(is_col_label) && self.char_at(offset + 1, '(')
    }

    /// `relation_expr`: `[ONLY] qualified_name [*]` or `ONLY (qualified_name)`.
    pub(crate) fn relation(&mut self) -> Parsing<TableRef> {
        let only: bool = self.eat_kw("only");
        let name: Vec<Identifier> = match only && self.is_char('(') {
            true => {
                self.at += 1;
                let name: Vec<Identifier> = self.table_name()?;
                self.expect_char(')')?;
                name
            }
            false => self.table_name()?,
        };
        if !only {
            self.eat_char('*');
        }
        let mut relation: TableRef = table(name);
        relation.only = only;
        Ok(relation)
    }

    /// `relation_expr` as an expression, for `TABLE`.
    pub(crate) fn relation_expression(&mut self) -> Parsing<Expression> {
        Ok(Expression::Table(Box::new(self.relation()?)))
    }

    /// `func_table func_alias_clause`, after any `LATERAL`.
    fn function_table(&mut self, is_lateral: bool, start: usize) -> Parsing<Expression> {
        let call: Expression = match self.keyword_at(0) {
            Some(word)
                if self.is_special_call(word)
                    || VALUE_FUNCTIONS.iter().any(|(name, _)| *name == word) =>
            {
                self.c_expr(Flavor::B)?
            }
            _ => {
                let mut parts: Vec<Identifier> = vec![identifier(&self.advance()?)];
                while self.eat_char('.') {
                    parts.push(self.name(is_col_label)?);
                }
                self.function_call(parts, true)?
            }
        };
        let call: Expression = match self.ordinality() {
            true => match call {
                Expression::Function(mut function) => {
                    function.name = format!("{} WITH ORDINALITY", function.name);
                    Expression::Function(function)
                }
                other => {
                    self.gap("ordinality", start);
                    other
                }
            },
            false => call,
        };
        let alias: Option<TableAlias> = self.function_alias()?;
        if is_lateral {
            let (name, columns) = match alias {
                Some(alias) => (Some(alias.name), alias.columns),
                None => (None, Vec::new()),
            };
            return Ok(lateral(call, name, columns));
        }
        Ok(aliased(call, alias))
    }

    /// `WITH ORDINALITY`, when present.
    fn ordinality(&mut self) -> bool {
        let found: bool = self.kw("with") && self.kw_at(1, "ordinality");
        self.at += 2 * usize::from(found);
        found
    }

    /// `func_alias_clause`: an alias whose list may define columns with types, which is a gap.
    fn function_alias(&mut self) -> Parsing<Option<TableAlias>> {
        let start: usize = self.offset();
        if self.kw("as") && self.char_at(1, '(') {
            self.at += 1;
            self.column_definitions()?;
            self.gap("column_definition_list", start);
            return Ok(None);
        }
        let explicit: bool = self.eat_kw("as");
        let name: Identifier = match self.is(is_col_id) {
            true => self.name(is_col_id)?,
            false if explicit => return Err(self.error()),
            false => return Ok(None),
        };
        let mut columns: Vec<Identifier> = Vec::new();
        if self.is_char('(') {
            match self.attempt(|parser| parser.name_list()) {
                Ok(names) => columns = names,
                Err(_) => {
                    self.column_definitions()?;
                    self.gap("column_definition_list", start);
                }
            }
        }
        Ok(Some(TableAlias {
            name,
            columns,
            explicit,
        }))
    }

    /// `( ColId Typename [COLLATE any_name], ... )`.
    fn column_definitions(&mut self) -> Parsing<()> {
        self.parenthesized(|parser| {
            parser.name(is_col_id)?;
            parser.typename()?;
            if parser.eat_kw("collate") {
                parser.qualified_name()?;
            }
            Ok(())
        })
        .map(|_| ())
    }

    /// `alias_clause`: `[AS] ColId [( name_list )]`, when present and `allowed`.
    fn alias_clause(&mut self, allowed: bool) -> Parsing<Option<TableAlias>> {
        if !allowed {
            return Ok(None);
        }
        let explicit: bool = self.eat_kw("as");
        let name: Identifier = match self.is(is_col_id) {
            true => self.name(is_col_id)?,
            false if explicit => return Err(self.error()),
            false => return Ok(None),
        };
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.name_list()?,
            false => Vec::new(),
        };
        Ok(Some(TableAlias {
            name,
            columns,
            explicit,
        }))
    }

    fn subquery_ref(
        &self,
        query: Expression,
        alias: Option<TableAlias>,
        is_lateral: bool,
    ) -> Expression {
        subquery(query, alias, is_lateral)
    }

    /// `(` in FROM: a subquery, or a parenthesized joined table.
    fn parenthesized_ref(&mut self) -> Parsing<Expression> {
        let reached: Option<usize> = match self.starts_query_at(1) {
            false => None,
            true => match self.attempt(|parser| {
                let query: Expression = parser.select_with_parens()?;
                let alias: Option<TableAlias> = parser.alias_clause(true)?;
                Ok(parser.subquery_ref(query, alias, false))
            }) {
                Ok(node) => return Ok(node),
                Err((error, _)) => Some(error.position),
            },
        };
        let joined = self.attempt(|parser| {
            parser.expect_char('(')?;
            let left: Expression = parser.table_ref()?;
            let joins: Vec<Join> = parser.joins()?;
            if joins.is_empty() && !matches!(left, Expression::JoinedTable(_)) {
                return Err(parser.error());
            }
            parser.expect_char(')')?;
            let start: usize = parser.offset();
            let alias: Option<TableAlias> = parser.alias_clause(true)?;
            if alias
                .as_ref()
                .is_some_and(|alias| !alias.columns.is_empty())
            {
                parser.gap("joined_table_column_aliases", start);
            }
            Ok(match joins.is_empty() {
                true => left,
                false => joined_table(left, joins, alias.map(|alias| alias.name)),
            })
        });
        match (joined, reached) {
            (Ok(node), _) => Ok(node),
            (Err((error, _)), Some(reached)) if error.position < reached => {
                Err(self.error_at_position(reached))
            }
            (Err((error, _)), _) => Err(error),
        }
    }
}
