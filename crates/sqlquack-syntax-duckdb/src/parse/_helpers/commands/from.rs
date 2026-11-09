//! `FromClause` and `TableRef` (select.gram): table references, joins, pivots and samples.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::queries::models::{
    From, Join, JoinKind, Pivot, Sample, SampleMethod, Select, Unpivot,
};
use sqlquack_syntax::expressions::statements::models::{HistoricalData, TableSample};

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_col_label, is_function_name, is_identifier, is_type_func_name,
};
use crate::parse::_helpers::tree::nodes::{
    JoinConstraint, TableAlias, boolean, function, join, joined_table, lateral, subquery, table,
    values,
};
use crate::parse::constants::INNER_JOIN;

/// The time travel `AT` names, which the tree keeps as `HistoricalData`.
const AT_WORD: &str = "AT";

/// What `AT (unit => value)` travels by.
const AT_UNITS: [&str; 2] = ["VERSION", "TIMESTAMP"];

/// The transformer's refusals of a sample: of a lateral reference, and of its size.
const LATERAL_FUNCTION_SAMPLE: &str = "TABLESAMPLE cannot be applied to a LATERAL table function";
const LATERAL_SUBQUERY_SAMPLE: &str = "TABLESAMPLE cannot be applied to a LATERAL subquery";
const SAMPLE_CONSTANT: &str = "Only constants are supported in sample clause currently";
/// `SampleOptions::MAX_SAMPLE_ROWS`.
const MAX_SAMPLE_ROWS: i128 = 1_000_000_000;
/// The first double past `int64`: such a row count fails its cast, not the parser.
const I64_LIMIT: f64 = 9_223_372_036_854_775_808.0;
use crate::scan::models::{Token, TokenKind};

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

/// A table name token: `TableName` takes a quoted name, a non-reserved word or a string (a path).
pub(crate) fn is_table_name(token: &Token) -> bool {
    is_identifier(token)
        || token.kind == TokenKind::String(crate::scan::models::StringPrefix::Plain)
}

