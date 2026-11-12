//! `select`, `selectnowith`, `oneselect` and `with` (parse.y): queries, compounds and CTEs.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    Cte, GroupBy, Having, Limit, NamedWindow, Offset, OrderBy, Ordered, Over, Select, Where, With,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_ids, is_nm};
use crate::parse::_helpers::tree::nodes::{cte, set_operation, star, values};
use crate::parse::constants::{COMPOUND_TERMS, MAX_COMPOUND_SELECT};
use crate::scan::models::TokenKind;

/// One `oneselect` of a compound, with the `ORDER BY` and `LIMIT` only its last term may have.
struct Term {
    body: Expression,
    order_by: Option<OrderBy>,
    limit: Option<(Expression, Option<Expression>)>,
    values: bool,
}

/// A `WINDOW` entry as `sqlite3WindowChain` compares it: name as written, ordering, frame.
struct Written {
    name: String,
    orders: bool,
    framed: bool,
}

/// `body` with `order_by` and `limit` as its result clauses.
fn finished(
    body: Expression,
    order_by: Option<OrderBy>,
    limit: Option<(Expression, Option<Expression>)>,
) -> Expression {
    let (count, offset) = match limit {
        Some((count, offset)) => (Some(count), offset),
        None => (None, None),
    };
    macro_rules! set_operation_finished {
        ($node:expr) => {{
            $node.order_by = order_by;
            $node.limit = count.map(Box::new);
            $node.offset = offset.map(Box::new);
        }};
    }
    let mut body: Expression = body;
    match &mut body {
        Expression::Select(select) => {
            select.order_by = order_by;
            select.limit = count.map(|this| Limit {
                this,
                percent: false,
                comments: Vec::new(),
            });
            select.offset = offset.map(|this| Offset { this, rows: None });
        }
        Expression::Union(node) => set_operation_finished!(node),
        Expression::Intersect(node) => set_operation_finished!(node),
        Expression::Except(node) => set_operation_finished!(node),
        _ => {}
    }
    body
}

/// An empty `SELECT`, built outside the recursive frames that fill it in.
#[inline(never)]
fn new_select() -> Box<Select> {
    Box::new(Select::new())
}

/// `body` with `with` as its common table expressions.
fn with_ctes(mut body: Expression, with: Option<With>) -> Expression {
    match &mut body {
        Expression::Select(select) => select.with = with,
        Expression::Union(node) => node.with = with,
        Expression::Intersect(node) => node.with = with,
        Expression::Except(node) => node.with = with,
        _ => {}
    }
    body
}

/// `sqlite3SelectOpName` of a compound operator, as the misplaced-clause message names it.
fn operator_name(operator: &str, all: bool) -> &'static str {
    match (operator, all) {
        ("UNION", true) => "UNION ALL",
        ("UNION", false) => "UNION",
        ("INTERSECT", _) => "INTERSECT",
        _ => "EXCEPT",
    }
}

