//! `SelectStatement` (select.gram): CTEs, set operations, `SimpleSelect` and result modifiers.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::queries::models::{
    Cte, Fetch, From, GroupBy, Having, Limit, NamedWindow, Offset, OrderBy, OrderByAll, Ordered,
    Qualify, Select, Where, With,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    call, cte, literal, set_operation, star, subquery, tuple, var,
};
use crate::parse::constants::{CUBE, GROUPING_SETS, ROLLUP};
use crate::scan::models::TokenKind;

/// The transformer's refusals of a lone describe with a CTE or result modifiers.
const DESCRIBE_WITH_CTE: &str = "with CTE not allowed - wrap the statement in a subquery instead";
const DESCRIBE_WITH_ORDER: &str =
    "with ORDER BY not allowed - wrap the statement in a subquery instead";

/// The statements a CTE body may be besides a query, and the transformer's refusal of others.
const CTE_STATEMENTS: [&str; 6] = ["WITH", "INSERT", "UPDATE", "DELETE", "TRUNCATE", "COPY"];
const CTE_BODY: &str = "A CTE body must be a SELECT, INSERT, UPDATE, DELETE, or COPY TO statement";
const CTE_COPY_FROM: &str = "COPY FROM cannot be used as a CTE body";
const RECURSIVE_DML: &str = "Recursive CTEs with DML statements are not supported";
const RECURSIVE_COPY: &str = "Recursive CTEs with COPY statements are not supported";
/// The row count of a `FETCH` without one.
const ONE_ROW: &str = "1";
/// The transformer's refusal of `EXCEPT [ALL] BY NAME`.
const EXCEPT_OPERATOR: &str = "EXCEPT";
const EXCEPT_BY_NAME: &str = "Invalid combination of EXCEPT and BY NAME";

/// The result modifiers after a set-operation chain.
#[derive(Default)]
struct Modifiers {
    order: Option<Vec<Ordered>>,
    limit: Option<Limit>,
    offset: Option<Offset>,
    fetch: Option<Fetch>,
}

impl Modifiers {
    fn is_empty(&self) -> bool {
        self.order.is_none()
            && self.limit.is_none()
            && self.offset.is_none()
            && self.fetch.is_none()
    }
}

/// `ORDER BY items`; `ORDER BY ALL` is its own node, without items.
fn order_by(expressions: Vec<Ordered>) -> OrderBy {
    let all: Option<OrderByAll> = match expressions.as_slice() {
        [item] if matches!(&item.this, Expression::Var(var) if var.this == "ALL") => {
            Some(OrderByAll {
                desc: item.desc,
                nulls_first: item.nulls_first,
                explicit_asc: item.explicit_asc,
            })
        }
        _ => None,
    };
    let expressions: Vec<Ordered> = match all {
        Some(_) => Vec::new(),
        None => expressions,
    };
    OrderBy {
        expressions,
        siblings: false,
        comments: Vec::new(),
        all,
    }
}

/// `SELECT * FROM (body)`, for a body that cannot take a clause itself.
pub(crate) fn wrapped(body: Expression) -> Select {
    let mut select: Select = Select::new();
    select.expressions = vec![Expression::Star(star(None, None))];
    select.from = Some(From {
        expressions: vec![subquery(body, None, false)],
    });
    select
}

/// `body` with `modifiers` applied, wrapped first when it already has one of them.
fn modified(body: Expression, modifiers: Modifiers) -> Expression {
    if modifiers.is_empty() {
        return body;
    }
    let taken: bool = match &body {
        Expression::Select(select) => {
            select.order_by.is_some()
                || select.limit.is_some()
                || select.offset.is_some()
                || select.fetch.is_some()
        }
        Expression::Union(node) => {
            node.order_by.is_some() || node.limit.is_some() || node.offset.is_some()
        }
        Expression::Intersect(node) => {
            node.order_by.is_some() || node.limit.is_some() || node.offset.is_some()
        }
        Expression::Except(node) => {
            node.order_by.is_some() || node.limit.is_some() || node.offset.is_some()
        }
        _ => true,
    };
    let mut body: Expression = match taken {
        true => Expression::Select(Box::new(wrapped(body))),
        false => body,
    };
    let Modifiers {
        order,
        limit,
        offset,
        fetch,
    } = modifiers;
    macro_rules! set_operation_modified {
        ($node:expr) => {{
            $node.order_by = order.map(order_by);
            $node.limit = limit.map(set_operation_limit);
            $node.offset = offset.map(|offset| Box::new(offset.this));
            if let Some(fetch) = fetch {
                $node.limit = Some(Box::new(fetch.count.unwrap_or_else(literal_one)));
            }
        }};
    }
    match &mut body {
        Expression::Select(select) => {
            select.order_by = order.map(order_by);
            select.limit = limit;
            select.offset = offset;
            select.fetch = fetch;
        }
        Expression::Union(node) => set_operation_modified!(node),
        Expression::Intersect(node) => set_operation_modified!(node),
        Expression::Except(node) => set_operation_modified!(node),
        _ => {}
    }
    body
}

