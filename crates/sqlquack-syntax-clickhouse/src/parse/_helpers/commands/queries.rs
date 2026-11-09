//! `ParserSelectWithUnionQuery`, `ParserSelectQuery`, `ParserWithElement` and settings lists.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::queries::models::{
    Cte, GroupBy, Having, Limit, NamedWindow, Offset, OrderBy, Ordered, Qualify, Select, Where,
    With,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{binary, column_from, cte, set_operation, string};
use crate::scan::main::unquote::unquote;
use crate::scan::models::TokenKind;

/// The precedence of a set operator: `INTERSECT` binds tighter than `UNION` and `EXCEPT`.
fn set_level(word: &str) -> Option<u8> {
    match word {
        "UNION" | "EXCEPT" => Some(1),
        "INTERSECT" => Some(2),
        _ => None,
    }
}

fn order_by(expressions: Vec<Ordered>) -> OrderBy {
    OrderBy {
        expressions,
        siblings: false,
        comments: Vec::new(),
        all: None,
    }
}

fn limit(this: Expression) -> Limit {
    Limit {
        this,
        percent: false,
        comments: Vec::new(),
    }
}

impl Parser<'_> {
    /// `ParserSelectWithUnionQuery`: queries joined by `UNION`, `EXCEPT` and `INTERSECT`.
    pub(crate) fn select_with_union(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let first: usize = parser.at;
            let query: Expression = parser.union_term(0)?;
            Ok(parser.respanned(first, query))
        })
    }

    /// Set operations of precedence above `min`, left-associative.
    fn union_term(&mut self, min: u8) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut left: Expression = self.union_element()?;
        let mut links: usize = 0;
        loop {
            let Some(word) = self.word_at(0) else {
                return Ok(left);
            };
            let Some(level) = set_level(&word) else {
                return Ok(left);
            };
            if level <= min {
                return Ok(left);
            }
            links += 1;
            self.within_depth(links)?;
            self.at += 1;
            let all: bool = self.eat_kw("ALL");
            let distinct: bool = !all && self.eat_kw("DISTINCT");
            let right: Expression = self.deeper(|parser| parser.union_term(level))?;
            left = self.spanned(first, set_operation(&word, left, right, (all, distinct)));
        }
    }

    /// `ParserUnionQueryElement`: a query in brackets, or a `SELECT`.
    fn union_element(&mut self) -> Parsing<Expression> {
        if self.is(TokenKind::OpeningRoundBracket) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.select_with_union()?;
                parser.expect_kind(TokenKind::ClosingRoundBracket)?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                return Ok(crate::parse::_helpers::tree::nodes::subquery(
                    query, None, false,
                ));
            }
        }
        self.select_query()
    }

    /// `ParserSelectQuery`.
    fn select_query(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut select: Select = Select::new();
        if self.eat_kw("WITH") {
            select.with = Some(self.with_clause()?);
        }
        let from_first: bool = self.eat_kw("FROM");
        if from_first {
            select = self.tables(select, false)?;
        }
        self.expect_kw("SELECT")?;
        select = self.select_head(select)?;
        if !from_first && self.eat_kw("FROM") {
            select = self.tables(select, true)?;
        }
        if select.from.is_some() && self.is(TokenKind::OpeningRoundBracket) {
            let start: usize = self.offset();
            self.at += 1;
            self.comma_list(|parser| parser.ident())?;
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
            self.gap("table_column_aliases", start);
        }
        select = self.select_filters(select)?;
        select = self.select_grouping(select)?;
        select = self.select_ordering(select)?;
        select = self.select_limits(select)?;
        if self.eat_kw("SETTINGS") {
            select.settings = Some(self.settings_list()?);
        }
        Ok(self.spanned(first, Expression::Select(Box::new(select))))
    }

    /// `[ALL] [DISTINCT [ON (...)]] [ALL] [TOP n [WITH TIES]] expressions`.
    fn select_head(&mut self, mut select: Select) -> Parsing<Select> {
        let mut all: bool = self.eat_kw("ALL");
        if self.kws(&["DISTINCT", "ON"]) {
            self.at += 2;
            self.expect_kind(TokenKind::OpeningRoundBracket)?;
            select.distinct_on = Some(self.comma_list(|parser| parser.element())?);
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
        } else if self.eat_kw("DISTINCT") {
            select.distinct = true;
        }
        if !all {
            all = self.eat_kw("ALL");
        }
        if all && (select.distinct || select.distinct_on.is_some()) {
            return Err(self.error());
        }
        select.all = all;
        if self.kw("TOP") {
            let start: usize = self.offset();
            self.at += 1;
            let bracketed: bool = self.eat(TokenKind::OpeningRoundBracket);
            self.expect_kind(TokenKind::Number)?;
            if bracketed {
                self.expect_kind(TokenKind::ClosingRoundBracket)?;
            }
            self.eat_kws(&["WITH", "TIES"]);
            self.gap("select_top", start);
        }
        select.expressions = self.comma_list(|parser| parser.aliased())?;
        Ok(select)
    }

    /// `PREWHERE` and `WHERE`.
    fn select_filters(&mut self, mut select: Select) -> Parsing<Select> {
        if self.eat_kw("PREWHERE") {
            select.prewhere = Some(self.element()?);
        }
        if self.eat_kw("WHERE") {
            select.where_clause = Some(Where {
                this: self.element()?,
            });
        }
        Ok(select)
    }

    /// `GROUP BY`, `WITH ROLLUP|CUBE|TOTALS`, `HAVING`, `WINDOW` and `QUALIFY`.
    fn select_grouping(&mut self, mut select: Select) -> Parsing<Select> {
        if self.eat_kws(&["GROUP", "BY"]) {
            let start: usize = self.offset();
            let mut group: GroupBy = GroupBy {
                expressions: Vec::new(),
                all: None,
                totals: false,
                comments: Vec::new(),
            };
            if self.kw_in(0, &["ROLLUP", "CUBE"]) || self.kws(&["GROUPING", "SETS"]) {
                self.skip_grouping_modifier()?;
                self.gap("group_by_modifier", start);
            } else if self.eat_kw("ALL") {
                group.all = Some(true);
            } else {
                group.expressions = self.comma_list(|parser| parser.element())?;
            }
            select.group_by = Some(group);
        }
        let mut totals: bool = false;
        if self.kw("WITH") {
            let start: usize = self.offset();
            match self.word_at(1).as_deref() {
                Some("TOTALS") => totals = true,
                Some("ROLLUP" | "CUBE") => self.gap("group_by_modifier", start),
                _ => return Err(self.error_at(1)),
            }
            self.at += 2;
        }
        if self.kw("WITH") {
            if totals || !self.kw_at(1, "TOTALS") {
                return Err(self.error_at(1));
            }
            totals = true;
            self.at += 2;
        }
        if totals {
            match &mut select.group_by {
                Some(group) => group.totals = true,
                None => {
                    let start: usize = self.offset();
                    self.gap("totals_without_group_by", start);
                }
            }
        }
        if self.eat_kw("HAVING") {
            select.having = Some(Having {
                this: self.element()?,
                comments: Vec::new(),
            });
        }
        if self.eat_kw("WINDOW") {
            select.windows = Some(self.comma_list(|parser| parser.named_window())?);
        }
        if self.eat_kw("QUALIFY") {
            select.qualify = Some(Qualify {
                this: self.element()?,
            });
        }
        Ok(select)
    }

    /// `ROLLUP (...)`, `CUBE (...)` or `GROUPING SETS (...)`, parsed and not kept.
    fn skip_grouping_modifier(&mut self) -> Parsing<()> {
        if !self.eat_kws(&["GROUPING", "SETS"]) {
            self.at += 1;
        }
        self.expect_kind(TokenKind::OpeningRoundBracket)?;
        self.comma_list(|parser| parser.element())?;
        self.expect_kind(TokenKind::ClosingRoundBracket)
    }

    /// `ParserWindowList`'s item: `name AS (definition)`.
    fn named_window(&mut self) -> Parsing<NamedWindow> {
        let name: Identifier = self.ident()?;
        self.expect_kw("AS")?;
        let spec = self.window_definition()?;
        Ok(NamedWindow { name, spec })
    }

    /// `ORDER BY` with `INTERPOLATE` after a `WITH FILL`.
    fn select_ordering(&mut self, mut select: Select) -> Parsing<Select> {
        if !self.eat_kws(&["ORDER", "BY"]) {
            return Ok(select);
        }
        let items: Vec<Ordered> = self.comma_list(|parser| parser.order_element())?;
        let filled: bool = items.iter().any(|item| item.with_fill.is_some());
        select.order_by = Some(order_by(items));
        if filled && self.kw("INTERPOLATE") {
            let start: usize = self.offset();
            self.at += 1;
            if self.eat(TokenKind::OpeningRoundBracket) {
                if !self.is(TokenKind::ClosingRoundBracket) {
                    self.comma_list(|parser| {
                        parser.ident()?;
                        if parser.eat_kw("AS") {
                            parser.expression()?;
                        }
                        Ok(())
                    })?;
                }
                self.expect_kind(TokenKind::ClosingRoundBracket)?;
            }
            self.gap("interpolate", start);
        }
        Ok(select)
    }

    /// `LIMIT [o,] n [OFFSET o] [WITH TIES] [BY ...]`, `OFFSET ... [FETCH ...]`, then a LIMIT.
    fn select_limits(&mut self, mut select: Select) -> Parsing<Select> {
        let mut limited: bool = false;
        if self.eat_kw("LIMIT") {
            limited = true;
            let (count, offset) = self.limit_values()?;
            if self.kws(&["WITH", "TIES"]) {
                let ties: usize = self.offset();
                self.at += 2;
                self.gap("with_ties", ties);
            }
            if self.eat_kw("BY") {
                select.limit_by = Some(match self.eat_kw("ALL") {
                    true => Vec::new(),
                    false => self.comma_list(|parser| parser.element())?,
                });
            }
            select.limit = Some(limit(count));
            select.offset = offset.map(|this| Offset { this, rows: None });
        } else if self.eat_kw("OFFSET") {
            let this: Expression = self.element()?;
            let row: bool = self.eat_kw("ROW");
            let rows: bool = self.eat_kw("ROWS");
            if row && rows {
                return Err(self.error_with("Can not use ROW and ROWS together", false));
            }
            select.offset = Some(Offset {
                this,
                rows: (row || rows).then_some(true),
            });
            if self.kw("FETCH") {
                let fetch: usize = self.offset();
                self.skip_fetch()?;
                self.gap("offset_fetch", fetch);
            }
        }
        if limited && select.limit_by.is_some() && self.kw("LIMIT") {
            let second: usize = self.offset();
            self.at += 1;
            self.limit_values()?;
            self.eat_kws(&["WITH", "TIES"]);
            self.gap("limit_after_limit_by", second);
        }
        Ok(select)
    }

    /// `n`, `o, n` or `n OFFSET o`: the count and the offset.
    fn limit_values(&mut self) -> Parsing<(Expression, Option<Expression>)> {
        let first: Expression = self.element()?;
        if self.eat(TokenKind::Comma) {
            let count: Expression = self.element()?;
            return Ok((count, Some(first)));
        }
        if self.eat_kw("OFFSET") {
            let offset: Expression = self.element()?;
            return Ok((first, Some(offset)));
        }
        Ok((first, None))
    }

    /// `FETCH FIRST|NEXT n ROW|ROWS ONLY|WITH TIES`, parsed and not kept.
    fn skip_fetch(&mut self) -> Parsing<()> {
        self.expect_kw("FETCH")?;
        if !self.eat_kw("FIRST") {
            self.expect_kw("NEXT")?;
        }
        self.element()?;
        if !self.eat_kw("ROW") {
            self.expect_kw("ROWS")?;
        }
        if !self.eat_kws(&["WITH", "TIES"]) {
            self.expect_kw("ONLY")?;
        }
        Ok(())
    }

    /// The error at the token `offset` ahead.
    pub(crate) fn error_at(&mut self, offset: usize) -> crate::parse::errors::ParseError {
        let at: usize = self.at;
        self.at = (at + offset).min(self.tokens.len());
        let error = self.error();
        self.at = at;
        error
    }

    /// `WITH [RECURSIVE] element, ...`, after `WITH`.
    pub(crate) fn with_clause(&mut self) -> Parsing<With> {
        let recursive: bool = self.eat_kw("RECURSIVE");
        let ctes: Vec<Cte> = self.comma_list(|parser| {
            let first: usize = parser.at;
            let mut node: Cte = parser.with_element()?;
            node.span = parser.source_range(first);
            Ok(node)
        })?;
        Ok(With {
            ctes,
            recursive,
            leading_comments: Vec::new(),
            search: None,
        })
    }

    /// `ParserWithElement`: `name [(aliases)] AS (query)`, or `expr [AS name]`.
    fn with_element(&mut self) -> Parsing<Cte> {
        let named = self.attempt(|parser| {
            let name: Identifier = match parser.is_at(0, is_identifier) {
                true => parser.ident()?,
                false => return Err(parser.error()),
            };
            let columns: Vec<Identifier> = match parser.is(TokenKind::OpeningRoundBracket) {
                true => {
                    parser.at += 1;
                    let columns: Vec<Identifier> = parser.comma_list(|parser| parser.ident())?;
                    parser.expect_kind(TokenKind::ClosingRoundBracket)?;
                    columns
                }
                false => Vec::new(),
            };
            parser.expect_kw("AS")?;
            if !parser.subquery_at(0) {
                return Err(parser.error());
            }
            parser.expect_kind(TokenKind::OpeningRoundBracket)?;
            let body: Expression = parser.select_with_union()?;
            parser.expect_kind(TokenKind::ClosingRoundBracket)?;
            Ok(cte(name, columns, body))
        });
        if let Ok(node) = named {
            return Ok(node);
        }
        let start: usize = self.offset();
        let this: Expression = self.expression()?;
        match self.alias_name(false)? {
            Some((name, _)) => {
                let mut node: Cte = cte(name, Vec::new(), this);
                node.alias_first = false;
                Ok(node)
            }
            None => {
                self.gap("with_expression_without_alias", start);
                Ok(cte(Identifier::empty(), Vec::new(), this))
            }
        }
    }

    /// `ParserSetQuery(true)`: `name = value, ...` as equalities.
    pub(crate) fn settings_list(&mut self) -> Parsing<Vec<Expression>> {
        self.comma_list(|parser| {
            let name: Vec<Identifier> = parser.compound_identifier()?;
            parser.expect_kind(TokenKind::Equals)?;
            let value: Expression = parser.setting_value()?;
            Ok(binary(Expression::Eq, column_from(name), value))
        })
    }

    /// `DEFAULT`, `disk(...)`, `ParserLiteral` or a `{'k': v}` map.
    fn setting_value(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        if self.kw("DEFAULT") {
            self.at += 1;
            self.gap("setting_default", start);
            return Ok(crate::parse::_helpers::tree::nodes::var("DEFAULT"));
        }
        if self.kw("disk") && self.kind_at(1, TokenKind::OpeningRoundBracket) {
            self.at += 1;
            let parts: Vec<Identifier> = vec![Identifier::new("disk")];
            self.at += 1;
            let node: Expression = self.function_layer(parts)?;
            self.gap("setting_disk", start);
            return Ok(node);
        }
        if self.eat(TokenKind::OpeningCurlyBrace) {
            if !self.is(TokenKind::ClosingCurlyBrace) {
                self.comma_list(|parser| {
                    parser.string_literal()?;
                    parser.expect_kind(TokenKind::Colon)?;
                    parser.literal_value().map(|_| ())
                })?;
            }
            self.expect_kind(TokenKind::ClosingCurlyBrace)?;
            self.gap("setting_map", start);
            return Ok(string(""));
        }
        self.literal_value()
    }

    /// `ParserLiteral`: a signed number, a string, `NULL`, `TRUE` or `FALSE`.
    pub(crate) fn literal_value(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::Number | TokenKind::StringLiteral => self.operand(),
            TokenKind::Minus | TokenKind::Plus if self.kind_at(1, TokenKind::Number) => {
                let negative: bool = token.kind == TokenKind::Minus;
                self.at += 1;
                let digits: String = self.advance()?.value;
                Ok(crate::parse::_helpers::tree::nodes::literal(
                    sqlquack_syntax::expressions::names::models::Literal::Number(match negative {
                        true => format!("-{digits}"),
                        false => digits,
                    }),
                ))
            }
            TokenKind::BareWord if self.kw_in(0, &["NULL", "TRUE", "FALSE"]) => self.operand(),
            _ => Err(self.error()),
        }
    }

    /// `FORMAT name`: the format a query's output is written in.
    pub(crate) fn format_name(&mut self) -> Parsing<Expression> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::BareWord => Ok(crate::parse::_helpers::tree::nodes::var(&token.value)),
            TokenKind::QuotedIdentifier | TokenKind::StringLiteral => Ok(
                crate::parse::_helpers::tree::nodes::var(&unquote(&token.value)),
            ),
            _ => Err(self.error_at(0)),
        }
    }
}