impl Parser<'_> {
    /// `select`: `[WITH [RECURSIVE] wqlist] selectnowith`.
    pub(crate) fn select(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let with: Option<With> = match parser.kw("WITH") {
                true => Some(parser.with_clause()?),
                false => None,
            };
            parser.select_with(with)
        })
    }

    /// `selectnowith` after its `WITH` clause, if it has one.
    pub(crate) fn select_with(&mut self, with: Option<With>) -> Parsing<Expression> {
        let first: usize = self.at;
        let start: usize = self.offset();
        let mut term: Term = self.one_select()?;
        let mut terms: usize = 1;
        let mut body: Expression = term.body;
        let mut last_values: bool = term.values;
        let mut misplaced: Option<String> = None;
        while let Some((operator, all)) = self.compound_operator() {
            if term.order_by.is_some() || term.limit.is_some() {
                let clause: &str = match term.order_by.is_some() {
                    true => "ORDER BY",
                    false => "LIMIT",
                };
                misplaced = Some(format!(
                    "{clause} clause should come after {} not before",
                    operator_name(operator, all)
                ));
            }
            term = self.one_select()?;
            terms += 1;
            last_values = term.values;
            body = set_operation(operator, body, term.body, (all, false));
        }
        if terms > MAX_COMPOUND_SELECT && !last_values {
            self.defer(self.error_with(COMPOUND_TERMS, start, false));
        } else if let Some(message) = misplaced {
            self.defer(self.error_with(&message, start, true));
        }
        let body: Expression = finished(body, term.order_by, term.limit);
        Ok(self.respanned(first, with_ctes(body, with)))
    }

    /// `multiselect_op`: `UNION [ALL]`, `INTERSECT` or `EXCEPT`, consumed.
    fn compound_operator(&mut self) -> Option<(&'static str, bool)> {
        let operator: &'static str = match self.kind_at(0)? {
            TokenKind::Keyword("UNION") => "UNION",
            TokenKind::Keyword("INTERSECT") => "INTERSECT",
            TokenKind::Keyword("EXCEPT") => "EXCEPT",
            _ => return None,
        };
        self.at += 1;
        let all: bool = matches!(operator, "UNION") && self.eat_kw("ALL");
        Some((operator, all))
    }

    /// `oneselect`: a `SELECT` with its clauses, or `VALUES` rows.
    fn one_select(&mut self) -> Parsing<Term> {
        let first: usize = self.at;
        if self.eat_kw("VALUES") {
            let mut rows: Vec<Vec<Expression>> = vec![self.values_row()?];
            while self.eat_op("COMMA") {
                rows.push(self.values_row()?);
            }
            return Ok(Term {
                body: self.respanned(first, values(rows, None)),
                order_by: None,
                limit: None,
                values: true,
            });
        }
        self.expect_kw("SELECT")?;
        let mut select: Box<Select> = new_select();
        if self.eat_kw("DISTINCT") {
            select.distinct = true;
        } else if self.eat_kw("ALL") {
            select.all = true;
        }
        select.expressions = self.comma_list(|parser| parser.result_column())?;
        if self.eat_kw("FROM") {
            select = self.select_from(select)?;
        }
        if self.eat_kw("WHERE") {
            select.where_clause = Some(Where {
                this: self.expression()?,
            });
        }
        if self.kw("GROUP") {
            self.at += 1;
            self.expect_kw("BY")?;
            select.group_by = Some(GroupBy {
                expressions: self.comma_list(|parser| parser.expression())?,
                all: None,
                totals: false,
                comments: Vec::new(),
            });
        }
        if self.eat_kw("HAVING") {
            select.having = Some(Having {
                this: self.expression()?,
                comments: Vec::new(),
            });
        }
        if self.eat_kw("WINDOW") {
            select.windows = Some(self.window_definitions()?);
        }
        let order_by: Option<OrderBy> = self.order_by_clause()?;
        let limit: Option<(Expression, Option<Expression>)> = self.limit_clause()?;
        let body: Expression = Expression::Select(select);
        Ok(Term {
            body: self.respanned(first, body),
            order_by,
            limit,
            values: false,
        })
    }

    /// `( nexprlist )`: one row of `VALUES`.
    fn values_row(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_op("LP")?;
        let row: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
        self.expect_op("RP")?;
        Ok(row)
    }

    /// `selcollist`'s item: `*`, `nm.*`, or an expression and its alias.
    fn result_column(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        if self.eat_op("STAR") {
            return Ok(self.spanned(first, Expression::Star(Box::new(star(None, None)))));
        }
        if self.is(is_nm) && self.op_at(1, "DOT") && self.op_at(2, "STAR") {
            let table: Identifier = self.name(is_nm)?;
            self.at += 2;
            let node = Expression::Star(Box::new(star(Some(table), None)));
            return Ok(self.spanned(first, node));
        }
        let this: Expression = self.expression()?;
        let explicit: bool = self.eat_kw("AS");
        let name: Option<Identifier> = match explicit {
            true => Some(self.name(is_nm)?),
            false if self.is(is_ids) => Some(self.name(is_ids)?),
            false => None,
        };
        Ok(match name {
            Some(name) => {
                let mut node: Alias = Alias::new(this, name);
                node.alias_explicit_as = explicit;
                self.respanned(first, Expression::Alias(Box::new(node)))
            }
            None => this,
        })
    }

    /// `orderby_opt`: `ORDER BY sortlist`, when it is there.
    pub(crate) fn order_by_clause(&mut self) -> Parsing<Option<OrderBy>> {
        if !self.kw("ORDER") {
            return Ok(None);
        }
        self.at += 1;
        self.expect_kw("BY")?;
        Ok(Some(OrderBy {
            expressions: self.sort_list()?,
            siblings: false,
            comments: Vec::new(),
            all: None,
        }))
    }

    /// `sortlist`: expressions, each with `ASC` or `DESC` and `NULLS FIRST` or `NULLS LAST`.
    pub(crate) fn sort_list(&mut self) -> Parsing<Vec<Ordered>> {
        self.comma_list(|parser| {
            let this: Expression = parser.expression()?;
            let (desc, explicit_asc) = match parser.kind_at(0) {
                Some(TokenKind::Keyword("ASC")) => (false, true),
                Some(TokenKind::Keyword("DESC")) => (true, false),
                _ => (false, false),
            };
            parser.at += usize::from(desc || explicit_asc);
            let mut nulls_first: Option<bool> = None;
            if parser.eat_kw("NULLS") {
                nulls_first = Some(match parser.kind_at(0) {
                    Some(TokenKind::Keyword("FIRST")) => true,
                    Some(TokenKind::Keyword("LAST")) => false,
                    _ => return Err(parser.error()),
                });
                parser.at += 1;
            }
            Ok(Ordered {
                nulls_auto: false,
                this,
                desc,
                nulls_first,
                explicit_asc,
                with_fill: None,
            })
        })
    }

    /// `limit_opt`: `LIMIT count [OFFSET offset]`, or `LIMIT offset, count`.
    fn limit_clause(&mut self) -> Parsing<Option<(Expression, Option<Expression>)>> {
        if !self.eat_kw("LIMIT") {
            return Ok(None);
        }
        let first: Expression = self.expression()?;
        if self.eat_kw("OFFSET") {
            let offset: Expression = self.expression()?;
            return Ok(Some((first, Some(offset))));
        }
        if self.eat_op("COMMA") {
            let count: Expression = self.expression()?;
            return Ok(Some((count, Some(first))));
        }
        Ok(Some((first, None)))
    }

    /// `with`: `WITH [RECURSIVE] wqitem, ...`.
    pub(crate) fn with_clause(&mut self) -> Parsing<With> {
        self.expect_kw("WITH")?;
        let recursive: bool = self.eat_kw("RECURSIVE");
        let mut ctes: Vec<Cte> = Vec::new();
        loop {
            let start: usize = self.offset();
            let node: Cte = self.common_table_expression()?;
            let name: &str = &node.alias.name;
            if ctes
                .iter()
                .any(|earlier| earlier.alias.name.eq_ignore_ascii_case(name))
            {
                let message: String = format!("duplicate WITH table name: {name}");
                self.defer(self.error_with(&message, start, true));
            }
            ctes.push(node);
            if !self.eat_op("COMMA") {
                break;
            }
        }
        Ok(With {
            ctes,
            recursive,
            leading_comments: Vec::new(),
            search: None,
        })
    }

    /// `wqitem`: `nm [( eidlist )] AS [[NOT] MATERIALIZED] ( select )`.
    fn common_table_expression(&mut self) -> Parsing<Cte> {
        let first: usize = self.at;
        let alias: Identifier = self.name(is_nm)?;
        let mut columns: Vec<Identifier> = Vec::new();
        if self.eat_op("LP") {
            columns = self.comma_list(|parser| parser.column_name())?;
            self.expect_op("RP")?;
        }
        self.expect_kw("AS")?;
        let materialized: Option<bool> = match self.kind_at(0) {
            Some(TokenKind::Keyword("MATERIALIZED")) => {
                self.at += 1;
                Some(true)
            }
            Some(TokenKind::Keyword("NOT")) => {
                self.at += 1;
                self.expect_kw("MATERIALIZED")?;
                Some(false)
            }
            _ => None,
        };
        self.expect_op("LP")?;
        let body: Expression = self.select()?;
        self.expect_op("RP")?;
        let mut node: Cte = cte(alias, columns, body);
        node.materialized = materialized;
        node.span = self.source_range(first);
        Ok(node)
    }

    /// A CTE column name: `nm`, refusing a `COLLATE` or sort order after it as SQLite does.
    pub(crate) fn column_name(&mut self) -> Parsing<Identifier> {
        let token = self.expect_token(is_nm)?;
        let collate: bool = self.kw("COLLATE");
        if collate {
            self.at += 1;
            self.expect_token(is_ids)?;
        }
        let ordered: bool = self.kw("ASC") || self.kw("DESC");
        if collate || ordered {
            let message: String = format!(
                "syntax error after column name \"{}\"",
                &self.sql[token.start..token.end]
            );
            return Err(self.error_with(&message, token.start, true));
        }
        Ok(identifier(&token))
    }

    /// `windowdefn_list`: `nm AS ( window ), ...`, each chained to the earlier one it names.
    fn window_definitions(&mut self) -> Parsing<Vec<NamedWindow>> {
        let mut windows: Vec<NamedWindow> = Vec::new();
        let mut written: Vec<Written> = Vec::new();
        loop {
            let token = self.expect_token(is_nm)?;
            self.expect_kw("AS")?;
            self.expect_op("LP")?;
            let start: usize = self.offset();
            let (spec, base) = self.window()?;
            self.expect_op("RP")?;
            let mut orders: bool = !spec.order_by.is_empty();
            if let (Some(base), false) = (base, windows.is_empty()) {
                orders |= self.chain_window(&spec, &base, &written, start)?;
            }
            written.push(Written {
                name: self.sql[token.start..token.end].to_owned(),
                orders,
                framed: spec.frame.is_some(),
            });
            windows.push(NamedWindow {
                name: identifier(&token),
                spec,
            });
            if !self.eat_op("COMMA") {
                return Ok(windows);
            }
        }
    }

    /// `sqlite3WindowChain`'s checks of a window on its base; whether the window then orders.
    fn chain_window(
        &self,
        spec: &Over,
        base: &str,
        earlier: &[Written],
        start: usize,
    ) -> Parsing<bool> {
        let Some(existing) = earlier
            .iter()
            .rev()
            .find(|window| window.name.eq_ignore_ascii_case(base))
        else {
            return Err(self.error_with(&format!("no such window: {base}"), start, false));
        };
        let overridden: Option<&str> = if !spec.partition_by.is_empty() {
            Some("PARTITION clause")
        } else if existing.orders && !spec.order_by.is_empty() {
            Some("ORDER BY clause")
        } else if existing.framed {
            Some("frame specification")
        } else {
            None
        };
        match overridden {
            Some(clause) => Err(self.error_with(
                &format!("cannot override {clause} of window: {base}"),
                start,
                false,
            )),
            None => Ok(existing.orders),
        }
    }
}
