//! `query`, its terms, primaries, specification and organization, and the pipe operators.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    ClusterBy, Cte, DistributeBy, From, GroupBy, Having, Hint, HintExpression, Limit, NamedWindow,
    OrderBy, Ordered, Qualify, Select, SortBy, Where, With,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    TableAlias, binary, call, cte, null, set_operation, star, subquery, tuple,
};
use crate::parse::constants::{CUBE, GROUPING_SETS, ROLLUP};
use crate::scan::models::TokenKind;

/// `queryOrganization`: the clauses after a query term.
#[derive(Default)]
pub(crate) struct Organization {
    order: Option<Vec<Ordered>>,
    cluster: Option<Vec<Expression>>,
    distribute: Option<Vec<Expression>>,
    sort: Option<Vec<Ordered>>,
    windows: Option<Vec<NamedWindow>>,
    limit: Option<Expression>,
    offset: Option<Expression>,
}

impl Organization {
    fn is_empty(&self) -> bool {
        self.order.is_none()
            && self.cluster.is_none()
            && self.distribute.is_none()
            && self.sort.is_none()
            && self.windows.is_none()
            && self.limit.is_none()
            && self.offset.is_none()
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

/// `SELECT * FROM (body)`, for a body that cannot take a clause itself.
pub(crate) fn wrapped(body: Expression) -> Select {
    let mut select: Select = Select::new();
    select.expressions = vec![Expression::Star(star(None, None))];
    select.from = Some(From {
        expressions: vec![subquery(body, None, false)],
    });
    select
}

/// Whether `select` is `SELECT * FROM ...` with nothing but its sources, joins and `WHERE`.
fn is_plain_star(select: &Select) -> bool {
    matches!(
        select.expressions.as_slice(),
        [Expression::Star(star)] if star.table.is_none() && star.except.is_none() && star.replace.is_none()
    ) && !select.distinct
        && select.from.is_some()
        && select.group_by.is_none()
        && select.having.is_none()
        && select.qualify.is_none()
        && select.windows.is_none()
        && select.order_by.is_none()
        && select.cluster_by.is_none()
        && select.distribute_by.is_none()
        && select.sort_by.is_none()
        && select.limit.is_none()
        && select.offset.is_none()
        && select.sample.is_none()
}

/// The select a pipe operator extends: `left` itself, or else `SELECT * FROM (left)`.
fn pipe_input(left: Expression, bare: bool) -> Select {
    match left {
        Expression::Select(select)
            if is_plain_star(&select) && !(bare && select.where_clause.is_some()) =>
        {
            *select
        }
        other => wrapped(other),
    }
}

/// Whether `body` can take every clause of `organization` without one it already has.
fn takes(body: &Expression, organization: &Organization) -> bool {
    macro_rules! set_operation_takes {
        ($node:expr) => {
            organization.windows.is_none()
                && !(organization.order.is_some() && $node.order_by.is_some())
                && !(organization.cluster.is_some() && $node.cluster_by.is_some())
                && !(organization.distribute.is_some() && $node.distribute_by.is_some())
                && !(organization.sort.is_some() && $node.sort_by.is_some())
                && $node.limit.is_none()
                && $node.offset.is_none()
        };
    }
    match body {
        Expression::Select(select) => {
            !(organization.order.is_some() && select.order_by.is_some())
                && !(organization.cluster.is_some() && select.cluster_by.is_some())
                && !(organization.distribute.is_some() && select.distribute_by.is_some())
                && !(organization.sort.is_some() && select.sort_by.is_some())
                && !(organization.windows.is_some() && select.windows.is_some())
                && select.limit.is_none()
                && select.offset.is_none()
        }
        Expression::Union(node) => set_operation_takes!(node),
        Expression::Intersect(node) => set_operation_takes!(node),
        Expression::Except(node) => set_operation_takes!(node),
        _ => false,
    }
}

/// `body` with `organization` applied, wrapped first when it cannot take it as it is.
fn organized(body: Expression, organization: Organization) -> Expression {
    if organization.is_empty() {
        return body;
    }
    let mut body: Expression = match takes(&body, &organization) {
        true => body,
        false => Expression::Select(Box::new(wrapped(body))),
    };
    let Organization {
        order,
        cluster,
        distribute,
        sort,
        windows,
        limit: count,
        offset,
    } = organization;
    let cluster = cluster.map(|expressions| ClusterBy {
        expressions: expressions.into_iter().map(Ordered::asc).collect(),
    });
    let distribute = distribute.map(|expressions| DistributeBy { expressions });
    let sort = sort.map(|expressions| SortBy { expressions });
    macro_rules! set_operation_organized {
        ($node:expr) => {{
            if order.is_some() {
                $node.order_by = order.map(order_by);
            }
            $node.cluster_by = cluster.or($node.cluster_by.take());
            $node.distribute_by = distribute.or($node.distribute_by.take());
            $node.sort_by = sort.or($node.sort_by.take());
            $node.limit = count.map(Box::new);
            $node.offset = offset.map(Box::new);
        }};
    }
    match &mut body {
        Expression::Select(select) => {
            if order.is_some() {
                select.order_by = order.map(order_by);
            }
            select.cluster_by = cluster.or(select.cluster_by.take());
            select.distribute_by = distribute.or(select.distribute_by.take());
            select.sort_by = sort.or(select.sort_by.take());
            select.windows = windows.or(select.windows.take());
            select.limit = count.map(limit);
            select.offset =
                offset.map(
                    |this| sqlquack_syntax::expressions::queries::models::Offset {
                        this,
                        rows: None,
                    },
                );
        }
        Expression::Union(node) => set_operation_organized!(node),
        Expression::Intersect(node) => set_operation_organized!(node),
        Expression::Except(node) => set_operation_organized!(node),
        _ => {}
    }
    body
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
    /// `query`: `[ctes] queryTerm queryOrganization`.
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
        let body: Expression = self.query_term(0)?;
        let organization: Organization = self.organization()?;
        let body: Expression = organized(body, organization);
        Ok(match with {
            Some(with) => with_ctes(body, with),
            None => body,
        })
    }

    /// `ctes`: `WITH [RECURSIVE] namedQuery, ...`.
    pub(crate) fn ctes(&mut self) -> Parsing<With> {
        self.expect_kw("WITH")?;
        let recursive: bool = self.eat_kw("RECURSIVE");
        let ctes: Vec<Cte> = self.comma_list(|parser| {
            let first: usize = parser.at;
            let mut node: Cte = parser.named_query()?;
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

    /// `namedQuery`: `name [(columns)] [MAX RECURSION LEVEL n] [AS] (query)`.
    fn named_query(&mut self) -> Parsing<Cte> {
        let name: Identifier = self.ident()?;
        let columns: Vec<Identifier> = match self.is_char('(') && !self.starts_query_at(1) {
            true => self.identifier_list()?,
            false => Vec::new(),
        };
        if self.eat_kws(&["MAX", "RECURSION", "LEVEL"]) {
            self.expect_token(|token| {
                token.kind == TokenKind::Number(crate::scan::models::NumberKind::Integer)
            })?;
        }
        self.eat_kw("AS");
        self.expect_char('(')?;
        let body: Expression = self.query()?;
        self.expect_char(')')?;
        Ok(cte(name, columns, body))
    }

    /// `queryTerm` of at least precedence `min`, every operator left-associative.
    fn query_term(&mut self, min: u8) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut left: Expression = self.query_primary()?;
        loop {
            if self.is_op("|>") || self.pipe_start_at(0) {
                if min > 1 {
                    return Ok(left);
                }
                self.at += 1;
                left = self.deeper(|parser| parser.pipe(left))?;
                continue;
            }
            let operator: String = match self.word_at(0) {
                Some(word)
                    if ["UNION", "EXCEPT", "MINUS", "INTERSECT"].contains(&word.as_str()) =>
                {
                    word
                }
                _ => return Ok(left),
            };
            let level: u8 = match operator.as_str() {
                "INTERSECT" => 3,
                _ => 2,
            };
            if level < min {
                return Ok(left);
            }
            self.at += 1;
            let all: bool = self.eat_kw("ALL");
            let distinct: bool = !all && self.eat_kw("DISTINCT");
            let right: Expression = self.deeper(|parser| parser.query_term(level + 1))?;
            left = self.spanned(
                first,
                set_operation(&operator, left, right, (all, distinct)),
            );
        }
    }

    /// `queryPrimary`: a `SELECT`, `FROM ...`, `TABLE t`, `VALUES ...` or `( query )`.
    pub(crate) fn query_primary(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            self.at += 1;
            let query: Expression = self.query()?;
            self.expect_char(')')?;
            return Ok(query);
        }
        match self.word_at(0).as_deref() {
            Some("SELECT") => self.query_specification(),
            Some("FROM") => self.leading_from_query(),
            Some("TABLE") => {
                self.at += 1;
                let item: Expression = self.table_identifier_reference()?;
                let mut select: Select = Select::new();
                select.expressions = vec![Expression::Star(star(None, None))];
                select.from = Some(From {
                    expressions: vec![item],
                });
                Ok(Expression::Select(Box::new(select)))
            }
            Some("VALUES") => self.inline_table(),
            _ => Err(self.error()),
        }
    }

    /// `querySpecification`'s `SELECT` form.
    fn query_specification(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut select: Select = self.select_clause(Select::new())?;
        if self.kw("FROM") {
            select = self.select_from_clause(select)?;
        }
        let select: Select = self.select_tail(select)?;
        Ok(self.spanned(first, Expression::Select(Box::new(select))))
    }

    /// `fromStatement`: `FROM relations` with one `SELECT` body or none (`SELECT *`).
    fn leading_from_query(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut select: Select = self.select_from_clause(Select::new())?;
        if self.kw("SELECT") {
            select = self.select_clause(select)?;
            select = self.select_tail(select)?;
            if self.kw("SELECT") {
                let start: usize = self.offset();
                while self.kw("SELECT") {
                    let more: Select = self.select_clause(Select::new())?;
                    self.select_tail(more)?;
                    self.organization()?;
                }
                self.gap("multi_select_from", start);
            }
        } else {
            select.expressions = vec![Expression::Star(star(None, None))];
        }
        Ok(self.spanned(first, Expression::Select(Box::new(select))))
    }

    /// `selectClause`: `SELECT hints [DISTINCT|ALL] namedExpressionSeq`.
    fn select_clause(&mut self, mut select: Select) -> Parsing<Select> {
        self.expect_kw("SELECT")?;
        if self.is_kind(TokenKind::HintStart) {
            select.hint = Some(self.hints()?);
        }
        if self.kw("DISTINCT") {
            self.at += 1;
            select.distinct = true;
        } else if self.kw("ALL") && !self.terminal_at(1) && !self.starts_clause(1) {
            self.at += 1;
            select.all = true;
        }
        select.expressions = self.comma_list(|parser| parser.named_expression())?;
        Ok(select)
    }

    /// One or more `/*+ ... */` hints.
    fn hints(&mut self) -> Parsing<Hint> {
        let mut expressions: Vec<HintExpression> = Vec::new();
        while self.is_kind(TokenKind::HintStart) {
            self.at += 1;
            loop {
                let name: Identifier = self.name(is_identifier)?;
                expressions.push(match self.is_char('(') {
                    true => HintExpression::Function {
                        name: name.name,
                        args: self.parenthesized(|parser| parser.primary_expression())?,
                    },
                    false => HintExpression::Identifier(name.name),
                });
                self.eat_char(',');
                if self.is_kind(TokenKind::HintEnd) {
                    break;
                }
            }
            self.at += 1;
        }
        Ok(Hint {
            source_dialect: None,
            expressions,
        })
    }

    /// Whether the current token is of `kind`.
    pub(crate) fn is_kind(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    /// The clauses after `FROM`: lateral views, `WHERE`, `GROUP BY`, `HAVING`, `WINDOW`, `QUALIFY`.
    fn select_tail(&mut self, mut select: Select) -> Parsing<Select> {
        select.lateral_views.extend(self.lateral_views()?);
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
        if self.kw("WINDOW") && self.is_at(1, is_identifier) {
            select.windows = Some(self.window_clause()?);
        }
        if self.eat_kw("QUALIFY") {
            select.qualify = Some(Qualify {
                this: self.expression()?,
            });
        }
        Ok(select)
    }

    /// `windowClause`: `WINDOW name AS windowSpec, ...`.
    fn window_clause(&mut self) -> Parsing<Vec<NamedWindow>> {
        self.expect_kw("WINDOW")?;
        self.comma_list(|parser| {
            let name: Identifier = parser.ident()?;
            parser.expect_kw("AS")?;
            Ok(NamedWindow {
                name,
                spec: parser.window_spec()?,
            })
        })
    }

    /// `aggregationClause`.
    fn group_by(&mut self) -> Parsing<GroupBy> {
        self.at += 2;
        if self.kw("ALL") && (self.terminal_at(1) || self.starts_clause(1)) {
            self.at += 1;
            return Ok(GroupBy {
                expressions: Vec::new(),
                all: Some(true),
                totals: false,
                comments: Vec::new(),
            });
        }
        let items: Vec<Expression> = self.comma_list(|parser| parser.group_item())?;
        let expressions: Vec<Expression> = if self.kw("WITH") && self.kw_in(1, &[ROLLUP, CUBE]) {
            let name: String = self.word_at(1).unwrap_or_default();
            self.at += 2;
            vec![call(&name, items)]
        } else if self.kw("GROUPING") && self.kw_at(1, "SETS") {
            self.at += 2;
            vec![call(
                GROUPING_SETS,
                self.parenthesized(|parser| parser.grouping_set())?,
            )]
        } else {
            items
        };
        Ok(GroupBy {
            expressions,
            all: None,
            totals: false,
            comments: Vec::new(),
        })
    }

    /// `groupByClause`: `groupingAnalytics` or an expression.
    fn group_item(&mut self) -> Parsing<Expression> {
        match self.grouping_analytics()? {
            Some(analytics) => Ok(analytics),
            None => self.expression(),
        }
    }

    /// `ROLLUP (...)`, `CUBE (...)` or `GROUPING SETS (...)`, as the calls the tree names them.
    fn grouping_analytics(&mut self) -> Parsing<Option<Expression>> {
        if self.kw_in(0, &[ROLLUP, CUBE]) && self.char_at(1, '(') {
            let name: String = self.word_at(0).unwrap_or_default();
            self.at += 1;
            let sets: Vec<Expression> = self.parenthesized(|parser| parser.grouping_set())?;
            return Ok(Some(call(&name, sets)));
        }
        if self.kw("GROUPING") && self.kw_at(1, "SETS") && self.char_at(2, '(') {
            self.at += 2;
            let elements: Vec<Expression> =
                self.parenthesized(|parser| match parser.grouping_analytics()? {
                    Some(analytics) => Ok(analytics),
                    None => parser.grouping_set(),
                })?;
            return Ok(Some(call(GROUPING_SETS, elements)));
        }
        Ok(None)
    }

    /// `groupingSet`: `( [expression, ...] )` or an expression.
    fn grouping_set(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let items: Vec<Expression> = match parser.is_char(')') {
                    true => Vec::new(),
                    false => parser.comma_list(|parser| parser.expression())?,
                };
                parser.expect_char(')')?;
                match parser.is_char(',') || parser.is_char(')') {
                    true => Ok(tuple(items)),
                    false => Err(parser.error()),
                }
            });
            if let Ok(set) = attempt {
                return Ok(set);
            }
        }
        self.expression()
    }

