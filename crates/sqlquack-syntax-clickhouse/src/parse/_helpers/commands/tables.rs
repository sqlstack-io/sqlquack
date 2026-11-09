//! `ParserTablesInSelectQuery`: table expressions, joins, `ARRAY JOIN`, `FINAL` and `SAMPLE`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    From, Join, JoinKind, Sample, SampleMethod, Select,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, join, subquery, table, tuple,
};
use crate::parse::constants::{INNER_JOIN, MAX_TABLE_NAME_PARTS};
use crate::scan::main::unquote::unquote;
use crate::scan::models::TokenKind;

/// `ANY`, `ALL`, `ASOF`, `SEMI` or `ANTI`, as `parseJoinStrictness` reads them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Strictness {
    Unspecified,
    Any,
    All,
    AsOf,
    Semi,
    Anti,
}

/// The shared-tree join kind of a ClickHouse kind and strictness; `ANY` has none.
fn join_kind(kind: &str, strictness: Strictness) -> Option<JoinKind> {
    let kind: JoinKind = match (kind, strictness) {
        ("INNER", Strictness::AsOf) => JoinKind::AsOf,
        ("LEFT", Strictness::AsOf) => JoinKind::AsOfLeft,
        ("RIGHT", Strictness::AsOf) => JoinKind::AsOfRight,
        ("LEFT", Strictness::Semi) => JoinKind::LeftSemi,
        ("RIGHT", Strictness::Semi) => JoinKind::RightSemi,
        ("LEFT", Strictness::Anti) => JoinKind::LeftAnti,
        ("RIGHT", Strictness::Anti) => JoinKind::RightAnti,
        (_, Strictness::Any | Strictness::AsOf) => return None,
        ("INNER", _) => JoinKind::Inner,
        ("LEFT", _) => JoinKind::Left,
        ("RIGHT", _) => JoinKind::Right,
        ("FULL", _) => JoinKind::Full,
        ("CROSS", _) => JoinKind::Cross,
        ("PASTE", _) => JoinKind::Paste,
        _ => return None,
    };
    Some(kind)
}

/// Whether `item` is a name, plain or qualified.
fn is_name(item: &Expression) -> bool {
    match item {
        Expression::Column(_) => true,
        Expression::Dot(dot) => is_name(&dot.this),
        _ => false,
    }
}

/// The column a `USING` item names: a name's last part, or its alias, as Polyglot keeps it.
fn using_name(item: &Expression) -> Option<Identifier> {
    match item {
        Expression::Column(column) => Some(column.name.clone()),
        Expression::Dot(dot) if is_name(&dot.this) => Some(dot.field.clone()),
        Expression::Alias(alias) if is_name(&alias.this) => Some(alias.alias.clone()),
        _ => None,
    }
}

