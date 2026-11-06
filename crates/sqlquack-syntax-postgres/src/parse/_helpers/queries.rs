//! `SelectStmt` (gram.y): simple selects, set operations, `WITH`, and the options after them.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{BooleanLiteral, Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    Cte, Fetch, From, GroupBy, Having, Limit, NamedWindow, Offset, OrderBy, Ordered, Select,
    SelectInto, Where, With,
};
use sqlquack_syntax::expressions::statements::models::Lock;

use crate::parse::_helpers::cursor::{Parser, Parsing, is_bare_label, is_col_id, is_col_label};
use crate::parse::_helpers::nodes::{
    cte, function, literal, null, set_operation, star, subquery, table, tuple, values,
};
use crate::scan::models::TokenKind;

/// `LIMIT`, `FETCH` and `OFFSET` as written after a query.
#[derive(Default)]
struct Limits {
    limit: Option<Expression>,
    fetch: Option<Fetch>,
    offset: Option<Offset>,
    position: usize,
}

/// `FOR UPDATE` and its kin, with whether any skips locked rows.
struct Locking {
    locks: Vec<Lock>,
    skip_locked: bool,
}

fn boolean_box(value: bool) -> Box<Expression> {
    Box::new(Expression::Boolean(BooleanLiteral { value }))
}

/// `VALUES` given options it cannot hold, as `SELECT * FROM (VALUES ...)`.
fn selectable(body: Expression) -> Expression {
    match body {
        Expression::Values(_) => {
            let mut select: Select = Select::new();
            select.expressions = vec![star(None)];
            select.from = Some(From {
                expressions: vec![subquery(body, None, Vec::new(), false, false)],
            });
            Expression::Select(Box::new(select))
        }
        other => other,
    }
}