    /// `queryOrganization`.
    pub(crate) fn organization(&mut self) -> Parsing<Organization> {
        let mut organization: Organization = Organization::default();
        if self.kw("ORDER") && self.kw_at(1, "BY") {
            self.at += 2;
            organization.order = Some(self.comma_list(|parser| parser.sort_item())?);
        }
        if self.kw("CLUSTER") && self.kw_at(1, "BY") {
            self.at += 2;
            organization.cluster = Some(self.comma_list(|parser| parser.expression())?);
        }
        if self.kw("DISTRIBUTE") && self.kw_at(1, "BY") {
            self.at += 2;
            organization.distribute = Some(self.comma_list(|parser| parser.expression())?);
        }
        if self.kw("SORT") && self.kw_at(1, "BY") {
            self.at += 2;
            organization.sort = Some(self.comma_list(|parser| parser.sort_item())?);
        }
        if self.kw("WINDOW") && self.is_at(1, is_identifier) {
            organization.windows = Some(self.window_clause()?);
        }
        if self.eat_kw("LIMIT") {
            organization.limit = Some(match self.eat_kw("ALL") {
                true => null(),
                false => self.expression()?,
            });
        }
        if self.eat_kw("OFFSET") {
            organization.offset = Some(self.expression()?);
        }
        Ok(organization)
    }