impl Parser<'_> {
    /// `ParserJoinStrictness`, if one is next.
    fn strictness(&mut self) -> Strictness {
        let strictness: Strictness = match self.word_at(0).as_deref() {
            Some("ANY") => Strictness::Any,
            Some("ALL") => Strictness::All,
            Some("ASOF") => Strictness::AsOf,
            Some("SEMI") => Strictness::Semi,
            Some("ANTI") => Strictness::Anti,
            _ => return Strictness::Unspecified,
        };
        self.at += 1;
        strictness
    }

    /// `ParserTablesInSelectQuery`: the first table expression and every element after it.
    pub(crate) fn tables(&mut self, mut select: Select, bare_alias: bool) -> Parsing<Select> {
        let first: Expression = self.table_expression(bare_alias)?;
        let mut sources: Vec<Expression> = vec![first];
        let mut joins: Vec<Join> = Vec::new();
        while let Some(element) = self.table_element(bare_alias)? {
            match element {
                Ok(join) => joins.push(join),
                Err(source) if joins.is_empty() => sources.push(source),
                Err(source) => joins.push(join(
                    source,
                    JoinKind::Cross,
                    JoinConstraint::None,
                    (false, false),
                )),
            }
        }
        select.from = Some(From {
            expressions: sources,
        });
        select.joins = joins;
        Ok(select)
    }

    /// The next `ARRAY JOIN`, join or `, table` (as `Err`); `None` when none starts here.
    fn table_element(&mut self, bare_alias: bool) -> Parsing<Option<Result<Join, Expression>>> {
        let start: usize = self.offset();
        if let Some(kind) = self.array_join_kind() {
            let items: Vec<Expression> = self.comma_list(|parser| parser.element())?;
            let this: Expression = match items.len() {
                1 => items
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| tuple(Vec::new())),
                _ => tuple(items),
            };
            return Ok(Some(Ok(join(
                this,
                kind,
                JoinConstraint::None,
                (false, false),
            ))));
        }
        if self.eat(TokenKind::Comma) {
            return Ok(Some(Err(self.table_expression(bare_alias)?)));
        }
        let at: usize = self.at;
        let locality: bool = self.eat_kw("GLOBAL") || self.eat_kw("LOCAL");
        let mut strictness: Strictness = self.strictness();
        let kind: Option<String> = self.word_at(0).filter(|word| {
            ["INNER", "LEFT", "RIGHT", "FULL", "CROSS", "PASTE"].contains(&word.as_str())
        });
        let named: bool = kind.is_some();
        self.at += usize::from(named);
        if strictness == Strictness::Unspecified {
            strictness = self.strictness();
        }
        let kind: String = kind.unwrap_or_else(|| match strictness {
            Strictness::Semi | Strictness::Anti => "LEFT".to_owned(),
            _ => "INNER".to_owned(),
        });
        let outer: bool =
            ["LEFT", "RIGHT", "FULL"].contains(&kind.as_str()) && self.eat_kw("OUTER");
        if !self.kw("JOIN") {
            self.at = at;
            return Ok(None);
        }
        if strictness != Strictness::Unspecified && ["CROSS", "PASTE"].contains(&kind.as_str()) {
            return Err(self.error_with("You must not specify ANY or ALL for this JOIN.", true));
        }
        if matches!(strictness, Strictness::Semi | Strictness::Anti)
            && !["LEFT", "RIGHT"].contains(&kind.as_str())
        {
            return Err(self.error_with("SEMI|ANTI JOIN should be LEFT or RIGHT.", true));
        }
        self.at += 1;
        let source: Expression = self.table_expression(bare_alias)?;
        let constraint: JoinConstraint = match kind.as_str() {
            "CROSS" | "PASTE" => JoinConstraint::None,
            _ => self.join_constraint()?,
        };
        let shared: Option<JoinKind> = join_kind(&kind, strictness);
        if locality || shared.is_none() {
            self.gap("join_locality_or_strictness", start);
        }
        let joined: JoinKind = shared.unwrap_or(JoinKind::Inner);
        Ok(Some(Ok(join(
            source,
            joined,
            constraint,
            (kind == INNER_JOIN && named, outer),
        ))))
    }

    /// `LEFT ARRAY JOIN`, `[INNER] ARRAY JOIN`, consumed.
    fn array_join_kind(&mut self) -> Option<JoinKind> {
        if self.eat_kws(&["LEFT", "ARRAY", "JOIN"]) {
            return Some(JoinKind::LeftArray);
        }
        if self.eat_kws(&["ARRAY", "JOIN"]) || self.eat_kws(&["INNER", "ARRAY", "JOIN"]) {
            return Some(JoinKind::Array);
        }
        None
    }

    /// `USING (cols)`, `USING cols` or `ON expr`.
    fn join_constraint(&mut self) -> Parsing<JoinConstraint> {
        if self.eat_kw("USING") {
            let start: usize = self.offset();
            let bracketed: bool = self.eat(TokenKind::OpeningRoundBracket);
            let items: Vec<Expression> = self.comma_list(|parser| parser.element())?;
            if bracketed {
                self.expect_kind(TokenKind::ClosingRoundBracket)?;
            }
            let names: Option<Vec<Identifier>> = items.iter().map(using_name).collect();
            return Ok(JoinConstraint::Using(match names {
                Some(names) => names,
                None => {
                    self.gap("using_expression", start);
                    Vec::new()
                }
            }));
        }
        self.expect_kw("ON")?;
        Ok(JoinConstraint::On(Box::new(self.expression()?)))
    }

    /// `ParserTableExpression`: a subquery, table function or name, with alias, FINAL and SAMPLE.
    pub(crate) fn table_expression(&mut self, bare_alias: bool) -> Parsing<Expression> {
        let first: usize = self.at;
        let start: usize = self.offset();
        let mut source: Expression = self.table_source(bare_alias)?;
        let final_: bool = self.eat_kw("FINAL");
        let sample: Option<Sample> = match self.eat_kw("SAMPLE") {
            true => Some(self.sample_ratio()?),
            false => None,
        };
        match &mut source {
            Expression::Table(table) => {
                table.final_ = final_;
                table.table_sample = sample.map(Box::new);
            }
            _ if final_ || sample.is_some() => self.gap("final_or_sample_on_non_table", start),
            _ => {}
        }
        Ok(self.spanned(first, source))
    }

    fn table_source(&mut self, bare_alias: bool) -> Parsing<Expression> {
        if self.subquery_at(0) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.select_with_union()?;
                parser.expect_kind(TokenKind::ClosingRoundBracket)?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                let alias: Option<TableAlias> = self.table_alias(bare_alias)?;
                return Ok(subquery(query, alias, false));
            }
        }
        if self.is(TokenKind::StringLiteral) {
            let name: String = unquote(&self.advance()?.value);
            let mut node: TableRef = table(vec![Identifier::quoted(name)]);
            node = self.apply_table_alias(node, bare_alias)?;
            return Ok(Expression::Table(Box::new(node)));
        }
        let parts: Vec<Identifier> = self.compound_identifier()?;
        if self.eat(TokenKind::OpeningRoundBracket) {
            let call: Expression = self.function_layer(parts)?;
            return Ok(match self.table_alias(bare_alias)? {
                Some(alias) => {
                    let mut node: Alias = Alias::new(call, alias.name);
                    node.alias_explicit_as = alias.explicit;
                    Expression::Alias(Box::new(node))
                }
                None => call,
            });
        }
        if parts.len() > MAX_TABLE_NAME_PARTS {
            return Err(self.error());
        }
        let mut node: TableRef = table(parts);
        node = self.apply_table_alias(node, bare_alias)?;
        Ok(Expression::Table(Box::new(node)))
    }

    fn apply_table_alias(&mut self, mut node: TableRef, bare_alias: bool) -> Parsing<TableRef> {
        if let Some(alias) = self.table_alias(bare_alias)? {
            node.alias = Some(alias.name);
            node.alias_explicit_as = alias.explicit;
        }
        Ok(node)
    }

    /// `ParserAlias` after a table expression.
    fn table_alias(&mut self, bare_alias: bool) -> Parsing<Option<TableAlias>> {
        Ok(self
            .alias_name(bare_alias)?
            .map(|(name, explicit)| TableAlias {
                name,
                columns: Vec::new(),
                explicit,
            }))
    }

    /// `ParserSampleRatio`: a ratio `n` or `n/m`, and `OFFSET` with another.
    fn sample_ratio(&mut self) -> Parsing<Sample> {
        let size: Expression = self.ratio()?;
        let offset: Option<Expression> = match self.eat_kw("OFFSET") {
            true => Some(self.ratio()?),
            false => None,
        };
        Ok(Sample {
            method: SampleMethod::Block,
            size,
            seed: None,
            offset,
            unit_after_size: false,
            use_sample_keyword: true,
            explicit_method: false,
            method_before_size: false,
            use_seed_keyword: false,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            is_using_sample: false,
            is_percent: false,
            suppress_method_output: true,
        })
    }

    /// One rational number: digits, optionally `/` digits.
    fn ratio(&mut self) -> Parsing<Expression> {
        let numerator: Expression = match self.is(TokenKind::Number) {
            true => self.operand()?,
            false => return Err(self.error()),
        };
        if self.is(TokenKind::Slash) && self.kind_at(1, TokenKind::Number) {
            self.at += 1;
            let denominator: Expression = self.operand()?;
            return Ok(crate::parse::_helpers::tree::nodes::binary(
                Expression::Div,
                numerator,
                denominator,
            ));
        }
        Ok(numerator)
    }
}
