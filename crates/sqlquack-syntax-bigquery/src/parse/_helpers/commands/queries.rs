//! `query`, its terms, primaries, specifications, organization and pipe operators (SqlBaseParser.g4).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Null};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    ClusterBy, Cte, DistributeBy, From, GroupBy, Having, Limit, NamedWindow, OrderBy, Ordered,
    PipeOperator, Qualify, Select, SortBy, Where, With,
};

use crate::keywords::main::is_reserved::is_reserved;
use crate::parse::_helpers::commands::alters::{clause, named};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    alias, call, cte, null, ordered, set_operation, star, subquery, table_sample, tuple, var,
};
use crate::parse::constants::{
    AFTER_FROM_QUERY, AFTER_SELECT_LIST, CUBE, GROUPING_SETS, MIXED_SET_MODIFIERS,
    MIXED_SET_OPERATIONS, PIPE_EXTEND, ROLLUP,
};

/// What a set operation in a chain must share with the first: its operation and modifiers.
#[derive(PartialEq)]
struct SetSignature {
    operation: (String, bool),
    modifiers: SetModifiers,
}

/// A set operation's outer mode, `BY NAME`, `CORRESPONDING`, `STRICT` and lowercased column list.
#[derive(PartialEq)]
struct SetModifiers {
    outer_kind: Option<String>,
    by_name: bool,
    corresponding: bool,
    strict: bool,
    columns: Vec<String>,
}

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

/// Whether `body` has common table expressions of its own.
fn has_with(body: &Expression) -> bool {
    let mut base: &Expression = body;
    while let Expression::PipeOperator(node) = base {
        base = &node.this;
    }
    match base {
        Expression::Select(select) => select.with.is_some(),
        Expression::Union(node) => node.with.is_some(),
        Expression::Intersect(node) => node.with.is_some(),
        Expression::Except(node) => node.with.is_some(),
        _ => false,
    }
}

/// `body` as a pipe's input: wrapped in a subquery when its first query has its own `WITH`.
fn pipe_input(body: Expression) -> Expression {
    match has_with(&body) {
        true => subquery(body, None, false),
        false => body,
    }
}

