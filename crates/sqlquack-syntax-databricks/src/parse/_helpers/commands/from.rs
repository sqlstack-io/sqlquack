//! `fromClause`, `relation` and its joins, aliases, pivots, samples and lateral views.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    From, Join, JoinKind, LateralView, NearestBy, Pivot, Sample, SampleMethod, Select, Stream,
    Unpivot, Watermark,
};
use sqlquack_syntax::expressions::statements::models::Version;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier, is_strict_identifier};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, alias, call, column_from, function, in_list, join, joined_table,
    lateral, literal, null, paren, string, subquery, table, tuple, values,
};
use crate::parse::constants::{AT_TIMESTAMP_DIGITS, IDENTIFIER_CLAUSE};
use crate::scan::models::{NumberKind, TokenKind};

/// `item` with `joins` after it, as one joined table when there are any.
fn collapse(item: Expression, joins: Vec<Join>) -> Expression {
    match joins.is_empty() {
        true => item,
        false => joined_table(item, joins, None),
    }
}

/// `item` under `alias`: its column names rename it, as lineage reads an aliased FROM item.
fn aliased(item: Expression, alias: Option<TableAlias>) -> Expression {
    let Some(alias) = alias else {
        return item;
    };
    let mut node: Alias = Alias::new(item, alias.name);
    node.column_aliases = alias.columns;
    node.alias_explicit_as = alias.explicit;
    Expression::Alias(Box::new(node))
}

/// The rows of an inline table: a row constructor's items, or the one value.
fn row(item: Expression) -> Vec<Expression> {
    match item {
        Expression::Tuple(row) => row.expressions,
        Expression::Paren(paren) => vec![paren.this],
        other => vec![other],
    }
}

/// The percentage a number literal's text (`negated` when after `-`) reads as, if outside 0..=100.
fn percent_outside_range(text: &str, negated: bool) -> Option<f64> {
    let percent: f64 = match text.parse::<f64>() {
        Ok(value) if negated => -value,
        Ok(value) => value,
        Err(_not_a_number) => return None,
    };
    (!(0.0..=100.0).contains(&percent)).then_some(percent)
}