impl Parser<'_> {
    /// `select_with_parens`: `( query )`.
    pub(crate) fn select_with_parens(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let query: Expression = self.query()?;
        self.expect_char(')')?;
        Ok(query)
    }

    /// `SelectStmt`: `[WITH ...] select_clause [ORDER BY] [LIMIT|OFFSET|FETCH] [FOR ...]`.
    pub(crate) fn query(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let with: Option<With> = match parser.kw("with") {
                true => Some(parser.with_clause()?),
                false => None,
            };
            let body: Expression = parser.set_expression(0)?;
            parser.select_options(body, with)
        })
    }

    /// `select_clause` with `UNION`/`EXCEPT` (left, lowest) and `INTERSECT` (left, tighter).
    pub(crate) fn set_expression(&mut self, min: u8) -> Parsing<Expression> {
        let mut left: Expression = self.select_primary()?;
        loop {
            let operator: &'static str = match self.keyword_at(0) {
                Some(word @ ("union" | "except" | "intersect")) => word,
                _ => return Ok(left),
            };
            let level: u8 = match operator {
                "intersect" => 2,
                _ => 1,
            };
            if level < min {
                return Ok(left);
            }
            self.at += 1;
            let all: bool = self.eat_kw("all");
            let distinct: bool = !all && self.eat_kw("distinct");
            let right: Expression = self.deeper(|parser| parser.set_expression(level + 1))?;
            left = set_operation(operator, left, right, all, distinct);
        }
    }

    /// `simple_select` without set operations, or a parenthesized query.
    fn select_primary(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            return self.select_with_parens();
        }
        match self.keyword_at(0) {
            Some("select") => self.simple_select(),
            Some("values") => self.values_clause(),
            Some("table") => {
                self.at += 1;
                let relation: Expression = self.relation_expression()?;
                let mut select: Select = Select::new();
                select.expressions = vec![star(None)];
                select.from = Some(From {
                    expressions: vec![relation],
                });
                Ok(Expression::Select(Box::new(select)))
            }
            _ => Err(self.error()),
        }
    }

    /// `VALUES ( expr_list ), ...`.
    fn values_clause(&mut self) -> Parsing<Expression> {
        self.expect_kw("values")?;
        let rows: Vec<Vec<Expression>> = self.comma_list(|parser| {
            parser.expect_char('(')?;
            let row: Vec<Expression> = parser.expr_list()?;
            parser.expect_char(')')?;
            Ok(row)
        })?;
        Ok(values(rows))
    }

    /// `SELECT [ALL | DISTINCT [ON (...)]] targets [INTO] [FROM] [WHERE] [GROUP BY] [HAVING]
    /// [WINDOW]`.
    fn simple_select(&mut self) -> Parsing<Expression> {
        self.expect_kw("select")?;
        let mut select: Select = Select::new();
        let distinct: bool = self.eat_kw("distinct");
        if distinct {
            select.distinct = true;
            if self.eat_kw("on") {
                select.distinct_on = Some(self.parenthesized(|parser| parser.a_expr())?);
            }
        } else {
            select.all = self.eat_kw("all");
        }
        if distinct || self.is_char('*') || self.starts_expression_at(0) {
            select.expressions = self.comma_list(|parser| parser.target())?;
        }
        if self.eat_kw("into") {
            select.into = Some(self.select_into_target()?);
        }
        if self.eat_kw("from") {
            let (items, joins) = self.parse_from_list()?;
            select.from = Some(From { expressions: items });
            select.joins = joins;
        }
        if self.eat_kw("where") {
            select.where_clause = Some(Where {
                this: self.a_expr()?,
            });
        }
        if self.kw("group") && self.kw_at(1, "by") {
            let start: usize = self.offset();
            self.at += 2;
            if self.eat_kw("distinct") {
                self.gap("group_by_distinct", start);
            } else {
                self.eat_kw("all");
            }
            select.group_by = Some(GroupBy {
                expressions: self.comma_list(|parser| parser.group_item())?,
                all: None,
                totals: false,
                comments: Vec::new(),
            });
        }
        if self.eat_kw("having") {
            select.having = Some(Having {
                this: self.a_expr()?,
                comments: Vec::new(),
            });
        }
        if self.eat_kw("window") {
            select.windows = Some(self.comma_list(|parser| {
                let name: Identifier = parser.name(is_col_id)?;
                parser.expect_kw("as")?;
                Ok(NamedWindow {
                    name,
                    spec: parser.window_specification()?,
                })
            })?);
        }
        Ok(Expression::Select(Box::new(select)))
    }

    /// `target_el`: `*`, `a_expr AS ColLabel`, `a_expr BareColLabel` or `a_expr`.
    pub(crate) fn target(&mut self) -> Parsing<Expression> {
        if self.is_char('*') {
            self.at += 1;
            return Ok(star(None));
        }
        let this: Expression = self.a_expr()?;
        let explicit: bool = self.eat_kw("as");
        let alias: Identifier = match explicit {
            true => self.name(is_col_label)?,
            false if self.is(is_bare_label) => self.name(is_bare_label)?,
            false => return Ok(this),
        };
        let mut node: Alias = Alias::new(this, alias);
        node.alias_explicit_as = explicit;
        Ok(Expression::Alias(Box::new(node)))
    }

    /// `OptTempTableName`: the table `SELECT ... INTO` creates.
    fn select_into_target(&mut self) -> Parsing<SelectInto> {
        let mut temporary: bool = false;
        let mut unlogged: bool = false;
        if (self.kw("local") || self.kw("global"))
            && (self.kw_at(1, "temporary") || self.kw_at(1, "temp"))
        {
            self.at += 2;
            temporary = true;
            self.eat_kw("table");
        } else if self.eat_kw("temporary") || self.eat_kw("temp") {
            temporary = true;
            self.eat_kw("table");
        } else if self.eat_kw("unlogged") {
            unlogged = true;
            self.eat_kw("table");
        } else {
            self.eat_kw("table");
        }
        let name: Vec<Identifier> = self.table_name()?;
        Ok(SelectInto {
            this: Expression::Table(Box::new(table(name))),
            temporary,
            unlogged,
            bulk_collect: false,
            expressions: Vec::new(),
        })
    }

    /// `group_by_item`: `()`, `CUBE (...)`, `ROLLUP (...)`, `GROUPING SETS (...)` or `a_expr`.
    fn group_item(&mut self) -> Parsing<Expression> {
        if self.is_char('(') && self.char_at(1, ')') {
            self.at += 2;
            return Ok(tuple(Vec::new()));
        }
        let name: Option<&str> = match self.keyword_at(0) {
            Some("cube") if self.char_at(1, '(') => Some("CUBE"),
            Some("rollup") if self.char_at(1, '(') => Some("ROLLUP"),
            Some("grouping") if self.kw_at(1, "sets") => Some("GROUPING SETS"),
            _ => None,
        };
        let Some(name) = name else {
            return self.a_expr();
        };
        self.at += match name {
            "GROUPING SETS" => 2,
            _ => 1,
        };
        let args: Vec<Expression> = match name {
            "GROUPING SETS" => self.parenthesized(|parser| parser.group_item())?,
            _ => self.parenthesized(|parser| parser.a_expr())?,
        };
        Ok(Expression::Function(Box::new(function(
            vec![Identifier::new(name)],
            args,
        ))))
    }

    /// `with_clause`: `WITH [RECURSIVE] cte, ...`.
    pub(crate) fn with_clause(&mut self) -> Parsing<With> {
        self.expect_kw("with")?;
        let recursive: bool = self.eat_kw("recursive");
        let ctes: Vec<Cte> = self.comma_list(|parser| parser.common_table_expression())?;
        Ok(With {
            ctes,
            recursive,
            leading_comments: Vec::new(),
            search: None,
        })
    }

    /// `common_table_expr`: `name [(columns)] AS [[NOT] MATERIALIZED] ( statement )` with
    /// `SEARCH` and `CYCLE` recorded as gaps.
    fn common_table_expression(&mut self) -> Parsing<Cte> {
        let alias: Identifier = self.name(is_col_id)?;
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.name_list()?,
            false => Vec::new(),
        };
        self.expect_kw("as")?;
        let materialized: Option<bool> = if self.eat_kw("materialized") {
            Some(true)
        } else if self.kw("not") && self.kw_at(1, "materialized") {
            self.at += 2;
            Some(false)
        } else {
            None
        };
        self.expect_char('(')?;
        let body: Expression = self.preparable_statement()?;
        self.expect_char(')')?;
        let start: usize = self.offset();
        if self.eat_kw("search") {
            if !(self.eat_kw("breadth") || self.eat_kw("depth")) {
                return Err(self.error());
            }
            self.expect_kw("first")?;
            self.expect_kw("by")?;
            self.comma_list(|parser| parser.name(is_col_id))?;
            self.expect_kw("set")?;
            self.name(is_col_id)?;
            self.gap("cte_search", start);
        }
        let start: usize = self.offset();
        if self.eat_kw("cycle") {
            self.comma_list(|parser| parser.name(is_col_id))?;
            self.expect_kw("set")?;
            self.name(is_col_id)?;
            if self.eat_kw("to") {
                self.c_expr(crate::parse::_helpers::expressions::Flavor::B)?;
                self.expect_kw("default")?;
                self.c_expr(crate::parse::_helpers::expressions::Flavor::B)?;
            }
            self.expect_kw("using")?;
            self.name(is_col_id)?;
            self.gap("cte_cycle", start);
        }
        Ok(cte(alias, columns, body, materialized))
    }

    /// The options after a query body, applied as gram.y's `insertSelectOptions` does.
    pub(crate) fn select_options(
        &mut self,
        body: Expression,
        with: Option<With>,
    ) -> Parsing<Expression> {
        let order_position: usize = self.offset();
        let order: Option<Vec<Ordered>> = match self.kw("order") && self.kw_at(1, "by") {
            true => Some(self.sort_clause()?),
            false => None,
        };
        let (limits, locking) = match self.kw("for") {
            true => {
                let locking: Locking = self.locking()?;
                (self.limits()?, locking)
            }
            false => {
                let limits: Limits = self.limits()?;
                let locking: Locking = match self.kw("for") {
                    true => self.locking()?,
                    false => Locking {
                        locks: Vec::new(),
                        skip_locked: false,
                    },
                };
                (limits, locking)
            }
        };
        let options: bool = order.is_some()
            || limits.limit.is_some()
            || limits.fetch.is_some()
            || limits.offset.is_some()
            || !locking.locks.is_empty()
            || with.is_some();
        if !options {
            return Ok(body);
        }
        let mut body: Expression = selectable(body);
        self.apply_options(&mut body, order, order_position, limits, locking, with)?;
        Ok(body)
    }

    fn apply_options(
        &mut self,
        body: &mut Expression,
        order: Option<Vec<Ordered>>,
        order_position: usize,
        limits: Limits,
        locking: Locking,
        with: Option<With>,
    ) -> Parsing<()> {
        let with_ties: bool = limits.fetch.as_ref().is_some_and(|fetch| fetch.with_ties);
        let start: usize = self.offset();
        macro_rules! set_operation_options {
            ($node:expr) => {{
                let node = $node;
                if let Some(order) = order {
                    if node.order_by.is_some() {
                        return Err(self
                            .error_with("multiple ORDER BY clauses not allowed", order_position));
                    }
                    node.order_by = Some(order_by(order));
                }
                if let Some(offset) = limits.offset {
                    if node.offset.is_some() {
                        return Err(
                            self.error_with("multiple OFFSET clauses not allowed", limits.position)
                        );
                    }
                    node.offset = Some(Box::new(offset.this));
                }
                let count: Option<Expression> = match limits.fetch {
                    Some(fetch) => Some(
                        fetch
                            .count
                            .unwrap_or_else(|| literal(Literal::Number("1".to_owned()))),
                    ),
                    None => limits.limit,
                };
                if let Some(count) = count {
                    if node.limit.is_some() {
                        return Err(
                            self.error_with("multiple LIMIT clauses not allowed", limits.position)
                        );
                    }
                    node.limit = Some(Box::new(count));
                }
                if with_ties && node.order_by.is_none() {
                    return Err(self.error_with(
                        "WITH TIES cannot be specified without ORDER BY clause",
                        limits.position,
                    ));
                }
                if with_ties {
                    self.gap("set_operation_with_ties", start);
                }
                if !locking.locks.is_empty() {
                    self.gap("set_operation_locking", start);
                }
                if let Some(with) = with {
                    if node.with.is_some() {
                        return Err(self.error_with("multiple WITH clauses not allowed", start));
                    }
                    node.with = Some(with);
                }
            }};
        }
        match body {
            Expression::Select(select) => {
                if let Some(order) = order {
                    if select.order_by.is_some() {
                        return Err(self
                            .error_with("multiple ORDER BY clauses not allowed", order_position));
                    }
                    select.order_by = Some(order_by(order));
                }
                if let Some(offset) = limits.offset {
                    if select.offset.is_some() {
                        return Err(
                            self.error_with("multiple OFFSET clauses not allowed", limits.position)
                        );
                    }
                    select.offset = Some(offset);
                }
                if limits.limit.is_some() || limits.fetch.is_some() {
                    if select.limit.is_some() || select.fetch.is_some() {
                        return Err(
                            self.error_with("multiple LIMIT clauses not allowed", limits.position)
                        );
                    }
                    select.limit = limits.limit.map(|this| Limit {
                        this,
                        percent: false,
                        comments: Vec::new(),
                    });
                    select.fetch = limits.fetch;
                }
                if with_ties && select.order_by.is_none() {
                    return Err(self.error_with(
                        "WITH TIES cannot be specified without ORDER BY clause",
                        limits.position,
                    ));
                }
                select.locks.extend(locking.locks);
                if with_ties
                    && (locking.skip_locked || select.locks.iter().any(|lock| lock.ignore_locked))
                {
                    return Err(self.error_with(
                        "SKIP LOCKED and WITH TIES options cannot be used together",
                        limits.position,
                    ));
                }
                if let Some(with) = with {
                    if select.with.is_some() {
                        return Err(self.error_with("multiple WITH clauses not allowed", start));
                    }
                    select.with = Some(with);
                }
            }
            Expression::Union(node) => set_operation_options!(node),
            Expression::Intersect(node) => set_operation_options!(node),
            Expression::Except(node) => set_operation_options!(node),
            _ => return Err(self.error()),
        }
        Ok(())
    }

    /// `select_limit`: at most one `LIMIT`/`FETCH` and one `OFFSET`, in either order.
    fn limits(&mut self) -> Parsing<Limits> {
        let mut limits: Limits = Limits {
            position: self.offset(),
            ..Limits::default()
        };
        loop {
            let counted: bool = limits.limit.is_some() || limits.fetch.is_some();
            if self.kw("limit") && !counted {
                let position: usize = self.offset();
                self.at += 1;
                let count: Expression = match self.eat_kw("all") {
                    true => null(),
                    false => self.a_expr()?,
                };
                if self.is_char(',') {
                    return Err(self.error_with("LIMIT #,# syntax is not supported", position));
                }
                limits.limit = Some(count);
            } else if self.kw("fetch") && !counted {
                self.at += 1;
                limits.fetch = Some(self.fetch()?);
            } else if self.kw("offset") && limits.offset.is_none() {
                self.at += 1;
                limits.offset = Some(self.offset_clause()?);
            } else {
                return Ok(limits);
            }
        }
    }

    /// `FETCH FIRST|NEXT [count] ROW|ROWS ONLY|WITH TIES`, after `FETCH`.
    fn fetch(&mut self) -> Parsing<Fetch> {
        let direction: String = match self.keyword_at(0) {
            Some(word @ ("first" | "next")) => word.to_uppercase(),
            _ => return Err(self.error()),
        };
        self.at += 1;
        let rows_now: bool = matches!(self.keyword_at(0), Some("row" | "rows"))
            && (self.kw_at(1, "only") || self.kw_at(1, "with"));
        let count: Option<Expression> = match rows_now {
            true => None,
            false => Some(self.fetch_value()?),
        };
        let rows: bool = match self.keyword_at(0) {
            Some(word @ ("row" | "rows")) => {
                self.at += 1;
                word == "rows"
            }
            _ => return Err(self.error()),
        };
        let with_ties: bool = match self.eat_kw("only") {
            true => false,
            false => {
                self.expect_kw("with")?;
                self.expect_kw("ties")?;
                true
            }
        };
        Ok(Fetch {
            direction,
            count,
            percent: false,
            rows,
            with_ties,
        })
    }

    /// `select_fetch_first_value`: `c_expr`, or a signed numeric constant.
    fn fetch_value(&mut self) -> Parsing<Expression> {
        let sign: Option<char> = match self.peek().map(|token| token.kind) {
            Some(TokenKind::Char(sign @ ('+' | '-'))) => Some(sign),
            _ => None,
        };
        let Some(sign) = sign else {
            return self.c_expr(crate::parse::_helpers::expressions::Flavor::B);
        };
        self.at += 1;
        let number = self.advance()?;
        if !matches!(number.kind, TokenKind::Integer | TokenKind::Numeric) {
            self.at -= 1;
            return Err(self.error());
        }
        let value: Expression = literal(Literal::Number(number.value));
        Ok(match sign {
            '-' => crate::parse::_helpers::expressions::binary_node(
                "-",
                literal(Literal::Number("0".to_owned())),
                value,
            ),
            _ => value,
        })
    }

    /// `offset_clause`: `OFFSET a_expr`, or `OFFSET value ROW|ROWS`, after `OFFSET`.
    fn offset_clause(&mut self) -> Parsing<Offset> {
        let rows = self.attempt(|parser| {
            let value: Expression = parser.fetch_value()?;
            match parser.eat_kw("row") || parser.eat_kw("rows") {
                true => Ok(value),
                false => Err(parser.error()),
            }
        });
        Ok(match rows {
            Ok(this) => Offset {
                this,
                rows: Some(true),
            },
            Err(_) => Offset {
                this: self.a_expr()?,
                rows: None,
            },
        })
    }

    /// `for_locking_clause`: `FOR READ ONLY`, or one or more `FOR ... [OF ...] [NOWAIT|SKIP
    /// LOCKED]`.
    fn locking(&mut self) -> Parsing<Locking> {
        let mut locking: Locking = Locking {
            locks: Vec::new(),
            skip_locked: false,
        };
        if self.kw("for") && self.kw_at(1, "read") && self.kw_at(2, "only") {
            self.at += 3;
            return Ok(locking);
        }
        while self.eat_kw("for") {
            let (update, key) = if self.eat_kw("update") {
                (true, false)
            } else if self.eat_kw("share") {
                (false, false)
            } else if self.eat_kw("no") {
                self.expect_kw("key")?;
                self.expect_kw("update")?;
                (true, true)
            } else {
                self.expect_kw("key")?;
                self.expect_kw("share")?;
                (false, true)
            };
            let expressions: Vec<Expression> = match self.eat_kw("of") {
                true => self.comma_list(|parser| {
                    Ok(Expression::Table(Box::new(table(parser.table_name()?))))
                })?,
                false => Vec::new(),
            };
            let nowait: bool = self.eat_kw("nowait");
            let skip: bool = !nowait && self.kw("skip") && self.kw_at(1, "locked");
            if skip {
                self.at += 2;
                locking.skip_locked = true;
            }
            locking.locks.push(Lock {
                ignore_locked: skip,
                update: Some(boolean_box(update)),
                expressions,
                wait: nowait.then(|| boolean_box(false)),
                key: key.then(|| boolean_box(true)),
            });
        }
        Ok(locking)
    }

    /// `qualified_name`, at most `catalog.schema.name`.
    pub(crate) fn table_name(&mut self) -> Parsing<Vec<Identifier>> {
        let start: usize = self.offset();
        let name: Vec<Identifier> = self.qualified_name()?;
        if name.len() > 3 {
            let text: String = name
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<&str>>()
                .join(".");
            return Err(self.error_with(
                &format!("improper qualified name (too many dotted names): {text}"),
                start,
            ));
        }
        Ok(name)
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
