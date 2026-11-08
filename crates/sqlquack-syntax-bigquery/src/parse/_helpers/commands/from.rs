//! `fromClause`: relations, joins, aliases, pivots, samples and lateral views (SqlBaseParser.g4).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    From, Join, JoinKind, LateralView, MatchRecognize, MatchRecognizeAfter, MatchRecognizeMeasure,
    Pivot, Sample, SampleMethod, Select, Unpivot,
};
use sqlquack_syntax::expressions::statements::models::Version;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_strict_identifier};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, alias, call, column_from, function, in_list, join, joined,
    joined_table, lateral, literal, null, subquery, table, table_sample, tuple, values,
};
use crate::parse::constants::{
    COMMA_AFTER_CONSECUTIVE_CONDITIONS, IDENTIFIER_CLAUSE, JOIN_CONDITION_COUNT,
    MIN_CONSECUTIVE_CONDITIONS, OUTER_JOIN_AFTER_COMMA, SYSTEM_VARIABLE_PREFIX,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// `item` with `joins` after it, as one joined table when there are any.
fn collapse(item: Expression, joins: Vec<Join>) -> Expression {
    match joins.is_empty() {
        true => item,
        false => joined_table(item, joins, None),
    }
}

/// A row pattern's name as written: a backquoted one keeps its backquotes, escaped.
fn pattern_name(name: Identifier) -> String {
    match name.quoted {
        true => format!("`{}`", name.name.replace('\\', "\\\\").replace('`', "\\`")),
        false => name.name,
    }
}

/// `item` under `alias`: its column names rename it, as lineage reads an aliased FROM item.
pub(crate) fn aliased(item: Expression, alias: Option<TableAlias>) -> Expression {
    let Some(alias) = alias else {
        return item;
    };
    let mut node: Alias = Alias::new(item, alias.name);
    node.column_aliases = alias.columns;
    node.alias_explicit_as = alias.explicit;
    Expression::Alias(Box::new(node))
}

/// Whether `join`'s kind takes a condition: not `CROSS`, a comma or `NATURAL`.
fn qualified(join: &Join) -> bool {
    !matches!(
        join.kind,
        JoinKind::Cross
            | JoinKind::Implicit
            | JoinKind::Natural
            | JoinKind::NaturalLeft
            | JoinKind::NaturalRight
            | JoinKind::NaturalFull
    )
}

/// One `ON` or `USING` clause of a join's condition list.
enum Condition {
    On(Box<Expression>),
    Using(Vec<Identifier>),
}

impl Condition {
    fn close(self, mut join: Join) -> Join {
        match self {
            Condition::On(condition) => join.on = Some(*condition),
            Condition::Using(columns) => join.using = columns,
        }
        join
    }
}

/// Whether a condition of this chain came from consecutive `ON` or `USING` clauses.
fn deferred(joins: &[Join]) -> bool {
    joins.iter().any(|join| join.deferred_condition)
}

/// A join_processor.cc stack entry: a from item and its joins, or a join awaiting a condition.
enum Entry {
    Chain(Box<Expression>, Vec<Join>),
    Open(Box<Join>, usize),
}

/// join_processor.cc's rewrite of consecutive `ON` or `USING` clauses; `None` if a join lacks one.
fn transformed(
    item: Expression,
    steps: Vec<(Join, Vec<Condition>)>,
) -> Option<(Expression, Vec<Join>)> {
    let mut stack: Vec<Entry> = vec![Entry::Chain(Box::new(item), Vec::new())];
    for (index, (mut node, conditions)) in steps.into_iter().enumerate() {
        let right: Expression = std::mem::replace(&mut node.this, null());
        match qualified(&node) {
            false => {
                let Some(Entry::Chain(_, joins)) = stack.last_mut() else {
                    return None;
                };
                node.this = right;
                joins.push(node);
            }
            true => {
                stack.push(Entry::Open(Box::new(node), index));
                stack.push(Entry::Chain(Box::new(right), Vec::new()));
            }
        }
        for (position, condition) in conditions.into_iter().enumerate() {
            let (Some(Entry::Chain(right, inner)), Some(Entry::Open(mut open, owner))) =
                (stack.pop(), stack.pop())
            else {
                return None;
            };
            open.this = collapse(*right, inner);
            let mut open: Join = condition.close(*open);
            open.deferred_condition = !(owner == index && position == 0);
            let Some(Entry::Chain(_, joins)) = stack.last_mut() else {
                return None;
            };
            joins.push(open);
        }
    }
    match (stack.pop(), stack.is_empty()) {
        (Some(Entry::Chain(item, joins)), true) => Some((*item, joins)),
        _ => None,
    }
}

/// The rows of an inline table: a row constructor's items, or the one value.
fn row(item: Expression) -> Vec<Expression> {
    match item {
        Expression::Tuple(row) => row.expressions,
        Expression::Paren(paren) => vec![paren.this],
        other => vec![other],
    }
}

impl Parser<'_> {
    /// `fromClause`: `FROM relation, ... lateralView*`, into `select`'s sources and joins.
    pub(crate) fn select_from_clause(&mut self, mut select: Select) -> Parsing<Select> {
        self.expect_kw("FROM")?;
        let mut items: Vec<Expression> = Vec::new();
        let mut joins: Vec<Join> = Vec::new();
        loop {
            let (item, more) = self.relation()?;
            if deferred(&more) && (!items.is_empty() || self.is_char(',')) {
                return Err(self.error_with(
                    COMMA_AFTER_CONSECUTIVE_CONDITIONS,
                    self.offset(),
                    true,
                ));
            }
            if !items.is_empty()
                && let Some(outer) = more.iter().find(|join| {
                    matches!(
                        join.kind,
                        JoinKind::Right
                            | JoinKind::Full
                            | JoinKind::NaturalRight
                            | JoinKind::NaturalFull
                    )
                })
            {
                let start: usize = outer.span.0.map_or(self.offset(), |span| span.start);
                return Err(self.error_with(OUTER_JOIN_AFTER_COMMA, start, true));
            }
            match joins.is_empty() {
                true => items.push(item),
                false => joins.push(join(
                    item,
                    JoinKind::Implicit,
                    JoinConstraint::None,
                    (false, false),
                )),
            }
            joins.extend(more);
            if !self.eat_char(',') {
                break;
            }
        }
        select.from = Some(From { expressions: items });
        select.joins = joins;
        select.lateral_views = self.lateral_views()?;
        if !select.lateral_views.is_empty() && self.kw_in(0, &["PIVOT", "UNPIVOT"]) {
            let start: usize = self.offset();
            self.pivot_or_unpivot(null())?;
            self.gap("pivot_after_lateral_view", start);
        }
        Ok(select)
    }

    /// `relation`: `[LATERAL] relationPrimary` and its joins.
    pub(crate) fn relation(&mut self) -> Parsing<(Expression, Vec<Join>)> {
        let lateral: bool = self.eat_kw("LATERAL");
        let item: Expression = self.relation_primary(lateral)?;
        let mut steps: Vec<(Join, Vec<Condition>)> = Vec::new();
        let (mut open, mut saved_error, mut consecutive) = (0usize, false, false);
        loop {
            if self.at_join() {
                let first: usize = self.at;
                let mut node: Join = self.deeper(|parser| parser.join_relation())?;
                node.span = self.source_range(first);
                let mut conditions: Vec<Condition> = Vec::new();
                if let Some(on) = node.on.take() {
                    conditions.push(Condition::On(Box::new(on)));
                } else if !node.using.is_empty() {
                    conditions.push(Condition::Using(std::mem::take(&mut node.using)));
                }
                while self.kw_in(0, &["ON", "USING"]) {
                    let start: usize = self.offset();
                    conditions.push(match self.eat_kw("ON") {
                        true => Condition::On(Box::new(self.expression()?)),
                        false => {
                            self.expect_kw("USING")?;
                            Condition::Using(self.identifier_list()?)
                        }
                    });
                    let available: usize = open + usize::from(qualified(&node));
                    if conditions.len() >= MIN_CONSECUTIVE_CONDITIONS
                        && (saved_error || conditions.len() > available)
                    {
                        return Err(self.error_with(JOIN_CONDITION_COUNT, start, true));
                    }
                }
                let available: usize = open + usize::from(qualified(&node));
                saved_error |= conditions.len() > available;
                open = available.saturating_sub(conditions.len());
                consecutive |= conditions.len() >= MIN_CONSECUTIVE_CONDITIONS;
                steps.push((node, conditions));
            } else {
                break;
            }
        }
        if !consecutive {
            let joins: Vec<Join> = steps
                .into_iter()
                .map(|(node, conditions)| match conditions.into_iter().next() {
                    Some(condition) => condition.close(node),
                    None => node,
                })
                .collect();
            return Ok((item, joins));
        }
        match transformed(item, steps) {
            Some(chain) => Ok(chain),
            None => Err(self.error_with(JOIN_CONDITION_COUNT, self.offset(), true)),
        }
    }

    /// Whether a `joinRelation` begins at the cursor.
    pub(crate) fn at_join(&self) -> bool {
        self.kw_in(
            0,
            &[
                "JOIN", "INNER", "CROSS", "LEFT", "RIGHT", "FULL", "SEMI", "ANTI", "NATURAL",
            ],
        )
    }

    /// `joinRelation`.
    pub(crate) fn join_relation(&mut self) -> Parsing<Join> {
        let natural: bool = self.eat_kw("NATURAL");
        let (mut inner, mut outer) = (false, false);
        let word: String = self.word_at(0).unwrap_or_default();
        let kind: JoinKind = match word.as_str() {
            "INNER" => {
                self.at += 1;
                inner = true;
                JoinKind::Inner
            }
            "CROSS" => {
                self.at += 1;
                JoinKind::Cross
            }
            "LEFT" => {
                self.at += 1;
                if self.eat_kw("SEMI") {
                    JoinKind::LeftSemi
                } else if self.eat_kw("ANTI") {
                    JoinKind::LeftAnti
                } else {
                    outer = self.eat_kw("OUTER");
                    JoinKind::Left
                }
            }
            "SEMI" => {
                self.at += 1;
                JoinKind::LeftSemi
            }
            "ANTI" => {
                self.at += 1;
                JoinKind::LeftAnti
            }
            "RIGHT" | "FULL" => {
                self.at += 1;
                outer = self.eat_kw("OUTER");
                match word.as_str() {
                    "RIGHT" => JoinKind::Right,
                    _ => JoinKind::Full,
                }
            }
            _ => JoinKind::Inner,
        };
        self.expect_kw("JOIN")?;
        let lateral: bool = self.eat_kw("LATERAL");
        let right: Expression = self.relation_primary(lateral)?;
        if natural {
            let kind: JoinKind = match kind {
                JoinKind::Left => JoinKind::NaturalLeft,
                JoinKind::Right => JoinKind::NaturalRight,
                JoinKind::Full => JoinKind::NaturalFull,
                _ => JoinKind::Natural,
            };
            return Ok(join(right, kind, JoinConstraint::None, (inner, outer)));
        }
        let constraint: JoinConstraint = if self.eat_kw("ON") {
            JoinConstraint::On(Box::new(self.expression()?))
        } else if self.eat_kw("USING") {
            JoinConstraint::Using(self.identifier_list()?)
        } else if self.kw_in(0, &["APPROX", "EXACT"]) && self.kw_at(1, "NEAREST") {
            let start: usize = self.offset();
            self.at += 2;
            if self
                .peek()
                .is_some_and(|token| token.kind == TokenKind::Number(NumberKind::Integer))
            {
                self.at += 1;
            }
            self.expect_kw("BY")?;
            if !(self.eat_kw("DISTANCE") || self.eat_kw("SIMILARITY")) {
                return Err(self.error());
            }
            self.expression()?;
            self.gap("nearest_by", start);
            JoinConstraint::None
        } else {
            JoinConstraint::None
        };
        Ok(join(right, kind, constraint, (inner, outer)))
    }

    /// `tableAlias`: `[AS] strictIdentifier [(columns)]`, when present.
    pub(crate) fn table_alias(&mut self) -> Parsing<Option<TableAlias>> {
        let explicit: bool = self.eat_kw("AS");
        if !explicit && !self.bare_alias_at(0, true) {
            return Ok(None);
        }
        let name: Identifier = self.name(is_strict_identifier)?;
        Ok(Some(TableAlias {
            name,
            columns: Vec::new(),
            explicit,
        }))
    }

    /// `relationPrimary`, `LATERAL` when it followed `LATERAL`.
    pub(crate) fn relation_primary(&mut self, is_lateral: bool) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut node: Expression = self.relation_primary_inner(is_lateral)?;
        let join: bool = match &node {
            Expression::JoinedTable(_) => true,
            Expression::Alias(alias) => matches!(alias.this, Expression::JoinedTable(_)),
            _ => false,
        };
        if !join && self.kw_in(0, &["PIVOT", "UNPIVOT"]) {
            node = self.pivot_or_unpivot(node)?;
        }
        loop {
            if self.kw("TABLESAMPLE") {
                node = table_sample(Some(node), self.sample()?);
            } else if self.kw("MATCH_RECOGNIZE") && self.char_at(1, '(') {
                node = self.match_recognize(Some(node))?;
            } else {
                break;
            }
        }
        Ok(self.spanned(first, node))
    }

    fn relation_primary_inner(&mut self, is_lateral: bool) -> Parsing<Expression> {
        let start: usize = self.offset();
        if self.kw("STREAM")
            && (self.char_at(1, '(')
                || (self.is_at(1, is_strict_identifier)
                    && !self.starts_clause(1)
                    && !self.kw_at(1, "AS")))
        {
            self.at += 1;
            let inner: Expression = self.relation_primary_inner(false)?;
            self.gap("stream", start);
            return Ok(inner);
        }
        if self.is_char('(') {
            return self.parenthesised_relation(is_lateral);
        }
        if self.kw("GRAPH_TABLE") && self.char_at(1, '(') {
            self.at += 1;
            let mut depth: usize = 0;
            loop {
                if self.is_char('(') {
                    depth += 1;
                } else if self.is_char(')') {
                    depth -= 1;
                }
                self.advance()?;
                if depth == 0 {
                    break;
                }
            }
            self.gap("graph_table", start);
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(aliased(null(), alias));
        }
        if self.kw("UNNEST") && self.char_at(1, '(') {
            self.at += 2;
            let items: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
            self.expect_char(')')?;
            let item: Expression = call("UNNEST", items);
            let alias: Option<TableAlias> = self.table_alias()?;
            let item: Expression = aliased(item, alias);
            return self.with_offset(item);
        }
        if self.kw("VALUES") {
            self.at += 1;
            let rows: Vec<Vec<Expression>> = self
                .comma_list(|parser| parser.expression())?
                .into_iter()
                .map(row)
                .collect();
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(values(rows, alias));
        }
        if self.kw("IDENTIFIER") && self.char_at(1, '(') {
            self.at += 2;
            let name: Expression = self.expression()?;
            self.expect_char(')')?;
            let item: Expression = call(IDENTIFIER_CLAUSE, vec![name]);
            if self.is_char('(') {
                self.gap("identifier_clause_function", start);
                self.function_call(vec![Identifier::new(IDENTIFIER_CLAUSE)])?;
            }
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(aliased(item, alias));
        }
        self.named_relation(is_lateral)
    }

    /// A `relationPrimary` in parentheses: a query, or a relation with its joins.
    fn parenthesised_relation(&mut self, is_lateral: bool) -> Parsing<Expression> {
        if self.starts_query_at(1) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.query()?;
                parser.expect_char(')')?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                self.watermark()?;
                let alias: Option<TableAlias> = self.table_alias()?;
                return Ok(subquery(query, alias, is_lateral));
            }
        }
        self.at += 1;
        let (item, joins) = self.deeper(|parser| parser.relation())?;
        self.expect_char(')')?;
        self.watermark()?;
        let alias: Option<TableAlias> = self.table_alias()?;
        Ok(match (joins.is_empty(), alias) {
            (false, Some(alias)) if alias.columns.is_empty() => {
                joined_table(item, joins, Some(alias.name))
            }
            (_, alias) => aliased(collapse(item, joins), alias),
        })
    }

    /// A `relationPrimary` that begins with a name: a table-valued function call or a table.
    fn named_relation(&mut self, is_lateral: bool) -> Parsing<Expression> {
        let name: Vec<Identifier> = self.tvf_name()?;
        if self.is_char('(') {
            let call: Expression = self.tvf(name)?;
            if self.kw("IDENTIFIED") && self.kw_at(1, "BY") {
                let clause: usize = self.offset();
                self.at += 2;
                self.ident()?;
                self.gap("identified_by", clause);
            }
            self.watermark()?;
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(match is_lateral {
                true => lateral(call, alias),
                false => aliased(call, alias),
            });
        }
        let mut relation: TableRef = table(name);
        if self.kw("CHANGES") && self.kw_at(1, "FROM") {
            let clause: usize = self.offset();
            self.at += 2;
            self.skip_changes()?;
            self.gap("changes", clause);
        }
        if self.kw("WITH") && self.char_at(1, '(') {
            let clause: usize = self.offset();
            self.at += 1;
            self.property_list()?;
            self.gap("relation_options", clause);
        }
        self.watermark()?;
        if let Some(alias) = self.table_alias()? {
            relation.alias = Some(alias.name);
            relation.alias_explicit_as = alias.explicit;
            relation.column_aliases = alias.columns;
        }
        relation.version = self.temporal_clause()?;
        if self.kw("TABLESAMPLE") {
            relation.table_sample = Some(Box::new(self.sample()?));
        }
        let mut item: Expression = Expression::Table(Box::new(relation));
        if self.kw("WITH") && self.kw_at(1, "OFFSET") {
            item = self.with_offset(item)?;
        }
        Ok(item)
    }

    /// `WITH OFFSET [[AS] alias]` after an array source, kept as a call wrapping the source.
    fn with_offset(&mut self, item: Expression) -> Parsing<Expression> {
        if !(self.kw("WITH") && self.kw_at(1, "OFFSET")) {
            return Ok(item);
        }
        self.at += 2;
        let mut args: Vec<Expression> = vec![item];
        if self.eat_kw("AS") || self.bare_alias_at(0, true) {
            args.push(Expression::Identifier(self.ident()?));
        }
        Ok(call("WITH OFFSET", args))
    }

    /// `watermarkClause`: `WATERMARK namedExpression DELAY OF interval`, as a gap.
    fn watermark(&mut self) -> Parsing<()> {
        if !self.kw("WATERMARK") || self.terminal_at(1) {
            return Ok(());
        }
        let start: usize = self.offset();
        self.at += 1;
        self.named_expression()?;
        self.expect_kw("DELAY")?;
        self.expect_kw("OF")?;
        self.expression()?;
        self.gap("watermark", start);
        Ok(())
    }

    /// The rest of a `changesClause` after `CHANGES FROM`.
    fn skip_changes(&mut self) -> Parsing<()> {
        let bound = |parser: &mut Self| -> Parsing<()> {
            if parser.eat_kw("SYSTEM_VERSION") || parser.eat_kw("VERSION") {
                parser.version_value()?;
            } else if parser.eat_kw("SYSTEM_TIME") || parser.eat_kw("TIMESTAMP") {
                parser.deeper(|parser| parser.value_expression(0))?;
            } else {
                return Err(parser.error());
            }
            if !parser.eat_kw("INCLUSIVE") {
                parser.eat_kw("EXCLUSIVE");
            }
            Ok(())
        };
        bound(self)?;
        if self.eat_kw("TO") {
            bound(self)?;
        }
        Ok(())
    }

    /// `version`: an integer or a string.
    fn version_value(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number(NumberKind::Integer)) => {
                let token = self.advance()?;
                Ok(literal(
                    sqlquack_syntax::expressions::names::models::Literal::Number(token.value),
                ))
            }
            _ => self.string_literal(),
        }
    }

    /// `for_version_clause`: `FOR SYSTEM_TIME AS OF expression` or `FOR SYSTEM TIME AS OF ...`.
    fn temporal_clause(&mut self) -> Parsing<Option<Box<Version>>> {
        let width: usize = match self.kw("FOR") {
            true if self.kw_at(1, "SYSTEM_TIME") => 2,
            true if self.kw_at(1, "SYSTEM") && self.kw_at(2, "TIME") => 3,
            _ => return Ok(None),
        };
        self.at += width;
        self.expect_kw("AS")?;
        self.expect_kw("OF")?;
        let value: Expression = self.expression()?;
        Ok(Some(Box::new(Version {
            this: Box::new(Expression::Identifier(Identifier::new("SYSTEM_TIME"))),
            kind: "AS OF".to_owned(),
            expression: Some(Box::new(value)),
        })))
    }

    /// `sample`: `TABLESAMPLE [SYSTEM|BERNOULLI] ( method ) [REPEATABLE (seed)]`.
    pub(crate) fn sample(&mut self) -> Parsing<Sample> {
        self.expect_kw("TABLESAMPLE")?;
        let explicit_method: bool = self.eat_kw("SYSTEM") || self.eat_kw("BERNOULLI");
        if !explicit_method {
            return Err(self.error());
        }
        let typed_method: Option<SampleMethod> = match explicit_method {
            true => Some(
                match self.tokens[self.at - 1].value.to_ascii_uppercase().as_str() {
                    "SYSTEM" => SampleMethod::System,
                    _ => SampleMethod::Bernoulli,
                },
            ),
            false => None,
        };
        self.expect_char('(')?;
        let mut sample = Sample {
            method: typed_method.unwrap_or(SampleMethod::Percent),
            size: null(),
            seed: None,
            offset: None,
            unit_after_size: true,
            use_sample_keyword: false,
            explicit_method,
            method_before_size: explicit_method,
            use_seed_keyword: false,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            is_using_sample: false,
            is_percent: false,
            suppress_method_output: false,
        };
        if self.eat_kw("BUCKET") {
            let numerator: Expression = self.primary_expression()?;
            self.expect_kw("OUT")?;
            self.expect_kw("OF")?;
            let denominator: Expression = self.primary_expression()?;
            if self.eat_kw("ON") {
                let field: Expression = self.expression()?;
                sample.bucket_field = Some(Box::new(field));
            }
            sample.method = SampleMethod::Bucket;
            sample.size = numerator.clone();
            sample.bucket_numerator = Some(Box::new(numerator));
            sample.bucket_denominator = Some(Box::new(denominator));
        } else if self.is_char(')') {
            return Err(self.error_with(
                "TABLESAMPLE does not accept empty inputs",
                self.offset(),
                true,
            ));
        } else {
            let size: Expression = self.expression()?;
            if self.eat_kw("PERCENT") {
                sample.is_percent = true;
                sample.method = typed_method.unwrap_or(SampleMethod::Percent);
            } else if self.eat_kw("ROWS") {
                sample.method = typed_method.unwrap_or(SampleMethod::Row);
            } else {
                let bytes: bool = matches!(&size, Expression::Column(column)
                    if column.table.is_none()
                        && !column.name.quoted
                        && column.name.name.len() > 1
                        && column.name.name[..column.name.name.len() - 1].bytes().all(|byte| byte.is_ascii_digit())
                        && column.name.name.ends_with(['b', 'B', 'k', 'K', 'm', 'M', 'g', 'G']));
                if !bytes {
                    return Err(self.error_with(
                        "not a valid byte length literal, expected syntax: DIGIT+ ('B' | 'K' | 'M' | 'G')",
                        self.offset(),
                        true,
                    ));
                }
                sample.method = typed_method.unwrap_or(SampleMethod::Block);
            }
            sample.size = size;
        }
        self.expect_char(')')?;
        if self.eat_kw("REPEATABLE") {
            self.expect_char('(')?;
            sample.seed = Some(self.primary_expression()?);
            sample.use_seed_keyword = true;
            self.expect_char(')')?;
        }
        Ok(sample)
    }

    /// `match_recognize_clause` over `input`, or as a pipe operator when there is none.
    pub(crate) fn match_recognize(&mut self, input: Option<Expression>) -> Parsing<Expression> {
        self.expect_kw("MATCH_RECOGNIZE")?;
        self.expect_char('(')?;
        let partition_by: Option<Vec<Expression>> = match self.kw("PARTITION") {
            true => {
                self.at += 1;
                self.expect_kw("BY")?;
                Some(self.comma_list(|parser| parser.expression())?)
            }
            false => None,
        };
        self.expect_kw("ORDER")?;
        self.expect_kw("BY")?;
        let order_by: Vec<_> = self.comma_list(|parser| parser.sort_item())?;
        self.expect_kw("MEASURES")?;
        let measures: Vec<MatchRecognizeMeasure> = self.comma_list(|parser| {
            let value: Expression = parser.expression()?;
            parser.expect_kw("AS")?;
            let name: Identifier = parser.ident()?;
            Ok(MatchRecognizeMeasure {
                this: alias(value, name, true),
                window_frame: None,
            })
        })?;
        let after: Option<MatchRecognizeAfter> = match self.eat_kw("AFTER") {
            true => {
                self.expect_kw("MATCH")?;
                self.expect_kw("SKIP")?;
                let past: bool = self.eat_kw("PAST");
                match past {
                    true => self.expect_kw("LAST")?,
                    false => {
                        self.expect_kw("TO")?;
                        self.expect_kw("NEXT")?;
                    }
                }
                self.expect_kw("ROW")?;
                Some(match past {
                    true => MatchRecognizeAfter::PastLastRow,
                    false => MatchRecognizeAfter::ToNextRow,
                })
            }
            false => None,
        };
        self.expect_kw("PATTERN")?;
        self.expect_char('(')?;
        let pattern: String = self.row_pattern()?;
        self.expect_char(')')?;
        self.expect_kw("DEFINE")?;
        let define: Vec<(Identifier, Expression)> = self.comma_list(|parser| {
            let symbol: Identifier = parser.ident()?;
            parser.expect_kw("AS")?;
            Ok((symbol, parser.expression()?))
        })?;
        let options: Vec<Expression> = match self.eat_kw("OPTIONS") {
            true => self.options_list()?,
            false => Vec::new(),
        };
        self.expect_char(')')?;
        let explicit: bool = self.eat_kw("AS");
        let name: Option<Identifier> = match explicit || self.bare_alias_at(0, true) {
            true => Some(self.ident()?),
            false => None,
        };
        Ok(Expression::MatchRecognize(Box::new(MatchRecognize {
            this: input.map(Box::new),
            partition_by,
            order_by: Some(order_by),
            measures: Some(measures),
            rows: None,
            after,
            pattern: Some(pattern),
            define: Some(define),
            alias: name,
            alias_explicit_as: explicit,
            options,
        })))
    }

    /// `row_pattern_expr`: alternatives of concatenations, any of them empty, joined by ` | `.
    fn row_pattern(&mut self) -> Parsing<String> {
        let mut alternatives: Vec<String> = vec![self.row_pattern_concatenation()?];
        loop {
            if self.eat_char('|') {
                alternatives.push(self.row_pattern_concatenation()?);
            } else if self.eat_op("||") {
                alternatives.push(String::new());
                alternatives.push(self.row_pattern_concatenation()?);
            } else {
                return Ok(alternatives.join(" | "));
            }
        }
    }

    /// `row_pattern_concatenation_or_empty`: factors up to the next alternation or `)`.
    fn row_pattern_concatenation(&mut self) -> Parsing<String> {
        let mut factors: Vec<String> = Vec::new();
        while !(self.is_char(')') || self.is_char('|') || self.is_op("||")) {
            factors.push(self.row_pattern_factor()?);
        }
        Ok(factors.join(" "))
    }

    /// `row_pattern_factor`: an anchor, or a symbol or parenthesised pattern with a quantifier.
    fn row_pattern_factor(&mut self) -> Parsing<String> {
        for anchor in ['^', '$'] {
            if self.eat_char(anchor) {
                return Ok(anchor.to_string());
            }
        }
        let mut factor: String = match self.eat_char('(') {
            true => {
                let inner: String = self.deeper(|parser| parser.row_pattern())?;
                self.expect_char(')')?;
                format!("({inner})")
            }
            false => {
                let symbol: Identifier = self.ident()?;
                pattern_name(symbol)
            }
        };
        let reluctant_allowed: bool = if self.eat_char('?') {
            factor.push('?');
            true
        } else if self.eat_char('*') {
            factor.push('*');
            true
        } else if self.eat_char('+') {
            factor.push('+');
            true
        } else if self.eat_char('{') {
            let lower: String = self.row_pattern_bound()?;
            match self.eat_char(',') {
                true => {
                    let upper: String = self.row_pattern_bound()?;
                    self.expect_char('}')?;
                    factor.push_str(&format!("{{{lower},{upper}}}"));
                    true
                }
                false if !lower.is_empty() => {
                    self.expect_char('}')?;
                    factor.push_str(&format!("{{{lower}}}"));
                    false
                }
                false => return Err(self.error()),
            }
        } else {
            false
        };
        if reluctant_allowed && self.eat_char('?') {
            factor.push('?');
        }
        Ok(factor)
    }

    /// A quantifier bound, `int_literal_or_parameter`, as written, or nothing.
    fn row_pattern_bound(&mut self) -> Parsing<String> {
        let name = |token: &Token| matches!(token.kind, TokenKind::Word | TokenKind::Quoted);
        let Some(token) = self.peek_at(0) else {
            return Ok(String::new());
        };
        let prefix: &str = match token.kind {
            TokenKind::Number(NumberKind::Integer | NumberKind::Hex) | TokenKind::Char('?') => {
                let value: String = token.value.clone();
                self.at += 1;
                return Ok(value);
            }
            TokenKind::Char('@') if self.is_at(1, name) => "@",
            TokenKind::Op if token.value == SYSTEM_VARIABLE_PREFIX && self.is_at(1, name) => {
                SYSTEM_VARIABLE_PREFIX
            }
            _ => return Ok(String::new()),
        };
        self.at += 1;
        let mut parts: Vec<String> = vec![pattern_name(self.name(name)?)];
        while prefix == SYSTEM_VARIABLE_PREFIX && self.is_char('.') && self.is_at(1, name) {
            self.at += 1;
            parts.push(pattern_name(self.name(name)?));
        }
        Ok(format!("{prefix}{}", parts.join(".")))
    }

    /// `pivotClause` or `unpivotClause` over `input`.
    pub(crate) fn pivot_or_unpivot(&mut self, input: Expression) -> Parsing<Expression> {
        match self.eat_kw("PIVOT") {
            true => self.pivot(input),
            false => {
                self.expect_kw("UNPIVOT")?;
                self.unpivot(input)
            }
        }
    }

    /// The alias after a pivot or unpivot: `[AS] identifier`.
    fn pivot_alias(&mut self) -> Parsing<Option<Identifier>> {
        if self.eat_kw("AS") {
            return Ok(Some(self.ident()?));
        }
        match self.bare_alias_at(0, true) {
            true => Ok(Some(self.ident()?)),
            false => Ok(None),
        }
    }

    fn pivot(&mut self, input: Expression) -> Parsing<Expression> {
        self.expect_char('(')?;
        let aggregates: Vec<Expression> = self.comma_list(|parser| parser.named_expression())?;
        self.expect_kw("FOR")?;
        let column: Expression = self.deeper(|parser| parser.value_expression(0))?;
        self.expect_kw("IN")?;
        let pivoted: Vec<Expression> = self.parenthesized(|parser| {
            let value: Expression = parser.expression()?;
            let explicit: bool = parser.eat_kw("AS");
            match explicit || parser.bare_alias_at(0, false) {
                true => Ok(alias(value, parser.ident()?, explicit)),
                false => Ok(value),
            }
        })?;
        self.expect_char(')')?;
        let name: Option<Identifier> = self.pivot_alias()?;
        Ok(Expression::Pivot(Box::new(Pivot {
            this: input,
            expressions: aggregates,
            fields: vec![in_list(column, pivoted, None, false)],
            using: Vec::new(),
            group: None,
            unpivot: false,
            into: None,
            alias: name,
            alias_columns: Vec::new(),
            include_nulls: None,
            default_on_null: None,
            with: None,
        })))
    }

    /// An unpivoted column with its optional label.
    fn unpivot_column(&mut self) -> Parsing<Expression> {
        let column: Expression = column_from(self.multipart_identifier()?);
        self.unpivot_labelled(column)
    }

    /// `item opt_as_string_or_integer`: a string label as a quoted name, an integer as a bare one.
    fn unpivot_labelled(&mut self, item: Expression) -> Parsing<Expression> {
        let explicit: bool = self.eat_kw("AS");
        let label: Option<Identifier> = match self.peek().map(|token| token.kind) {
            Some(TokenKind::String { bytes: false, .. }) => {
                let mut text: String = String::new();
                while let Some(TokenKind::String { bytes: false, .. }) =
                    self.peek().map(|token| token.kind)
                {
                    text.push_str(&self.advance()?.value);
                }
                Some(Identifier::quoted(text))
            }
            Some(TokenKind::Number(NumberKind::Integer | NumberKind::Hex)) => {
                Some(Identifier::new(self.advance()?.value))
            }
            _ => None,
        };
        match label {
            Some(label) => Ok(alias(item, label, explicit)),
            None if explicit => Err(self.error_with(
                "Syntax error: Expected integer literal or string literal",
                self.offset(),
                true,
            )),
            None => Ok(item),
        }
    }

    fn unpivot(&mut self, input: Expression) -> Parsing<Expression> {
        let include_nulls: Option<bool> = if self.kw("INCLUDE") && self.kw_at(1, "NULLS") {
            self.at += 2;
            Some(true)
        } else if self.kw("EXCLUDE") && self.kw_at(1, "NULLS") {
            self.at += 2;
            Some(false)
        } else {
            None
        };
        self.expect_char('(')?;
        let parenthesized: bool = self.is_char('(');
        let path = |parser: &mut Self| parser.multipart_identifier().map(joined);
        let mut value_columns: Vec<Identifier> = match parenthesized {
            true => self.parenthesized(path)?,
            false => vec![path(self)?],
        };
        self.expect_kw("FOR")?;
        let name_column: Identifier = path(self)?;
        self.expect_kw("IN")?;
        let columns: Vec<Expression> = self.parenthesized(|parser| match parser.is_char('(') {
            true => {
                let set: Vec<Expression> = parser
                    .parenthesized(|parser| Ok(column_from(parser.multipart_identifier()?)))?;
                parser.unpivot_labelled(tuple(set))
            }
            false => parser.unpivot_column(),
        })?;
        self.expect_char(')')?;
        let name: Option<Identifier> = self.pivot_alias()?;
        let value_column: Identifier = value_columns.remove(0);
        Ok(Expression::Unpivot(Box::new(Unpivot {
            this: input,
            value_column,
            name_column,
            columns,
            alias: name,
            alias_columns: Vec::new(),
            value_column_parenthesized: parenthesized,
            include_nulls,
            extra_value_columns: value_columns,
        })))
    }

    /// `lateralView*`: `LATERAL VIEW [OUTER] generator(args) table [AS] column, ...`.
    pub(crate) fn lateral_views(&mut self) -> Parsing<Vec<LateralView>> {
        let mut views: Vec<LateralView> = Vec::new();
        while self.kw("LATERAL") && self.kw_at(1, "VIEW") {
            self.at += 2;
            let outer: bool = self.eat_kw("OUTER");
            let name: Vec<Identifier> = self.multipart_identifier()?;
            self.expect_char('(')?;
            let args: Vec<Expression> = match self.is_char(')') {
                true => Vec::new(),
                false => self.comma_list(|parser| parser.expression())?,
            };
            self.expect_char(')')?;
            let table_alias: Identifier = self.ident()?;
            let mut column_aliases: Vec<Identifier> = Vec::new();
            if self.eat_kw("AS") || self.bare_alias_at(0, false) {
                column_aliases.push(self.ident()?);
                while self.is_char(',') && self.bare_alias_at(1, false) {
                    self.at += 1;
                    column_aliases.push(self.ident()?);
                }
            }
            views.push(LateralView {
                this: Expression::Function(Box::new(function(name, args))),
                table_alias: Some(table_alias),
                column_aliases,
                outer,
            });
        }
        Ok(views)
    }

    /// `options_list`: `( name = expression, ... )`, possibly empty, as `name = value` pairs.
    pub(crate) fn options_list(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        if self.eat_char(')') {
            return Ok(Vec::new());
        }
        let options: Vec<Expression> = self.comma_list(|parser| {
            let name: Identifier = parser.ident()?;
            let operator: Option<&str> = match parser.eat_char('=') {
                true => None,
                false if parser.eat_op("+=") => Some("+="),
                false if parser.eat_op("-=") => Some("-="),
                false => return Err(parser.error()),
            };
            let value: Expression = parser.expression()?;
            let value: Expression = match operator {
                Some(operator) => {
                    crate::parse::_helpers::commands::alters::clause(operator, vec![value])
                }
                None => value,
            };
            Ok(crate::parse::_helpers::tree::nodes::binary(
                Expression::Eq,
                Expression::Identifier(name),
                value,
            ))
        })?;
        self.expect_char(')')?;
        Ok(options)
    }

    /// `propertyList`: `( key [=] value, ... )`, as `key = value` pairs.
    pub(crate) fn property_list(&mut self) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| parser.property(false))
    }

    /// `property`, or with `expressions` an `expressionProperty`, whose value is any expression.
    pub(crate) fn property(&mut self, expressions: bool) -> Parsing<Expression> {
        let mut strings: usize = 0;
        while self.string_at(strings) {
            strings += 1;
        }
        let key: Expression = match self.string_at(0) {
            true if !self.char_at(strings, '=') => {
                let token = self.advance()?;
                crate::parse::_helpers::tree::nodes::string(&token.value)
            }
            true => self.string_literal()?,
            false => Expression::Identifier(crate::parse::_helpers::tree::nodes::joined(
                self.multipart_identifier()?,
            )),
        };
        let equals: bool = self.eat_char('=');
        let value: Option<Expression> = if expressions && (equals || !self.terminal_at(0)) {
            Some(self.expression()?)
        } else if equals || !self.terminal_at(0) {
            Some(self.property_value()?)
        } else {
            None
        };
        Ok(match value {
            Some(value) => crate::parse::_helpers::tree::nodes::binary(Expression::Eq, key, value),
            None => key,
        })
    }

    /// `propertyValue`: an integer, decimal, boolean or string.
    fn property_value(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number(NumberKind::Integer | NumberKind::Float))
            | Some(TokenKind::String { .. }) => self.primary_expression(),
            Some(TokenKind::Word) if self.kw_in(0, &["TRUE", "FALSE"]) => self.primary_expression(),
            _ => Err(self.error()),
        }
    }
}