/// A set operation's limit: its value, or the whole node when it is a percentage.
fn set_operation_limit(limit: Limit) -> Box<Expression> {
    match limit.percent {
        true => Box::new(Expression::Limit(Box::new(limit))),
        false => Box::new(limit.this),
    }
}

/// The limit of a `FETCH` without a count, which DuckDB reads as one row.
fn literal_one() -> Expression {
    literal(Literal::Number(ONE_ROW.to_owned()))
}

/// `body` with `with` as its common table expressions, wrapped when it has its own already.
pub(crate) fn with_ctes(body: Expression, with: With) -> Expression {
    let has_with: bool = match &body {
        Expression::Select(select) => select.with.is_some(),
        Expression::Union(node) => node.with.is_some(),
        Expression::Intersect(node) => node.with.is_some(),
        Expression::Except(node) => node.with.is_some(),
        _ => true,
    };
    let mut body: Expression = match has_with {
        true => Expression::Select(Box::new(wrapped(body))),
        false => body,
    };
    match &mut body {
        Expression::Select(select) => select.with = Some(with),
        Expression::Union(node) => node.with = Some(with),
        Expression::Intersect(node) => node.with = Some(with),
        Expression::Except(node) => node.with = Some(with),
        _ => {}
    }
    body
}

impl Parser<'_> {
    /// `SelectStatementInternal`: `[WithClause] SelectSetOpChain [ResultModifiers]`.
    pub(crate) fn query(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            let first: usize = parser.at;
            let with: Option<With> = match parser.kw("WITH") {
                true => Some(parser.ctes()?),
                false => None,
            };
            let query: Expression = parser.query_rest(with)?;
            Ok(parser.respanned(first, query))
        })
    }

    /// A query after its common table expressions.
    pub(crate) fn query_rest(&mut self, with: Option<With>) -> Parsing<Expression> {
        let (first, start): (usize, usize) = (self.at, self.offset());
        let body: Expression = self.set_operation_chain()?;
        let modifiers: Modifiers = self.result_modifiers()?;
        let lone: bool = !matches!(
            body,
            Expression::Union(_) | Expression::Intersect(_) | Expression::Except(_)
        );
        if let Some((_, label)) = self.describe.filter(|(atom, _)| lone && *atom == first) {
            if with.is_some() {
                self.refuse_later(&format!("{label} {DESCRIBE_WITH_CTE}"), start);
            } else if !modifiers.is_empty() {
                self.refuse_later(&format!("{label} {DESCRIBE_WITH_ORDER}"), start);
            }
        }
        let body: Expression = modified(body, modifiers);
        Ok(match with {
            Some(with) => with_ctes(body, with),
            None => body,
        })
    }

    /// `WithClause`: `WITH [RECURSIVE] WithStatement, ...`.
    pub(crate) fn ctes(&mut self) -> Parsing<With> {
        self.expect_kw("WITH")?;
        let recursive: bool = self.eat_kw("RECURSIVE");
        let ctes: Vec<Cte> = self.comma_list(|parser| {
            let first: usize = parser.at;
            let start: usize = parser.offset();
            let mut node: Cte = parser.with_statement()?;
            node.span = parser.source_range(first);
            let refusal: Option<&str> = match &node.this {
                Expression::Copy(_) => Some(RECURSIVE_COPY),
                Expression::Insert(_)
                | Expression::Update(_)
                | Expression::Delete(_)
                | Expression::Truncate(_) => Some(RECURSIVE_DML),
                _ => None,
            };
            if let Some(message) = refusal.filter(|_| recursive) {
                parser.refuse_later(message, start);
            }
            Ok(node)
        })?;
        Ok(With {
            ctes,
            recursive,
            leading_comments: Vec::new(),
            search: None,
        })
    }

    /// `WithStatement`: `name [(columns)] [USING KEY (...)] AS [[NOT] MATERIALIZED] (body)`.
    fn with_statement(&mut self) -> Parsing<Cte> {
        let name: Identifier = self.col_id_or_string()?;
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.col_id_or_string())?,
            false => Vec::new(),
        };
        let mut keys: Vec<Identifier> = Vec::new();
        let mut key_items: Vec<Expression> = Vec::new();
        if self.kw("USING") && self.kw_at(1, "KEY") {
            self.at += 2;
            let items: Vec<Expression> = self.parenthesized(|parser| parser.named_expression())?;
            let bare: bool = items
                .iter()
                .all(|item| matches!(item, Expression::Column(column) if column.table.is_none()));
            match bare {
                true => {
                    for item in items {
                        if let Expression::Column(column) = item {
                            keys.push(column.name);
                        }
                    }
                }
                false => key_items = items,
            }
        }
        self.expect_kw("AS")?;
        let materialized: Option<bool> = if self.eat_kws(&["NOT", "MATERIALIZED"]) {
            Some(false)
        } else if self.eat_kw("MATERIALIZED") {
            Some(true)
        } else {
            None
        };
        self.expect_char('(')?;
        let start: usize = self.offset();
        let body: Expression = match self.starts_query_at(0) && !self.kw("WITH") {
            true => self.query()?,
            false => {
                let word: String = self.word_at(0).unwrap_or_default();
                let database: bool =
                    self.kw("COPY") && self.kw_at(1, "FROM") && self.kw_at(2, "DATABASE");
                if database || !CTE_STATEMENTS.contains(&word.as_str()) {
                    self.refuse_later(CTE_BODY, start);
                }
                let body: Expression = self.statement()?;
                if matches!(&body, Expression::Merge(_)) {
                    self.refuse_later(CTE_BODY, start);
                }
                if matches!(&body, Expression::Copy(copy) if copy.kind) {
                    self.refuse_later(CTE_COPY_FROM, start);
                }
                body
            }
        };
        self.expect_char(')')?;
        let mut node: Cte = cte(name, columns, body);
        node.materialized = materialized;
        node.key_expressions = keys;
        node.key_items = key_items;
        Ok(node)
    }

    /// `SelectSetOpChain`: `UNION` and `EXCEPT` over `INTERSECT` chains, left-associative.
    fn set_operation_chain(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut left: Expression = self.intersect_chain()?;
        while self.kw_in(0, &["UNION", "EXCEPT"]) {
            let operator: String = self.word_at(0).unwrap_or_default();
            self.at += 1;
            let quantifier: (bool, bool) = self.distinct_or_all();
            let by_name_start: usize = self.offset();
            let by_name: bool = self.eat_kws(&["BY", "NAME"]);
            if by_name && operator == EXCEPT_OPERATOR {
                self.refuse_later(EXCEPT_BY_NAME, by_name_start);
            }
            let right: Expression = self.deeper(|parser| parser.intersect_chain())?;
            let mut node: Expression = set_operation(&operator, left, right, quantifier);
            if let Expression::Union(union) = &mut node {
                union.by_name = by_name;
            }
            left = self.spanned(first, node);
        }
        Ok(left)
    }

    /// `DistinctOrAll?` after a set operator: (`ALL`, `DISTINCT`).
    fn distinct_or_all(&mut self) -> (bool, bool) {
        let all: bool = self.eat_kw("ALL");
        let distinct: bool = !all && self.eat_kw("DISTINCT");
        (all, distinct)
    }

    /// `IntersectChain`.
    fn intersect_chain(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut left: Expression = self.select_atom()?;
        while self.eat_kw("INTERSECT") {
            let quantifier: (bool, bool) = self.distinct_or_all();
            let right: Expression = self.deeper(|parser| parser.select_atom())?;
            left = self.spanned(first, set_operation("INTERSECT", left, right, quantifier));
        }
        Ok(left)
    }

    /// `SelectAtom`: a parenthesized query or one `SelectStatementType`.
    fn select_atom(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        self.describe = None;
        if self.is_char('(') {
            self.at += 1;
            let query: Expression = self.query()?;
            self.expect_char(')')?;
            self.describe = self
                .describe
                .filter(|(atom, _)| *atom == first + 1)
                .map(|(_, label)| (first, label));
            return Ok(query);
        }
        match self.word_at(0).as_deref() {
            Some("DESCRIBE" | "DESC" | "SUMMARIZE" | "SHOW") => {
                let (node, label) = self.describe_statement()?;
                self.describe = Some((first, label));
                Ok(node)
            }
            Some("SELECT" | "FROM") => self.simple_select(),
            Some("PIVOT" | "PIVOT_WIDER") => self.pivot_statement(),
            Some("UNPIVOT" | "PIVOT_LONGER") => self.unpivot_statement(),
            Some("VALUES") => {
                let rows = self.values_rows()?;
                Ok(crate::parse::_helpers::tree::nodes::values(rows, None))
            }
            Some("TABLE") => {
                self.at += 1;
                let mut select: Select = Select::new();
                select.expressions = vec![Expression::Star(star(None, None))];
                select.from = Some(From {
                    expressions: vec![Expression::Table(Box::new(self.base_table_name()?))],
                });
                Ok(Expression::Select(Box::new(select)))
            }
            _ => Err(self.error()),
        }
    }

    /// `ValuesClause`'s rows: `VALUES (a, b), ...`.
    pub(crate) fn values_rows(&mut self) -> Parsing<Vec<Vec<Expression>>> {
        self.expect_kw("VALUES")?;
        let mut rows: Vec<Vec<Expression>> = Vec::new();
        loop {
            self.expect_char('(')?;
            rows.push(self.expression_list(')')?);
            self.expect_char(')')?;
            if !self.eat_char(',') || !self.is_char('(') {
                return Ok(rows);
            }
        }
    }

    /// `SimpleSelect`: `SELECT ... [FROM ...]` or `FROM ... [SELECT ...]`, then its clauses.
    fn simple_select(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut select: Select = Select::new();
        if self.kw("SELECT") {
            select = self.select_clause(select)?;
            if self.kw("FROM") {
                select = self.select_from_clause(select)?;
            }
        } else {
            select = self.select_from_clause(select)?;
            match self.kw("SELECT") {
                true => select = self.select_clause(select)?,
                false => select.expressions = vec![Expression::Star(star(None, None))],
            }
        }
        if self.eat_kw("WHERE") {
            select.where_clause = Some(Where {
                this: self.expression()?,
            });
        }
        if self.kw("GROUP") && self.kw_at(1, "BY") {
            select.group_by = Some(self.group_by()?);
        }
        if self.eat_kw("HAVING") {
            select.having = Some(Having {
                this: self.expression()?,
                comments: Vec::new(),
            });
        }
        if self.kw("WINDOW") {
            select.windows = Some(self.window_clause()?);
        }
        if self.eat_kw("QUALIFY") {
            select.qualify = Some(Qualify {
                this: self.expression()?,
            });
        }
        if self.kw("USING") && self.kw_at(1, "SAMPLE") {
            self.at += 2;
            let mut sample = self.sample_entry()?;
            sample.is_using_sample = true;
            select.sample = Some(sample);
        }
        Ok(self.spanned(first, Expression::Select(Box::new(select))))
    }

    /// `SelectClause`: `SELECT [DISTINCT [ON (...)] | ALL] [TargetList]`.
    fn select_clause(&mut self, mut select: Select) -> Parsing<Select> {
        self.expect_kw("SELECT")?;
        if self.eat_kw("DISTINCT") {
            select.distinct = true;
            if self.kw("ON") && self.char_at(1, '(') {
                self.at += 2;
                select.distinct_on = Some(self.expression_list(')')?);
                self.expect_char(')')?;
            }
        } else if self.eat_kw("ALL") {
            select.all = true;
        }
        if self.target_ends() {
            let position: usize = self.offset();
            self.refuse_later("SELECT clause without selection list", position);
            return Ok(select);
        }
        let mut expressions: Vec<Expression> = vec![self.named_expression()?];
        while self.eat_char(',') {
            if self.target_ends() {
                break;
            }
            expressions.push(self.named_expression()?);
        }
        select.expressions = expressions;
        Ok(select)
    }

    /// Whether the target list ends here: at the end, `)`, `;` or a clause keyword.
    fn target_ends(&self) -> bool {
        self.terminal_at(0)
            || self.kw_in(
                0,
                &[
                    "FROM",
                    "WHERE",
                    "GROUP",
                    "HAVING",
                    "WINDOW",
                    "QUALIFY",
                    "ORDER",
                    "LIMIT",
                    "OFFSET",
                    "FETCH",
                    "UNION",
                    "EXCEPT",
                    "INTERSECT",
                    "USING",
                ],
            )
    }

    /// `WindowClause`: `WINDOW name AS (spec), ...`.
    fn window_clause(&mut self) -> Parsing<Vec<NamedWindow>> {
        self.expect_kw("WINDOW")?;
        self.comma_list(|parser| {
            let name: Identifier = parser.name(is_identifier)?;
            parser.expect_kw("AS")?;
            Ok(NamedWindow {
                name,
                spec: parser.window_spec()?,
            })
        })
    }

    /// `GroupByClause`: `GROUP BY ALL` or a list of grouping items.
    fn group_by(&mut self) -> Parsing<GroupBy> {
        self.at += 2;
        if self.eat_kw("ALL") {
            return Ok(GroupBy {
                expressions: Vec::new(),
                all: Some(true),
                totals: false,
                comments: Vec::new(),
            });
        }
        let expressions: Vec<Expression> = self.comma_list(|parser| parser.group_item())?;
        Ok(GroupBy {
            expressions,
            all: None,
            totals: false,
            comments: Vec::new(),
        })
    }

    /// `GroupByExpression`: `()`, `CUBE|ROLLUP (...)`, `GROUPING SETS (...)` or an expression.
    fn group_item(&mut self) -> Parsing<Expression> {
        if self.is_char('(') && self.char_at(1, ')') {
            self.at += 2;
            return Ok(tuple(Vec::new()));
        }
        if self.kw_in(0, &[ROLLUP, CUBE]) && self.char_at(1, '(') {
            let name: String = self.word_at(0).unwrap_or_default();
            self.at += 2;
            if self.is_char(')') {
                return Err(self.error_with(
                    "CUBE or ROLLUP column list cannot be empty",
                    self.offset(),
                    false,
                ));
            }
            let items: Vec<Expression> = self.expression_list(')')?;
            self.expect_char(')')?;
            return Ok(call(&name, items));
        }
        if self.kw("GROUPING") && self.kw_at(1, "SETS") && self.char_at(2, '(') {
            self.at += 2;
            let items: Vec<Expression> = self.parenthesized(|parser| parser.group_item())?;
            return Ok(call(GROUPING_SETS, items));
        }
        self.expression()
    }

    /// `ResultModifiers`: `ORDER BY` and one of the `LIMIT`/`OFFSET`/`FETCH` orders, never `LIMIT` with `FETCH`.
    fn result_modifiers(&mut self) -> Parsing<Modifiers> {
        let mut modifiers: Modifiers = Modifiers::default();
        if self.order_by_at() {
            modifiers.order = Some(self.order_by_clause()?);
        }
        for _ in 0..2 {
            let counted: bool = modifiers.limit.is_some() || modifiers.fetch.is_some();
            if !counted && self.eat_kw("LIMIT") {
                modifiers.limit = Some(self.limit_value()?);
            } else if modifiers.offset.is_none() && self.eat_kw("OFFSET") {
                let this: Expression = self.expression()?;
                let rows: Option<bool> = match self.eat_kw("ROWS") || self.eat_kw("ROW") {
                    true => Some(true),
                    false => None,
                };
                modifiers.offset = Some(Offset { this, rows });
            } else if !counted && self.kw("FETCH") {
                modifiers.fetch = Some(self.fetch_clause()?);
            }
        }
        Ok(modifiers)
    }

    /// `LimitValue`: `ALL`, a number then `PERCENT`, or an expression with an optional `%`.
    fn limit_value(&mut self) -> Parsing<Limit> {
        if self.eat_kw("ALL") {
            return Ok(Limit {
                this: var("ALL"),
                percent: false,
                comments: Vec::new(),
            });
        }
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Number)
            && self.kw_at(1, "PERCENT")
        {
            let this: Expression = self.primary_expression()?;
            self.at += 1;
            return Ok(Limit {
                this,
                percent: true,
                comments: Vec::new(),
            });
        }
        let this: Expression = self.expression()?;
        let percent: bool = self.eat_char('%');
        Ok(Limit {
            this,
            percent,
            comments: Vec::new(),
        })
    }

    /// `FetchClause`: `FETCH FIRST|NEXT [n] ROW|ROWS ONLY`.
    fn fetch_clause(&mut self) -> Parsing<Fetch> {
        self.expect_kw("FETCH")?;
        let direction: String = match self.word_at(0).as_deref() {
            Some(direction @ ("FIRST" | "NEXT")) => direction.to_owned(),
            _ => return Err(self.error()),
        };
        self.at += 1;
        let count: Option<Expression> = match self.kw_in(0, &["ROW", "ROWS"]) {
            true => None,
            false => Some(self.expression()?),
        };
        if !(self.eat_kw("ROWS") || self.eat_kw("ROW")) {
            return Err(self.error());
        }
        self.expect_kw("ONLY")?;
        Ok(Fetch {
            direction,
            count,
            percent: false,
            rows: true,
            with_ties: false,
        })
    }
}