/// `this |> expression`, a pipe operator as its `( KEYWORDS operands... )` clause.
pub(crate) fn piped(this: Expression, expression: Expression) -> Expression {
    Expression::PipeOperator(Box::new(PipeOperator { this, expression }))
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
fn organized(body: Expression, organization: Organization, parenthesised: bool) -> Expression {
    if organization.is_empty() {
        return body;
    }
    let outside: bool =
        parenthesised && organization.order.is_some() && matches!(body, Expression::Select(_));
    if outside || !takes(&body, &organization) {
        return reorganized(body, organization);
    }
    let mut body: Expression = body;
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

/// `(body) ORDER BY ... LIMIT ...` for a body with clauses of its own, as pipe steps where it can.
fn reorganized(body: Expression, organization: Organization) -> Expression {
    let pipes: bool = organization.cluster.is_none()
        && organization.distribute.is_none()
        && organization.sort.is_none()
        && organization.windows.is_none()
        && (organization.offset.is_none() || organization.limit.is_some());
    if !pipes {
        return organized(
            Expression::Select(Box::new(wrapped(body))),
            organization,
            false,
        );
    }
    let mut body: Expression = pipe_input(body);
    if let Some(items) = organization.order {
        let items: Vec<Expression> = items
            .into_iter()
            .map(|item| Expression::Ordered(Box::new(item)))
            .collect();
        body = piped(body, clause("ORDER BY", items));
    }
    if let Some(count) = organization.limit {
        body = piped(
            body,
            match organization.offset {
                Some(offset) => clause("LIMIT OFFSET", vec![count, offset]),
                None => clause("LIMIT", vec![count]),
            },
        );
    }
    body
}

/// `body` with `with` as its common table expressions, wrapped when it has its own already.
pub(crate) fn with_ctes(mut body: Expression, with: With) -> Expression {
    let mut base: &mut Expression = &mut body;
    while let Expression::PipeOperator(node) = base {
        base = &mut node.this;
    }
    let first: Expression = std::mem::replace(base, Expression::Null(Null));
    *base = first_with_ctes(first, with);
    body
}

/// A pipe chain's first query with `with` as its common table expressions.
fn first_with_ctes(body: Expression, with: With) -> Expression {
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
        let (body, parenthesised) = self.query_term()?;
        let organization: Organization = self.organization()?;
        let mut body: Expression = organized(body, organization, parenthesised);
        if self.is_op("|>") || self.pipe_start_at(0) {
            body = pipe_input(body);
        }
        while self.is_op("|>") || self.pipe_start_at(0) {
            self.at += 1;
            body = self.deeper(|parser| parser.pipe(body))?;
        }
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

    /// `aliased_query`: `name AS (query)`; GoogleSQL takes no column list.
    fn named_query(&mut self) -> Parsing<Cte> {
        let name: Identifier = self.ident()?;
        self.expect_kw("AS")?;
        self.expect_char('(')?;
        let body: Expression = self.query()?;
        self.expect_char(')')?;
        Ok(cte(name, Vec::new(), body))
    }

    /// `query_set_operation`, with whether the term is one parenthesised query.
    fn query_term(&mut self) -> Parsing<(Expression, bool)> {
        let first: usize = self.at;
        let parenthesised: bool = self.is_char('(');
        let mut left: Expression = self.query_primary()?;
        let mut chain: Option<SetSignature> = None;
        let mut first_term: bool = true;
        loop {
            let outer: usize = self.set_outer_mode_width();
            let operator: String = match self.word_at(outer) {
                Some(word) if ["UNION", "EXCEPT", "INTERSECT"].contains(&word.as_str()) => word,
                _ => return Ok((left, parenthesised && first_term)),
            };
            first_term = false;
            let start: usize = self.offset();
            let outer_kind: Option<String> = match outer {
                0 => None,
                _ => {
                    let side: String = match self.word_at(0).unwrap_or_default().as_str() {
                        "OUTER" => "FULL".to_string(),
                        side => side.to_string(),
                    };
                    self.at += outer;
                    Some(side)
                }
            };
            self.at += 1;
            let all: bool = self.eat_kw("ALL");
            let distinct: bool = !all && self.eat_kw("DISTINCT");
            if !(all || distinct) {
                return Err(self.error());
            }
            let (mut by_name, mut corresponding, mut on) = (false, false, Vec::new());
            let strict: bool = self.set_operation_strict(outer_kind.is_some())?;
            if self.kw("BY") && self.kw_at(1, "NAME") {
                self.at += 2;
                by_name = true;
                if self.eat_kw("ON") {
                    on = self.identifier_list()?;
                }
            } else if self.eat_kw("CORRESPONDING") {
                corresponding = true;
                if self.eat_kw("BY") {
                    on = self.identifier_list()?;
                }
            }
            let signature: SetSignature = SetSignature {
                operation: (operator.clone(), all),
                modifiers: SetModifiers {
                    outer_kind: outer_kind.clone(),
                    by_name,
                    corresponding,
                    strict,
                    columns: on.iter().map(|name| name.name.to_lowercase()).collect(),
                },
            };
            match &chain {
                Some(earlier) if earlier.operation != signature.operation => {
                    return Err(self.error_with(MIXED_SET_OPERATIONS, start, true));
                }
                Some(earlier) if earlier.modifiers != signature.modifiers => {
                    return Err(self.error_with(MIXED_SET_MODIFIERS, start, false));
                }
                _ => chain = Some(signature),
            }
            let right: Expression = self.deeper(|parser| parser.query_primary())?;
            let mut node: Expression = set_operation(&operator, left, right, (all, distinct));
            macro_rules! matched {
                ($node:expr) => {{
                    $node.by_name = by_name;
                    $node.corresponding = corresponding;
                    $node.strict = strict;
                    $node.on_columns = on.iter().cloned().map(Expression::Identifier).collect();
                    $node.side = outer_kind.clone();
                }};
            }
            match &mut node {
                Expression::Union(node) => matched!(node),
                Expression::Intersect(node) => matched!(node),
                Expression::Except(node) => matched!(node),
                _ => {}
            }
            left = self.spanned(first, node);
        }
    }

    /// `opt_strict` after a set operation's `ALL` or `DISTINCT`, refused with an outer mode or `BY NAME`.
    fn set_operation_strict(&mut self, outer: bool) -> Parsing<bool> {
        let start: usize = self.offset();
        if !self.eat_kw("STRICT") {
            return Ok(false);
        }
        if outer || (self.kw("BY") && self.kw_at(1, "NAME")) {
            return Err(self.error_with(
                "Syntax error: STRICT cannot be used with outer mode or BY NAME in set operations",
                start,
                true,
            ));
        }
        Ok(true)
    }

    /// `queryPrimary`: a `SELECT`, `FROM ...`, `TABLE t`, `VALUES ...` or `( query )`.
    pub(crate) fn query_primary(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            self.at += 1;
            let query: Expression = self.query()?;
            self.expect_char(')')?;
            if !(self.kw("AS") && self.is_at(1, is_identifier)) {
                return Ok(query);
            }
            self.at += 1;
            let name: Identifier = self.ident()?;
            return Ok(piped(query, clause("AS", vec![named(name)])));
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

    /// `from_query`: `FROM relations` alone, a select with no select list.
    fn leading_from_query(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let select: Select = self.select_from_clause(Select::new())?;
        let clause_word: Option<String> = self
            .word_at(0)
            .filter(|word| AFTER_FROM_QUERY.contains(&word.as_str()))
            .filter(|word| !["ORDER", "GROUP"].contains(&word.as_str()) || self.kw_at(1, "BY"));
        if let Some(word) = clause_word {
            let shown: String = match word.as_str() {
                "ORDER" | "GROUP" => format!("{word} BY"),
                _ => word,
            };
            return Err(self.error_with(
                &format!("Syntax error: {shown} not supported after FROM query"),
                self.offset(),
                true,
            ));
        }
        Ok(self.spanned(first, Expression::Select(Box::new(select))))
    }

    /// `selectClause`: `SELECT hints [DISTINCT|ALL] namedExpressionSeq`.
    fn select_clause(&mut self, mut select: Select) -> Parsing<Select> {
        self.expect_kw("SELECT")?;
        if self.kw("WITH") && self.is_at(1, crate::parse::_helpers::tree::cursor::is_identifier) {
            let start: usize = self.offset();
            self.at += 2;
            if self.kw("OPTIONS") && self.char_at(1, '(') {
                self.at += 1;
                self.options_list()?;
            }
            self.gap("select_with", start);
        }
        if self.kw("DISTINCT") {
            self.at += 1;
            select.distinct = true;
        } else if self.kw("ALL") && !self.terminal_at(1) && !self.starts_clause(1) {
            self.at += 1;
            select.all = true;
        }
        if self.kw("AS") && self.kw_in(1, &["STRUCT", "VALUE"]) {
            select.kind = self.word_at(1);
            self.at += 2;
        }
        let mut items: Vec<Expression> = vec![self.named_expression()?];
        while self.eat_char(',') {
            let clause: bool = self.word_at(0).is_some_and(|word| {
                AFTER_SELECT_LIST.contains(&word.as_str()) && is_reserved(&word)
            });
            if clause || self.ends_at(0) || self.is_char(')') || self.is_op("|>") {
                break;
            }
            items.push(self.named_expression()?);
        }
        select.expressions = items;
        Ok(select)
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
        if self.is_char('(') && self.char_at(1, ')') {
            self.at += 2;
            return Ok(tuple(Vec::new()));
        }
        match self.grouping_analytics()? {
            Some(analytics) => Ok(analytics),
            None => self.expression(),
        }
    }

    /// Whether pipe AGGREGATE's `GROUP [AND ORDER] BY` begins at the cursor.
    fn at_pipe_group(&self) -> bool {
        self.kw("GROUP")
            && (self.kw_at(1, "BY")
                || (self.kw_at(1, "AND") && self.kw_at(2, "ORDER") && self.kw_at(3, "BY")))
    }

    /// `item` with a pipe item's `ASC | DESC [NULLS FIRST | LAST]`, as an `Ordered` over it.
    fn selection_order(&mut self, item: Expression, grouping: bool) -> Expression {
        let explicit_asc: bool = self.eat_kw("ASC");
        let desc: bool = !explicit_asc && self.eat_kw("DESC");
        let mut nulls_first: Option<bool> = None;
        if (explicit_asc || desc || grouping)
            && self.kw("NULLS")
            && self.kw_in(1, &["FIRST", "LAST"])
        {
            nulls_first = Some(self.kw_at(1, "FIRST"));
            self.at += 2;
        }
        match explicit_asc || desc || nulls_first.is_some() {
            true => Expression::Ordered(Box::new(ordered(item, desc, explicit_asc, nulls_first))),
            false => item,
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

    /// `pipe_operator` after `|>`: `left |> ( KEYWORDS operands... )` as a `PipeOperator`.
    pub(crate) fn pipe(&mut self, left: Expression) -> Parsing<Expression> {
        let word: String = self.word_at(0).unwrap_or_default();
        let operator: Expression = match word.as_str() {
            "SELECT" => {
                let mut select: Select = self.select_clause(Select::new())?;
                if self.kw("WINDOW") && self.is_at(1, is_identifier) {
                    select.windows = Some(self.window_clause()?);
                }
                clause("SELECT", vec![Expression::Select(Box::new(select))])
            }
            "EXTEND" | "WINDOW" => {
                self.at += 1;
                let mut select: Select = Select::new();
                select.expressions = self.pipe_items()?;
                if word == PIPE_EXTEND && self.kw("WINDOW") && self.is_at(1, is_identifier) {
                    select.windows = Some(self.window_clause()?);
                }
                clause(&word, vec![Expression::Select(Box::new(select))])
            }
            "SET" => {
                self.at += 1;
                let replace: Vec<Alias> = self.pipe_list(|parser| {
                    let name: Identifier = parser.pipe_column("SET")?;
                    parser.expect_char('=')?;
                    Ok(Alias::new(parser.expression()?, name))
                })?;
                let star = sqlquack_syntax::expressions::names::models::Star {
                    replace: Some(replace),
                    ..star(None, None)
                };
                clause("SET", vec![Expression::Star(star)])
            }
            "DROP" => {
                self.at += 1;
                let except: Vec<Identifier> =
                    self.pipe_list(|parser| parser.pipe_column("DROP"))?;
                clause("DROP", vec![Expression::Star(star(None, Some(except)))])
            }
            "RENAME" => {
                self.at += 1;
                let renames: Vec<(Identifier, Identifier)> = self.pipe_list(|parser| {
                    let from: Identifier = parser.pipe_column("RENAME")?;
                    parser.eat_kw("AS");
                    Ok((from, parser.ident()?))
                })?;
                let star = sqlquack_syntax::expressions::names::models::Star {
                    rename: Some(renames),
                    ..star(None, None)
                };
                clause("RENAME", vec![Expression::Star(star)])
            }
            "DISTINCT" => {
                self.at += 1;
                clause("DISTINCT", Vec::new())
            }
            "AS" => {
                self.at += 1;
                clause("AS", vec![named(self.ident()?)])
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
                clause("WHERE", vec![condition])
            }
            "ORDER" if self.kw_at(1, "BY") => {
                self.at += 2;
                let items: Vec<Ordered> = self.pipe_list(|parser| parser.sort_item())?;
                let items: Vec<Expression> = items
                    .into_iter()
                    .map(|item| Expression::Ordered(Box::new(item)))
                    .collect();
                clause("ORDER BY", items)
            }
            "LIMIT" => {
                self.at += 1;
                let count: Expression = self.expression()?;
                match self.eat_kw("OFFSET") {
                    true => clause("LIMIT OFFSET", vec![count, self.expression()?]),
                    false => clause("LIMIT", vec![count]),
                }
            }
            "PIVOT" | "UNPIVOT" => {
                let pivoted: Expression = self.pivot_or_unpivot(null())?;
                clause(&word, vec![pivoted])
            }
            "TABLESAMPLE" => {
                let sample = self.sample()?;
                clause("TABLESAMPLE", vec![table_sample(None, sample)])
            }
            _ if self.pipe_set_ahead() => return self.pipe_set_operation(left),
            "AGGREGATE" => self.pipe_aggregate()?,
            "WITH" => {
                let start: usize = self.offset();
                self.ctes()?;
                self.eat_char(',');
                self.gap("pipe_with", start);
                return Ok(left);
            }
            "MATCH_RECOGNIZE" => {
                let recognized: Expression = self.match_recognize(None)?;
                clause("MATCH_RECOGNIZE", vec![recognized])
            }
            "CALL" => {
                self.at += 1;
                let mut operands: Vec<Expression> = vec![self.aliased_tvf()?];
                if self.kw("PARTITION") && self.kw_at(1, "BY") {
                    self.at += 2;
                    let keys: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
                    operands.push(clause("PARTITION BY", keys));
                }
                if self.kw("ORDER") && self.kw_at(1, "BY") {
                    self.at += 2;
                    let items: Vec<Expression> = self
                        .comma_list(|parser| parser.sort_item())?
                        .into_iter()
                        .map(|item| Expression::Ordered(Box::new(item)))
                        .collect();
                    operands.push(clause("ORDER BY", items));
                }
                clause("CALL", operands)
            }
            _ if self.at_join() => {
                let join = self.join_relation()?;
                clause("JOIN", vec![Expression::Join(Box::new(join))])
            }
            _ => return Err(self.error()),
        };
        Ok(piped(left, operator))
    }

    /// A pipe set operation, the standard one with `left` as its first input.
    fn pipe_set_operation(&mut self, left: Expression) -> Parsing<Expression> {
        let start: usize = self.offset();
        let width: usize = self.set_outer_mode_width();
        let side: bool = width > 0;
        self.at += width;
        let word: String = self.word_at(0).unwrap_or_default();
        self.at += 1;
        let all: bool = self.eat_kw("ALL");
        let distinct: bool = !all && self.eat_kw("DISTINCT");
        if !(all || distinct) {
            return Err(self.error());
        }
        let strict: bool = self.set_operation_strict(side)?;
        let mut qualified: bool = strict;
        if self.eat_kw("CORRESPONDING") {
            qualified = true;
            if self.eat_kw("BY") {
                self.identifier_list()?;
            }
        } else if self.eat_kws(&["BY", "NAME"]) {
            qualified = true;
            if self.eat_kw("ON") {
                self.identifier_list()?;
            }
        }
        let mut right: Expression = self.query_primary()?;
        let mut more: bool = false;
        while self.eat_char(',') {
            if self.terminal_at(0) {
                break;
            }
            right = self.query_primary()?;
            more = true;
        }
        if side || qualified || more {
            self.gap("pipe_set_operation", start);
        }
        Ok(set_operation(&word, left, right, (all, distinct)))
    }

    /// The pipe `AGGREGATE` operator after `|>`, with its `GROUP [AND ORDER] BY` keys.
    fn pipe_aggregate(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let mut group_and_order: bool = false;
        let mut aggregates: Vec<Expression> = Vec::new();
        while !(self.terminal_at(0) || self.at_pipe_group()) {
            let aggregate: Expression = self.named_expression()?;
            aggregates.push(self.selection_order(aggregate, false));
            if !self.eat_char(',') {
                break;
            }
        }
        let mut select: Select = Select::new();
        if self.at_pipe_group() {
            self.at += 1;
            group_and_order = self.eat_kws(&["AND", "ORDER"]);
            self.expect_kw("BY")?;
            let mut keys: Vec<Expression> = Vec::new();
            loop {
                let item: Expression = self.group_item()?;
                let key: Expression = match self.kw("AS") || self.bare_alias_at(0, false) {
                    true => {
                        let explicit: bool = self.eat_kw("AS");
                        let name: Identifier = self.ident()?;
                        alias(item, name, explicit)
                    }
                    false => item,
                };
                keys.push(self.selection_order(key, true));
                if !self.eat_char(',') || self.terminal_at(0) {
                    break;
                }
            }
            select.group_by = Some(GroupBy {
                expressions: keys,
                all: None,
                totals: false,
                comments: Vec::new(),
            });
        }
        select.expressions = aggregates;
        let mut operands: Vec<Expression> = vec![Expression::Select(Box::new(select))];
        if group_and_order {
            operands.push(var("GROUP AND ORDER BY"));
        }
        Ok(clause("AGGREGATE", operands))
    }

    /// A pipe operator's items, `item, ...`, with a trailing comma allowed.
    fn pipe_list<T>(&mut self, mut item: impl FnMut(&mut Self) -> Parsing<T>) -> Parsing<Vec<T>> {
        let mut items: Vec<T> = vec![item(self)?];
        while self.eat_char(',') {
            if self.terminal_at(0) || self.kw("WINDOW") {
                break;
            }
            items.push(item(self)?);
        }
        Ok(items)
    }

    /// `pipe_selection_item_list`: select items, with a trailing comma allowed.
    fn pipe_items(&mut self) -> Parsing<Vec<Expression>> {
        self.pipe_list(|parser| parser.named_expression())
    }

    /// A column `SET`, `DROP` or `RENAME` names: a name alone, never a path.
    fn pipe_column(&mut self, operator: &str) -> Parsing<Identifier> {
        let name: Identifier = self.ident()?;
        if self.is_char('.') {
            return Err(self.error_with(
                &format!(
                    "Syntax error: Pipe {operator} can only refer to columns by name alone; \
                     columns under table aliases or fields under paths are not supported"
                ),
                self.offset(),
                true,
            ));
        }
        Ok(name)
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