impl Parser<'_> {
    /// `fromClause`: `FROM relation, ... lateralView*`, into `select`'s sources and joins.
    pub(crate) fn select_from_clause(&mut self, mut select: Select) -> Parsing<Select> {
        self.expect_kw("FROM")?;
        let mut items: Vec<Expression> = Vec::new();
        let mut joins: Vec<Join> = Vec::new();
        loop {
            let (item, more) = self.relation()?;
            items.push(item);
            joins.extend(more);
            if !self.eat_char(',') {
                break;
            }
        }
        select.from = Some(From { expressions: items });
        select.joins = joins;
        select.lateral_views = self.lateral_views()?;
        if self.kw_in(0, &["PIVOT", "UNPIVOT"]) {
            let start: usize = self.offset();
            self.pivot_or_unpivot(null())?;
            self.gap("pivot_after_lateral_view", start);
        }
        Ok(select)
    }

    /// `relation`: `[LATERAL] relationPrimary` and its joins, pivots and unpivots.
    pub(crate) fn relation(&mut self) -> Parsing<(Expression, Vec<Join>)> {
        let lateral: bool = self.eat_kw("LATERAL");
        let mut item: Expression = self.relation_primary(lateral)?;
        let mut joins: Vec<Join> = Vec::new();
        loop {
            if self.at_join() {
                let first: usize = self.at;
                let mut node: Join = self.deeper(|parser| parser.join_relation())?;
                node.span = self.source_range(first);
                joins.push(node);
            } else if self.kw_in(0, &["PIVOT", "UNPIVOT"]) {
                let input: Expression = collapse(item, std::mem::take(&mut joins));
                item = self.pivot_or_unpivot(input)?;
            } else if self.kw("MATCH_RECOGNIZE") && self.char_at(1, '(') {
                let input: Expression = collapse(item, std::mem::take(&mut joins));
                item = self.match_recognize(input)?;
            } else {
                return Ok((item, joins));
            }
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
            let nearest: NearestBy = self.nearest_by()?;
            let mut node: Join = join(right, kind, JoinConstraint::None, (inner, outer));
            node.nearest_by = Some(Box::new(nearest));
            return Ok(node);
        } else {
            JoinConstraint::None
        };
        Ok(join(right, kind, constraint, (inner, outer)))
    }

    /// `{APPROX | EXACT} NEAREST [num_results] BY {DISTANCE | SIMILARITY} expression`.
    fn nearest_by(&mut self) -> Parsing<NearestBy> {
        let exact: bool = self.kw("EXACT");
        self.at += 2;
        let results: Option<Expression> = match self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Number(NumberKind::Integer))
        {
            true => Some(self.primary_expression()?),
            false => None,
        };
        self.expect_kw("BY")?;
        let similarity: bool = if self.eat_kw("DISTANCE") {
            false
        } else if self.eat_kw("SIMILARITY") {
            true
        } else {
            return Err(self.error());
        };
        Ok(NearestBy {
            exact,
            results,
            similarity,
            by: self.expression()?,
        })
    }

    /// `tableAlias`: `[AS] strictIdentifier [(columns)]`, when present.
    pub(crate) fn table_alias(&mut self) -> Parsing<Option<TableAlias>> {
        let explicit: bool = self.eat_kw("AS");
        if !explicit && !self.bare_alias_at(0, true) {
            return Ok(None);
        }
        let name: Identifier = self.name(is_strict_identifier)?;
        let columns: Vec<Identifier> = match self.is_char('(') && self.is_at(1, is_identifier) {
            true => self.identifier_list()?,
            false => Vec::new(),
        };
        Ok(Some(TableAlias {
            name,
            columns,
            explicit,
        }))
    }

    /// `relationPrimary`, `LATERAL` when it followed `LATERAL`.
    pub(crate) fn relation_primary(&mut self, is_lateral: bool) -> Parsing<Expression> {
        let first: usize = self.at;
        let node: Expression = self.deeper(|parser| parser.relation_primary_inner(is_lateral))?;
        Ok(self.spanned(first, node))
    }

    /// What `STREAM ( … )` reads: a table name, or a table function call, with no alias.
    fn stream_source(&mut self) -> Parsing<Expression> {
        if self.kw("IDENTIFIER") && self.char_at(1, '(') {
            self.at += 2;
            let name: Expression = self.expression()?;
            self.expect_char(')')?;
            return Ok(call(IDENTIFIER_CLAUSE, vec![name]));
        }
        let name: Vec<Identifier> = self.multipart_identifier()?;
        match self.is_char('(') {
            true => self.function_call(name),
            false => Ok(Expression::Table(Box::new(table(name)))),
        }
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
            if self.eat_char('(') {
                let inner: Expression = self.stream_source()?;
                self.expect_char(')')?;
                let watermark: Option<Box<Watermark>> = self.watermark()?.map(Box::new);
                let alias: Option<TableAlias> = self.table_alias()?;
                let stream: Expression = Expression::Stream(Box::new(Stream {
                    this: paren(inner),
                    on: None,
                    show_initial_rows: None,
                    watermark,
                }));
                return Ok(aliased(stream, alias));
            }
            let inner: Expression = self.deeper(|parser| parser.relation_primary_inner(false))?;
            return Ok(Expression::Stream(Box::new(Stream {
                this: inner,
                on: None,
                show_initial_rows: None,
                watermark: None,
            })));
        }
        if self.is_char('(') && self.kw_in(1, &["DESC", "DESCRIBE"]) {
            self.at += 1;
            let describe: Expression = self.describe_statement()?;
            self.expect_char(')')?;
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(subquery(describe, alias, is_lateral));
        }
        if self.is_char('(') {
            return self.parenthesized_relation(start, is_lateral);
        }
        if self.kw("VALUES") {
            self.at += 1;
            let items: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
            let bare_rows: Vec<usize> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !matches!(item, Expression::Tuple(_) | Expression::Paren(_)))
                .map(|(at, _)| at)
                .collect();
            let rows: Vec<Vec<Expression>> = items.into_iter().map(row).collect();
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(values(rows, bare_rows, alias));
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
        let name: Vec<Identifier> = self.multipart_identifier()?;
        if self.is_char('(') {
            let call: Expression = self.function_call(name)?;
            if self.kw("IDENTIFIED") && self.kw_at(1, "BY") {
                let clause: usize = self.offset();
                self.at += 2;
                self.ident()?;
                self.gap("identified_by", clause);
            }
            self.watermark_gap()?;
            let alias: Option<TableAlias> = self.table_alias()?;
            return Ok(match is_lateral {
                true => lateral(call, alias),
                false => aliased(call, alias),
            });
        }
        self.named_relation(name)
    }

    /// A parenthesized `relationPrimary` from `start`: a query or a relation, and its alias.
    fn parenthesized_relation(&mut self, start: usize, is_lateral: bool) -> Parsing<Expression> {
        if self.starts_query_at(1) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.query()?;
                parser.expect_char(')')?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                let table_sample: Option<Sample> = match self.kw("TABLESAMPLE") {
                    true => Some(self.sample()?),
                    false => None,
                };
                let watermark: Option<Watermark> = self.watermark()?;
                let alias: Option<TableAlias> = self.table_alias()?;
                let mut node: Expression = subquery(query, alias, is_lateral);
                if let Expression::Subquery(ref mut derived) = node {
                    derived.table_sample = table_sample.map(Box::new);
                    derived.watermark = watermark.map(Box::new);
                }
                return Ok(node);
            }
        }
        self.at += 1;
        let (item, joins) = self.relation()?;
        self.expect_char(')')?;
        self.trailing_relation_clauses(start, "parenthesized_relation_sample")?;
        let alias: Option<TableAlias> = self.table_alias()?;
        Ok(match (joins.is_empty(), alias) {
            (false, Some(alias)) if alias.columns.is_empty() => {
                joined_table(item, joins, Some(alias.name))
            }
            (_, alias) => aliased(collapse(item, joins), alias),
        })
    }

    /// A table named `name` and its temporal, sampling, watermark and alias clauses.
    fn named_relation(&mut self, name: Vec<Identifier>) -> Parsing<Expression> {
        let mut relation: TableRef = table(name);
        if self.kw("CHANGES") && self.kw_at(1, "FROM") {
            let clause: usize = self.offset();
            self.at += 2;
            self.skip_changes()?;
            self.gap("changes", clause);
        } else {
            relation.version = self.temporal_clause()?;
        }
        if self.kw("WITH") && self.char_at(1, '(') {
            let clause: usize = self.offset();
            self.at += 1;
            self.property_list()?;
            self.gap("relation_options", clause);
        }
        if self.kw("TABLESAMPLE") {
            relation.table_sample = Some(Box::new(self.sample()?));
        }
        relation.watermark = self.watermark()?.map(Box::new);
        if let Some(alias) = self.table_alias()? {
            relation.alias = Some(alias.name);
            relation.alias_explicit_as = alias.explicit;
            relation.column_aliases = alias.columns;
        }
        Ok(Expression::Table(Box::new(relation)))
    }

    /// `sample? watermarkClause?` after a parenthesized relation, as gaps.
    fn trailing_relation_clauses(&mut self, start: usize, construct: &'static str) -> Parsing<()> {
        if self.kw("TABLESAMPLE") {
            self.sample()?;
            self.gap(construct, start);
        }
        self.watermark_gap()
    }

    /// A `watermarkClause` where the tree has no place for one, as a gap.
    fn watermark_gap(&mut self) -> Parsing<()> {
        let start: usize = self.offset();
        if self.watermark()?.is_some() {
            self.gap("watermark", start);
        }
        Ok(())
    }

    /// `watermarkClause`: `WATERMARK namedExpression DELAY OF interval`, when written.
    fn watermark(&mut self) -> Parsing<Option<Watermark>> {
        if !self.kw("WATERMARK") || self.terminal_at(1) {
            return Ok(None);
        }
        self.at += 1;
        let this: Expression = self.expression()?;
        let alias_explicit_as: bool = self.eat_kw("AS");
        let aliases_parenthesized: bool = alias_explicit_as && self.is_char('(');
        let aliases: Vec<Identifier> = if aliases_parenthesized {
            self.identifier_list()?
        } else if alias_explicit_as
            || (self.is(is_identifier) && !(self.kw("DELAY") && self.kw_at(1, "OF")))
        {
            vec![self.ident()?]
        } else {
            Vec::new()
        };
        self.expect_kw("DELAY")?;
        self.expect_kw("OF")?;
        if !self.kw("INTERVAL") {
            return Err(self.error());
        }
        Ok(Some(Watermark {
            this,
            aliases,
            alias_explicit_as,
            aliases_parenthesized,
            delay: self.primary_expression()?,
        }))
    }

    /// The rest of a `changesClause` after `CHANGES FROM`.
    fn skip_changes(&mut self) -> Parsing<()> {
        let bound = |parser: &mut Self| -> Parsing<()> {
            if parser.eat_kw("SYSTEM_VERSION") || parser.eat_kw("VERSION") {
                parser.version_value()?;
            } else if parser.eat_kw("SYSTEM_TIME") || parser.eat_kw("TIMESTAMP") {
                parser.value_expression(0)?;
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

    /// `temporalClause`: `[FOR] VERSION|TIMESTAMP AS OF ...`, when present.
    pub(crate) fn temporal_clause(&mut self) -> Parsing<Option<Box<Version>>> {
        let offset: usize = usize::from(self.kw("FOR"));
        let word: Option<String> = self.word_at(offset);
        let kind: &str = match word.as_deref() {
            Some(kind @ ("SYSTEM_VERSION" | "VERSION" | "SYSTEM_TIME" | "TIMESTAMP"))
                if self.kw_at(offset + 1, "AS") && self.kw_at(offset + 2, "OF") =>
            {
                kind
            }
            _ => return Ok(None),
        };
        let by_version: bool = kind.ends_with("VERSION");
        let kind: String = kind.to_owned();
        self.at += offset + 3;
        let value: Expression = match by_version {
            true => self.version_value()?,
            false => self.value_expression(0)?,
        };
        Ok(Some(Box::new(Version {
            this: Box::new(Expression::Identifier(Identifier::new(kind))),
            kind: "AS OF".to_owned(),
            expression: Some(Box::new(value)),
            for_keyword: offset == 1,
        })))
    }

    /// Delta's `@v<version>` or `@<yyyyMMddHHmmssSSS>`, written against the table name before it.
    pub(crate) fn at_version(&mut self) -> Parsing<Option<Box<Version>>> {
        let touches: bool = self.at > 0
            && self.is_char('@')
            && self.tokens[self.at - 1].end == self.tokens[self.at].start
            && self.adjacent(0);
        if !touches {
            return Ok(None);
        }
        self.at += 1;
        let token = self.advance()?;
        let digits =
            |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
        let (kind, value): (&str, &str) = match token.kind {
            TokenKind::Word
                if token.value.len() > 1
                    && token.value.starts_with(['v', 'V'])
                    && digits(&token.value[1..]) =>
            {
                ("VERSION", &token.value[1..])
            }
            TokenKind::Number(NumberKind::Integer) if token.value.len() == AT_TIMESTAMP_DIGITS => {
                ("TIMESTAMP", token.value.as_str())
            }
            _ => return Err(self.error_with("Syntax error at or near '@'", token.start, true)),
        };
        Ok(Some(Box::new(Version {
            this: Box::new(Expression::Identifier(Identifier::new(kind))),
            kind: "@".to_owned(),
            expression: Some(Box::new(string(value))),
            for_keyword: false,
        })))
    }

    /// `sample`: `TABLESAMPLE [SYSTEM|BERNOULLI] ( method ) [REPEATABLE (seed)]`.
    pub(crate) fn sample(&mut self) -> Parsing<Sample> {
        self.expect_kw("TABLESAMPLE")?;
        let explicit_method: bool = self.eat_kw("SYSTEM") || self.eat_kw("BERNOULLI");
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
                let percent: Option<f64> = match &size {
                    Expression::Literal(number) => percent_outside_range(number.value_str(), false),
                    Expression::Neg(negated) => match &negated.this {
                        Expression::Literal(number) => {
                            percent_outside_range(number.value_str(), true)
                        }
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(percent) = percent {
                    let message: String = format!(
                        "[INVALID_TABLESAMPLE_FRACTION] Sampling fraction ({}) must be on interval [0, 1]",
                        percent / 100.0
                    );
                    return Err(self.error_with(&message, self.offset(), false));
                }
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
            self.expect_char(')')?;
        }
        Ok(sample)
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

    /// The alias after a pivot, unpivot or match recognition: `[AS] errorCapturingIdentifier`.
    pub(crate) fn pivot_alias(&mut self) -> Parsing<Option<Identifier>> {
        if self.eat_kw("AS") {
            return Ok(Some(self.ident()?));
        }
        match self.bare_alias_at(0, false) && !self.join_ahead() {
            true => Ok(Some(self.ident()?)),
            false => Ok(None),
        }
    }

    /// Whether the words here lead to `JOIN` through the words a join type is made of.
    fn join_ahead(&self) -> bool {
        let mut offset: usize = 0;
        while self.kw_in(
            offset,
            &[
                "INNER", "CROSS", "LEFT", "RIGHT", "FULL", "SEMI", "ANTI", "NATURAL", "OUTER",
            ],
        ) {
            offset += 1;
        }
        self.kw_at(offset, "JOIN")
    }

    fn pivot(&mut self, input: Expression) -> Parsing<Expression> {
        self.expect_char('(')?;
        let aggregates: Vec<Expression> = self.comma_list(|parser| parser.named_expression())?;
        self.expect_kw("FOR")?;
        let column: Expression = match self.is_char('(') {
            true => tuple(
                self.identifier_list()?
                    .into_iter()
                    .map(|name| column_from(vec![name]))
                    .collect(),
            ),
            false => column_from(vec![self.ident()?]),
        };
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

    /// An unpivoted column with its optional alias.
    fn unpivot_column(&mut self) -> Parsing<Expression> {
        let column: Expression = column_from(self.multipart_identifier()?);
        let explicit: bool = self.eat_kw("AS");
        match explicit || self.bare_alias_at(0, false) {
            true => Ok(alias(column, self.ident()?, explicit)),
            false => Ok(column),
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
        let mut value_columns: Vec<Identifier> = match parenthesized {
            true => self.identifier_list()?,
            false => vec![self.ident()?],
        };
        self.expect_kw("FOR")?;
        let name_column: Identifier = self.ident()?;
        self.expect_kw("IN")?;
        let columns: Vec<Expression> = self.parenthesized(|parser| match parenthesized {
            true => {
                let set: Vec<Expression> = parser
                    .parenthesized(|parser| Ok(column_from(parser.multipart_identifier()?)))?;
                let explicit: bool = parser.eat_kw("AS");
                match explicit || parser.bare_alias_at(0, false) {
                    true => Ok(alias(tuple(set), parser.ident()?, explicit)),
                    false => Ok(tuple(set)),
                }
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
            let table_alias: Option<Identifier> = match self.kw("AS") {
                true => None,
                false => Some(self.ident()?),
            };
            let mut column_aliases: Vec<Identifier> = Vec::new();
            let column_aliases_explicit_as: bool = self.eat_kw("AS");
            if column_aliases_explicit_as || self.bare_alias_at(0, false) {
                column_aliases.push(self.ident()?);
                while self.is_char(',') && self.bare_alias_at(1, false) {
                    self.at += 1;
                    column_aliases.push(self.ident()?);
                }
            }
            views.push(LateralView {
                this: Expression::Function(Box::new(function(name, args))),
                table_alias,
                column_aliases,
                column_aliases_explicit_as,
                outer,
            });
        }
        Ok(views)
    }

    /// `propertyList`: `( key [=] value, ... )`, as `key = value` pairs.
    pub(crate) fn property_list(&mut self) -> Parsing<Vec<Expression>> {
        Ok(self
            .spelled_property_list()?
            .into_iter()
            .map(|(property, _)| property)
            .collect())
    }

    /// `propertyList`, each property with whether its value followed the key without `=`.
    pub(crate) fn spelled_property_list(&mut self) -> Parsing<Vec<(Expression, bool)>> {
        self.parenthesized(|parser| parser.property(false))
    }

    /// `property` (with `expressions`, `expressionProperty`) and whether its value lacks an `=`.
    pub(crate) fn property(&mut self, expressions: bool) -> Parsing<(Expression, bool)> {
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
            Some(value) => (
                crate::parse::_helpers::tree::nodes::binary(Expression::Eq, key, value),
                !equals,
            ),
            None => (key, false),
        })
    }

    /// `propertyValue`: an integer, decimal, boolean or string.
    fn property_value(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number(NumberKind::Integer | NumberKind::Decimal))
            | Some(TokenKind::String { .. }) => self.primary_expression(),
            Some(TokenKind::Word) if self.kw_in(0, &["TRUE", "FALSE"]) => self.primary_expression(),
            _ => Err(self.error()),
        }
    }
}