impl Parser<'_> {
    /// `FromClause`: `FROM TableRef, ...`, into `select`'s sources and joins.
    pub(crate) fn select_from_clause(&mut self, mut select: Select) -> Parsing<Select> {
        self.expect_kw("FROM")?;
        let mut refs: Vec<(Expression, Vec<Join>)> =
            self.comma_list(|parser| parser.table_ref())?;
        let (first, joins) = refs.remove(0);
        let mut items: Vec<Expression> = vec![first];
        items.extend(refs.into_iter().map(|(item, more)| collapse(item, more)));
        select.from = Some(From { expressions: items });
        select.joins = joins;
        Ok(select)
    }

    /// `TableRef`: `InnerTableRef` and its joins, pivots and unpivots.
    pub(crate) fn table_ref(&mut self) -> Parsing<(Expression, Vec<Join>)> {
        let mut item: Expression = self.inner_table_ref()?;
        let mut joins: Vec<Join> = Vec::new();
        loop {
            if self.kw("PIVOT") && self.char_at(1, '(') {
                let input: Expression = collapse(item, std::mem::take(&mut joins));
                item = self.table_pivot(input)?;
            } else if self.kw("UNPIVOT") {
                let input: Expression = collapse(item, std::mem::take(&mut joins));
                item = self.table_unpivot(input)?;
            } else if self.at_join() {
                let first: usize = self.at;
                let mut node: Join = self.deeper(|parser| parser.join_clause())?;
                node.span = self.source_range(first);
                joins.push(node);
            } else {
                return Ok((item, joins));
            }
        }
    }

    /// Whether a `JoinClause` begins at the cursor.
    pub(crate) fn at_join(&self) -> bool {
        let offset: usize = usize::from(self.kw("ASOF"));
        let typed: usize = match self.word_at(offset).as_deref() {
            Some("FULL" | "LEFT" | "RIGHT") => 1 + usize::from(self.kw_at(offset + 1, "OUTER")),
            Some("SEMI" | "ANTI" | "INNER") => 1,
            Some("CROSS" | "NATURAL" | "POSITIONAL") if offset == 0 => return true,
            _ => 0,
        };
        self.kw_at(offset + typed, "JOIN")
    }

    /// `JoinType?`: the kind it names, and whether `INNER` and `OUTER` were written.
    fn join_type(&mut self) -> (Option<JoinKind>, bool, bool) {
        let word: String = self.word_at(0).unwrap_or_default();
        let kind: JoinKind = match word.as_str() {
            "FULL" => JoinKind::Full,
            "LEFT" => JoinKind::Left,
            "RIGHT" => JoinKind::Right,
            "SEMI" => JoinKind::Semi,
            "ANTI" => JoinKind::Anti,
            "INNER" => JoinKind::Inner,
            _ => return (None, false, false),
        };
        self.at += 1;
        let outer: bool = matches!(kind, JoinKind::Full | JoinKind::Left | JoinKind::Right)
            && self.eat_kw("OUTER");
        (Some(kind), word == INNER_JOIN, outer)
    }

    /// `JoinClause`: a join with `ON`/`USING`, or a `CROSS`, `NATURAL` or `POSITIONAL` one.
    fn join_clause(&mut self) -> Parsing<Join> {
        let prefix: Option<JoinKind> = match self.word_at(0).as_deref() {
            Some("CROSS") => Some(JoinKind::Cross),
            Some("POSITIONAL") => Some(JoinKind::Positional),
            Some("NATURAL") => Some(JoinKind::Natural),
            _ => None,
        };
        if let Some(prefix) = prefix {
            self.at += 1;
            let (kind, inner, outer) = match prefix {
                JoinKind::Natural => self.join_type(),
                _ => (None, false, false),
            };
            let kind: JoinKind = match (prefix, kind) {
                (JoinKind::Natural, Some(JoinKind::Left)) => JoinKind::NaturalLeft,
                (JoinKind::Natural, Some(JoinKind::Right)) => JoinKind::NaturalRight,
                (JoinKind::Natural, Some(JoinKind::Full)) => JoinKind::NaturalFull,
                (JoinKind::Natural, Some(JoinKind::Semi)) => JoinKind::NaturalSemi,
                (JoinKind::Natural, Some(JoinKind::Anti)) => JoinKind::NaturalAnti,
                (prefix, _) => prefix,
            };
            self.expect_kw("JOIN")?;
            let right: Expression = self.inner_table_ref()?;
            return Ok(join(right, kind, JoinConstraint::None, (inner, outer)));
        }
        let asof: bool = self.eat_kw("ASOF");
        let (kind, inner, outer) = self.join_type();
        self.expect_kw("JOIN")?;
        let start: usize = self.offset();
        if !asof && kind.is_none() && self.kw("BY") && self.char_at(1, '(') {
            self.at += 2;
            self.expect_kw("TYPE")?;
            self.name(is_col_label)?;
            self.expect_char(')')?;
            let (right, nested) = self.table_ref()?;
            let right: Expression = collapse(right, nested);
            self.join_qualifier()?;
            self.gap("join_by", start);
            return Ok(join(
                right,
                JoinKind::Inner,
                JoinConstraint::None,
                (false, false),
            ));
        }
        let kind_or_inner: JoinKind = kind.unwrap_or(JoinKind::Inner);
        if !asof {
            let bare = self.attempt(|parser| {
                let right: TableRef = parser.base_table_name()?;
                parser.nearest_tail()?;
                Ok(right)
            });
            if let Ok(right) = bare {
                self.gap("nearest_join", start);
                let right: Expression = Expression::Table(Box::new(right));
                return Ok(join(
                    right,
                    kind_or_inner,
                    JoinConstraint::None,
                    (inner, outer),
                ));
            }
        }
        let (right, nested) = self.table_ref()?;
        let right: Expression = collapse(right, nested);
        if !asof && self.kw_in(0, &["APPROX", "EXACT", "NEAREST"]) {
            self.nearest_tail()?;
            self.gap("nearest_join", start);
            return Ok(join(
                right,
                kind_or_inner,
                JoinConstraint::None,
                (inner, outer),
            ));
        }
        let kind: JoinKind = match (asof, kind) {
            (true, Some(JoinKind::Left)) => JoinKind::AsOfLeft,
            (true, Some(JoinKind::Right)) => JoinKind::AsOfRight,
            (true, Some(JoinKind::Full)) => JoinKind::AsOfFull,
            (true, Some(JoinKind::Semi)) => JoinKind::AsOfSemi,
            (true, Some(JoinKind::Anti)) => JoinKind::AsOfAnti,
            (true, _) => JoinKind::AsOf,
            (false, kind) => kind.unwrap_or(JoinKind::Inner),
        };
        let constraint: JoinConstraint = self.join_qualifier()?;
        Ok(join(right, kind, constraint, (inner, outer)))
    }

    /// `JoinQualifier`: `ON expression` or `USING (columns)`.
    pub(crate) fn join_qualifier(&mut self) -> Parsing<JoinConstraint> {
        if self.eat_kw("ON") {
            return Ok(JoinConstraint::On(Box::new(self.expression()?)));
        }
        if self.eat_kw("USING") {
            let columns = self.parenthesized(|parser| parser.col_id_or_string())?;
            return Ok(JoinConstraint::Using(columns));
        }
        Err(self.error())
    }

    /// `[APPROX|EXACT] NEAREST [n] BY DISTANCE|SIMILARITY expression`, after a join's target.
    fn nearest_tail(&mut self) -> Parsing<()> {
        if !self.eat_kw("APPROX") {
            self.eat_kw("EXACT");
        }
        self.expect_kw("NEAREST")?;
        if self.is_kind_number() {
            self.at += 1;
        }
        self.expect_kw("BY")?;
        if !(self.eat_kw("DISTANCE") || self.eat_kw("SIMILARITY")) {
            return Err(self.error());
        }
        self.expression()?;
        Ok(())
    }

    /// Whether the current token is a number.
    fn is_kind_number(&self) -> bool {
        self.peek()
            .is_some_and(|token| token.kind == TokenKind::Number)
    }

    /// `TableAlias`: `AS name [(columns)]` or `name [(columns)]`, when present.
    pub(crate) fn table_alias(&mut self) -> Parsing<Option<TableAlias>> {
        let explicit: bool = self.eat_kw("AS");
        let name: Identifier = match explicit {
            true => self.col_id_or_string()?,
            false if self.is(is_identifier) => self.name(is_identifier)?,
            false => return Ok(None),
        };
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.col_id_or_string())?,
            false => Vec::new(),
        };
        Ok(Some(TableAlias {
            name,
            columns,
            explicit,
        }))
    }

    /// `TableAliasColon`: `name :` before a table reference, when present.
    fn alias_colon(&mut self) -> Parsing<Option<TableAlias>> {
        let named: bool = (self.is(is_identifier) || self.string_at(0)) && self.char_at(1, ':');
        if !named || self.op_at(1, "::") {
            return Ok(None);
        }
        let name: Identifier = self.col_id_or_string()?;
        self.at += 1;
        Ok(Some(TableAlias {
            name,
            columns: Vec::new(),
            explicit: true,
        }))
    }

    /// The alias after a reference: the one written before `:`, or a trailing one.
    fn trailing_alias(&mut self, colon: Option<TableAlias>) -> Parsing<Option<TableAlias>> {
        match colon {
            Some(colon) => Ok(Some(colon)),
            None => self.table_alias(),
        }
    }

    /// `InnerTableRef`: `VALUES`, a table function, a subquery, a table, or `( TableRef )`.
    pub(crate) fn inner_table_ref(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let node: Expression = self.inner_table_ref_unspanned()?;
        Ok(self.spanned(first, node))
    }

    fn inner_table_ref_unspanned(&mut self) -> Parsing<Expression> {
        let colon: Option<TableAlias> = self.alias_colon()?;
        if self.kw("VALUES") {
            let rows: Vec<Vec<Expression>> = self.values_rows()?;
            let alias: Option<TableAlias> = self.trailing_alias(colon)?;
            let node: Expression = values(rows, alias);
            return self.sampled(node);
        }
        let is_lateral: bool = self.eat_kw("LATERAL");
        if self.is_char('(') && self.starts_query_at(1) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.query()?;
                parser.expect_char(')')?;
                Ok(query)
            });
            if let Ok(query) = attempt {
                let alias: Option<TableAlias> = self.trailing_alias(colon)?;
                let start: usize = self.offset();
                let node: Expression = self.sampled(subquery(query, alias, is_lateral))?;
                if is_lateral && matches!(node, Expression::TableSample(_)) {
                    self.refuse_later(LATERAL_SUBQUERY_SAMPLE, start);
                }
                return Ok(node);
            }
        }
        if let Some(call) = self.table_function()? {
            let with_ordinality: bool = self.eat_kws(&["WITH", "ORDINALITY"]);
            let alias: Option<TableAlias> = self.trailing_alias(colon)?;
            let mut node: Expression = match is_lateral || with_ordinality {
                true => lateral(call, alias),
                false => aliased(call, alias),
            };
            if let (Expression::Lateral(lateral), true) = (&mut node, with_ordinality) {
                lateral.ordinality = Some(Box::new(boolean(true)));
            }
            let start: usize = self.offset();
            let node: Expression = self.sampled(node)?;
            if is_lateral && matches!(node, Expression::TableSample(_)) {
                self.refuse_later(LATERAL_FUNCTION_SAMPLE, start);
            }
            return Ok(node);
        }
        if is_lateral {
            return Err(self.error());
        }
        if self.is_char('(') {
            self.at += 1;
            let (item, joins) = self.deeper(|parser| parser.table_ref())?;
            self.expect_char(')')?;
            let alias: Option<TableAlias> = self.trailing_alias(colon)?;
            let node: Expression = match (joins.is_empty(), alias) {
                (false, Some(alias)) if alias.columns.is_empty() => {
                    joined_table(item, joins, Some(alias.name))
                }
                (_, alias) => aliased(collapse(item, joins), alias),
            };
            return self.sampled(node);
        }
        let mut relation: TableRef = self.base_table_name()?;
        if let Some(alias) = self.trailing_alias(colon)? {
            relation.alias = Some(alias.name);
            relation.alias_explicit_as = alias.explicit;
            relation.column_aliases = alias.columns;
        }
        if self.kw("AT") && self.char_at(1, '(') {
            self.at += 2;
            let Some(kind) = self
                .word_at(0)
                .filter(|word| AT_UNITS.contains(&word.as_str()))
            else {
                return Err(self.error());
            };
            self.at += 1;
            if !self.eat_op("=>") {
                return Err(self.error());
            }
            let expression: Expression = self.expression()?;
            self.expect_char(')')?;
            relation.when = Some(Box::new(HistoricalData {
                this: Box::new(Expression::Identifier(Identifier::new(AT_WORD))),
                kind,
                expression: Box::new(expression),
            }));
        }
        if self.kw("TABLESAMPLE") {
            self.at += 1;
            relation.table_sample = Some(Box::new(self.sample_entry()?));
        }
        Ok(Expression::Table(Box::new(relation)))
    }

    /// `SampleClause?` after a reference other than a table, as a sample of that reference.
    fn sampled(&mut self, node: Expression) -> Parsing<Expression> {
        if !self.eat_kw("TABLESAMPLE") {
            return Ok(node);
        }
        let sample: Sample = self.sample_entry()?;
        Ok(Expression::TableSample(Box::new(TableSample {
            this: Some(Box::new(node)),
            sample: Some(Box::new(sample)),
            expressions: Vec::new(),
            method: None,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            percent: None,
            rows: None,
            size: None,
            seed: None,
        })))
    }

    /// `TableFunction`: `[catalog.][schema.]name ( args )`, when one starts here.
    fn table_function(&mut self) -> Parsing<Option<Expression>> {
        Ok(self
            .table_function_call()?
            .map(|(name, args)| Expression::Function(Box::new(function(name, args)))))
    }

    /// `QualifiedTableFunction TableFunctionArguments`: the function's name parts and its arguments.
    pub(crate) fn table_function_call(
        &mut self,
    ) -> Parsing<Option<(Vec<Identifier>, Vec<Expression>)>> {
        let mut offset: usize = 0;
        while self.is_at(offset, is_identifier) && self.char_at(offset + 1, '.') {
            offset += 2;
        }
        let named: bool = match offset {
            0 => self.is_at(0, is_function_name),
            _ => self.is_at(offset, is_col_label),
        };
        if !named || !self.char_at(offset + 1, '(') {
            return Ok(None);
        }
        let mut name: Vec<Identifier> = Vec::new();
        for _ in 0..=offset / 2 {
            name.push(identifier(&self.advance()?));
            self.eat_char('.');
        }
        let args: Vec<Expression> = self.table_function_arguments()?;
        Ok(Some((name, args)))
    }

    /// `TableFunctionArguments`: `( [argument, ...] )`, a trailing comma allowed.
    pub(crate) fn table_function_arguments(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        let mut args: Vec<Expression> = Vec::new();
        while !self.is_char(')') {
            args.push(self.table_function_argument()?);
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char(')')?;
        Ok(args)
    }

    /// `FunctionArgument` in a table function's list.
    fn table_function_argument(&mut self) -> Parsing<Expression> {
        if self.is(is_type_func_name) && (self.op_at(1, ":=") || self.op_at(1, "=>")) {
            let name: Identifier = self.name(is_type_func_name)?;
            let separator = match self.is_op(":=") {
                true => sqlquack_syntax::expressions::operators::models::NamedArgSeparator::ColonEq,
                false => sqlquack_syntax::expressions::operators::models::NamedArgSeparator::DArrow,
            };
            self.at += 1;
            let value: Expression = self.expression()?;
            return Ok(Expression::NamedArgument(Box::new(
                sqlquack_syntax::expressions::operators::models::NamedArgument {
                    name,
                    value,
                    separator,
                },
            )));
        }
        self.expression()
    }

    /// `BaseTableName`: `[catalog.]schema.table`, `schema.table` or `table`; a qualified table may be `'name'`, quotes kept.
    pub(crate) fn base_table_name(&mut self) -> Parsing<TableRef> {
        let first: Token = match self.peek() {
            Some(token) if is_table_name(token) => token.clone(),
            _ => return Err(self.error()),
        };
        self.at += 1;
        let mut parts: Vec<Identifier> = vec![identifier(&first)];
        let path: bool = matches!(first.kind, TokenKind::String(_));
        while !path && self.is_char('.') && self.is_at(1, is_col_label) {
            self.at += 1;
            parts.push(identifier(&self.advance()?));
        }
        if !path {
            parts.extend(self.quoted_table_part()?);
        }
        Ok(table(parts))
    }

    /// A qualified table's last part written `.'name'`, its quotes kept as `ReservedTableName` keeps them.
    pub(crate) fn quoted_table_part(&mut self) -> Parsing<Option<Identifier>> {
        if !(self.is_char('.') && self.single_quoted_at(1)) {
            return Ok(None);
        }
        self.at += 1;
        let token: Token = self.advance()?;
        let written: &str = &self.sql[token.start..token.end];
        Ok(Some(
            Identifier::quoted(written.to_owned()).with_span(token.span),
        ))
    }

    /// `SampleEntry`: `n [%|PERCENT|ROWS] [(method[, seed])]` or `[method] (n unit) [REPEATABLE (seed)]`.
    pub(crate) fn sample_entry(&mut self) -> Parsing<Sample> {
        let method_first: bool = self.is(is_identifier) && self.char_at(1, '(');
        let method_word: Option<String> = match method_first {
            true => Some(self.advance()?.value),
            false => None,
        };
        let parenthesized: bool = self.eat_char('(');
        let size_start: usize = self.offset();
        let size: Expression = self.sample_value()?;
        let (percent, rows) = self.sample_unit();
        self.check_sample_size(&size, percent, size_start);
        let mut method_name: Option<String> = method_word;
        let mut seed: Option<Expression> = None;
        if parenthesized {
            self.expect_char(')')?;
            if self.kw("REPEATABLE") && self.char_at(1, '(') {
                self.at += 2;
                seed = Some(self.sample_seed()?);
                self.expect_char(')')?;
            }
        } else if self.eat_char('(') {
            method_name = Some(self.name(is_identifier)?.name);
            if self.eat_char(',') {
                seed = Some(self.sample_seed()?);
            }
            self.expect_char(')')?;
        }
        let method: SampleMethod = match method_name
            .as_deref()
            .map(str::to_ascii_uppercase)
            .as_deref()
        {
            Some("SYSTEM") => SampleMethod::System,
            Some("BERNOULLI") => SampleMethod::Bernoulli,
            Some("RESERVOIR") => SampleMethod::Reservoir,
            _ if rows => SampleMethod::Row,
            _ => SampleMethod::Percent,
        };
        Ok(Sample {
            method,
            size,
            seed,
            offset: None,
            unit_after_size: percent || rows,
            use_sample_keyword: false,
            explicit_method: method_name.is_some(),
            method_before_size: method_first,
            use_seed_keyword: false,
            bucket_numerator: None,
            bucket_denominator: None,
            bucket_field: None,
            is_using_sample: false,
            is_percent: percent,
            suppress_method_output: false,
        })
    }

    /// `SampleValue`: a number or a parameter.
    fn sample_value(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number | TokenKind::Char('?' | '$')) => self.primary_expression(),
            _ => Err(self.error()),
        }
    }

    /// `SampleSeed`: a number.
    fn sample_seed(&mut self) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Number) => self.primary_expression(),
            _ => Err(self.error()),
        }
    }

    /// `TransformSampleCount`'s refusals: a parameter, a percentage outside 0 to 100, too many rows.
    fn check_sample_size(&mut self, size: &Expression, percent: bool, start: usize) {
        let Expression::Literal(literal) = size else {
            self.refuse_later(SAMPLE_CONSTANT, start);
            return;
        };
        let Literal::Number(text) = literal.as_ref() else {
            return;
        };
        let digits: String = text.replace('_', "");
        let value: f64 = digits.parse().unwrap_or(0.0);
        let message: Option<String> = match percent {
            true if !(0.0..=100.0).contains(&value) => Some(format!(
                "Sample sample_size {value:.6} out of range, must be between 0 and 100"
            )),
            true => None,
            false => sample_rows(&digits, value)
                .filter(|rows| *rows > MAX_SAMPLE_ROWS)
                .map(|rows| {
                    format!(
                        "Sample rows {rows} out of range, must be between 0 and {MAX_SAMPLE_ROWS}"
                    )
                }),
        };
        if let Some(message) = message {
            self.refuse_later(&message, start);
        }
    }

    /// `SampleUnit?`: (`%` or `PERCENT`, `ROWS`).
    fn sample_unit(&mut self) -> (bool, bool) {
        if self.eat_char('%') || self.eat_kw("PERCENT") {
            return (true, false);
        }
        (false, self.eat_kw("ROWS"))
    }

    /// An alias after a pivot: a `TableAlias`.
    fn pivot_alias(&mut self) -> Parsing<(Option<Identifier>, Vec<Identifier>)> {
        Ok(match self.table_alias()? {
            Some(alias) => (Some(alias.name), alias.columns),
            None => (None, Vec::new()),
        })
    }

    /// `TablePivotClause`: `PIVOT ( targets FOR header IN (values) ... [GROUP BY ...] ) [alias]`.
    fn table_pivot(&mut self, input: Expression) -> Parsing<Expression> {
        self.at += 1;
        self.expect_char('(')?;
        let aggregates: Vec<Expression> = self.comma_list(|parser| parser.named_expression())?;
        self.expect_kw("FOR")?;
        let mut fields: Vec<Expression> = Vec::new();
        loop {
            let header: Expression = self.primary_expression()?;
            self.expect_kw("IN")?;
            fields.push(self.pivot_values(header)?);
            if self.kw("GROUP") || self.is_char(')') {
                break;
            }
        }
        let group: Option<Box<Expression>> = self.pivot_group_by()?;
        self.expect_char(')')?;
        let (name, columns) = self.pivot_alias()?;
        Ok(Expression::Pivot(Box::new(Pivot {
            this: input,
            expressions: aggregates,
            fields,
            using: Vec::new(),
            group,
            unpivot: false,
            into: None,
            alias: name,
            alias_columns: columns,
            include_nulls: None,
            default_on_null: None,
            with: None,
        })))
    }

    /// `TableUnpivotClause`: `UNPIVOT [INCLUDE|EXCLUDE NULLS] ( value FOR name IN (cols) ... )`.
    fn table_unpivot(&mut self, input: Expression) -> Parsing<Expression> {
        self.at += 1;
        let include_nulls: Option<bool> = if self.eat_kws(&["INCLUDE", "NULLS"]) {
            Some(true)
        } else if self.eat_kws(&["EXCLUDE", "NULLS"]) {
            Some(false)
        } else {
            None
        };
        self.expect_char('(')?;
        let parenthesized: bool = self.is_char('(');
        let mut value_columns: Vec<Identifier> = match parenthesized {
            true => self.parenthesized(|parser| parser.col_id_or_string())?,
            false => vec![self.col_id_or_string()?],
        };
        self.expect_kw("FOR")?;
        let name_column: Identifier = self.col_id_or_string()?;
        self.expect_kw("IN")?;
        let columns: Vec<Expression> = self.parenthesized(|parser| parser.named_expression())?;
        self.expect_char(')')?;
        let (name, alias_columns) = self.pivot_alias()?;
        let value_column: Identifier = value_columns.remove(0);
        Ok(Expression::Unpivot(Box::new(Unpivot {
            this: input,
            value_column,
            name_column,
            columns,
            alias: name,
            alias_columns,
            value_column_parenthesized: parenthesized,
            include_nulls,
            extra_value_columns: value_columns,
        })))
    }
}

/// A row count as `int64`: an integer exactly, a decimal rounded half to even; `None` past `int64`.
fn sample_rows(digits: &str, value: f64) -> Option<i128> {
    if let Ok(rows) = digits.parse::<i128>() {
        return (rows <= i128::from(i64::MAX)).then_some(rows);
    }
    let rounded: f64 = value.round_ties_even();
    (rounded < I64_LIMIT).then_some(rounded as i128)
}