    /// `operatorPipeRightSide` after `|>`, applied to `left` as the select it amounts to.
    fn pipe(&mut self, left: Expression) -> Parsing<Expression> {
        let word: String = self.word_at(0).unwrap_or_default();
        let select: Select = match word.as_str() {
            "SELECT" => {
                let mut select: Select = self.select_clause(pipe_input(left, false))?;
                if self.kw("GROUP") && self.kw_at(1, "BY") {
                    select.group_by = Some(self.group_by()?);
                }
                if self.kw("WINDOW") && self.is_at(1, is_identifier) {
                    select.windows = Some(self.window_clause()?);
                }
                select
            }
            "EXTEND" => {
                self.at += 1;
                let mut select: Select = pipe_input(left, false);
                select
                    .expressions
                    .extend(self.comma_list(|parser| parser.named_expression())?);
                select
            }
            "SET" => {
                self.at += 1;
                let replace: Vec<Alias> = self.comma_list(|parser| {
                    let name: Identifier = parser.ident()?;
                    parser.expect_char('=')?;
                    Ok(Alias::new(parser.expression()?, name))
                })?;
                let mut select: Select = pipe_input(left, false);
                select.expressions = vec![Expression::Star(
                    sqlquack_syntax::expressions::names::models::Star {
                        replace: Some(replace),
                        ..star(None, None)
                    },
                )];
                select
            }
            "DROP" => {
                self.at += 1;
                let except: Vec<Identifier> = self.name_parts_list()?;
                let mut select: Select = pipe_input(left, false);
                select.expressions = vec![Expression::Star(star(None, Some(except)))];
                select
            }
            "AS" => {
                self.at += 1;
                let name: Identifier = self.ident()?;
                let alias = TableAlias {
                    name,
                    columns: Vec::new(),
                    explicit: true,
                };
                let mut select: Select = Select::new();
                select.expressions = vec![Expression::Star(star(None, None))];
                select.from = Some(From {
                    expressions: vec![subquery(left, Some(alias), false)],
                });
                select
            }
            "WHERE" => {
                self.at += 1;
                let condition: Expression = self.expression()?;
                if self.kw("WINDOW") {
                    return Err(self.error_with(
                        "the WHERE pipe operator takes no WINDOW clause",
                        self.offset(),
                        false,
                    ));
                }
                let mut select: Select = pipe_input(left, false);
                select.where_clause = Some(Where {
                    this: match select.where_clause.take() {
                        Some(earlier) => binary(Expression::And, earlier.this, condition),
                        None => condition,
                    },
                });
                select
            }
            "PIVOT" | "UNPIVOT" => {
                let input: Expression = match pipe_input(left, true) {
                    select
                        if select.joins.is_empty()
                            && select.lateral_views.is_empty()
                            && select.where_clause.is_none()
                            && select
                                .from
                                .as_ref()
                                .is_some_and(|from| from.expressions.len() == 1) =>
                    {
                        select
                            .from
                            .map(|mut from| from.expressions.remove(0))
                            .unwrap_or_else(null)
                    }
                    select => subquery(Expression::Select(Box::new(select)), None, false),
                };
                let pivoted: Expression = self.pivot_or_unpivot(input)?;
                let mut select: Select = Select::new();
                select.expressions = vec![Expression::Star(star(None, None))];
                select.from = Some(From {
                    expressions: vec![pivoted],
                });
                select
            }
            "TABLESAMPLE" => {
                let sample = self.sample()?;
                let mut select: Select = wrapped(left);
                select.sample = Some(sample);
                select
            }
            "UNION" | "EXCEPT" | "MINUS" | "INTERSECT" => {
                self.at += 1;
                let all: bool = self.eat_kw("ALL");
                let distinct: bool = !all && self.eat_kw("DISTINCT");
                let right: Expression = self.query_primary()?;
                return Ok(set_operation(&word, left, right, (all, distinct)));
            }
            "ORDER" | "CLUSTER" | "DISTRIBUTE" | "SORT" | "WINDOW" | "LIMIT" | "OFFSET" => {
                let organization: Organization = self.organization()?;
                if organization.is_empty() {
                    return Err(self.error());
                }
                return Ok(organized(left, organization));
            }
            "AGGREGATE" => {
                self.at += 1;
                let mut select: Select = pipe_input(left, false);
                let aggregates: Vec<Expression> =
                    match self.terminal_at(0) || (self.kw("GROUP") && self.kw_at(1, "BY")) {
                        true => Vec::new(),
                        false => self.comma_list(|parser| parser.named_expression())?,
                    };
                let group: Option<GroupBy> = match self.kw("GROUP") && self.kw_at(1, "BY") {
                    true => Some(self.group_by()?),
                    false => None,
                };
                let keys: Vec<Expression> = group
                    .as_ref()
                    .map(|group| group.expressions.clone())
                    .unwrap_or_default();
                select.expressions = keys.into_iter().chain(aggregates).collect();
                select.group_by = group;
                select
            }
            _ if self.terminal_at(0) => return Ok(left),
            _ if self.at_join() => {
                let join = self.join_relation()?;
                let mut select: Select = pipe_input(left, true);
                select.joins.push(join);
                select
            }
            _ => return Err(self.error()),
        };
        Ok(Expression::Select(Box::new(select)))
    }

    /// `inlineTable` as a query: `VALUES expression, ... [alias]`.
    fn inline_table(&mut self) -> Parsing<Expression> {
        self.relation_primary(false)
    }

    /// `TABLE name` in a query: the table, or the `IDENTIFIER (...)` clause naming it.
    fn table_identifier_reference(&mut self) -> Parsing<Expression> {
        if self.kw("IDENTIFIER") && self.char_at(1, '(') {
            self.at += 2;
            let name: Expression = self.expression()?;
            self.expect_char(')')?;
            return Ok(call(crate::parse::constants::IDENTIFIER_CLAUSE, vec![name]));
        }
        Ok(Expression::Table(Box::new(
            crate::parse::_helpers::tree::nodes::table(self.multipart_identifier()?),
        )))
    }
}
