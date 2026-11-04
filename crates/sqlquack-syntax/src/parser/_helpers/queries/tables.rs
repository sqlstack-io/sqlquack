//! Parser queries: tables.

use crate::expressions::columns::models::ColumnDef;
use crate::expressions::functions::models::AtIndex;
use crate::expressions::functions::models::PivotAny;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::In;
use crate::expressions::operators::models::Placeholder;
use crate::expressions::queries::models::Pivot;
use crate::expressions::queries::models::PivotAlias;
use crate::expressions::queries::models::Resample;
use crate::expressions::queries::models::Unpivot;
use crate::expressions::statements::models::TableAlias;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::models::DateTimeField;
use crate::expressions::types::models::Tuple;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    #[inline(never)]
    pub(crate) fn parse_table_expression_tail(
        &mut self,
        mut expr: Expression,
        has_only: bool,
    ) -> Result<Expression> {
        self.match_token(TokenType::Star);

        if self.check_keyword_text("CHANGES") {
            if let Some(changes_expr) = self.parse_changes()? {
                if let Expression::Table(ref mut table) = expr {
                    if let Expression::Changes(changes_box) = changes_expr {
                        table.changes = Some(changes_box);
                    }
                }
            }
        }

        if self.check(TokenType::Before) || self.check_keyword_text("AT") {
            if let Some(historical_expr) = self.parse_historical_data()? {
                if let Expression::Table(ref mut table) = expr {
                    if let Expression::HistoricalData(hd) = historical_expr {
                        table.when = Some(hd);
                    }
                }
            }
        }

        expr = self.parse_table_for_clause(expr)?;

        expr = self.parse_table_for_system_time(expr)?;

        expr = self.parse_table_version_clause(expr)?;

        expr = self.parse_table_sample(expr)?;

        if self.check(TokenType::With) && self.check_next(TokenType::LParen) {
            if let Expression::Table(ref mut table) = expr {
                if let Some(hint_expr) = self.parse_table_hints()? {
                    match hint_expr {
                        Expression::Tuple(tuple) => {
                            table.hints = tuple.expressions;
                        }
                        other => {
                            table.hints = vec![other];
                        }
                    }
                }
            }
        }

        expr = self.parse_table_index_hints(expr)?;

        expr = self.parse_table_indexed_by(expr)?;

        if self.check(TokenType::Pivot) && self.check_next(TokenType::LParen) {
            self.skip();
            expr = self.parse_pivot(expr)?;
        }
        if self.check(TokenType::Unpivot) && self.is_unpivot_clause_start() {
            self.skip();
            expr = self.parse_unpivot(expr)?;
        } else if self.check(TokenType::MatchRecognize)
            && !matches!(&expr, Expression::Pivot(_) | Expression::Unpivot(_))
        {
            self.skip();
            expr = self.parse_match_recognize(Some(expr))?;
        }

        if self.is_resample_start() {
            expr = self.parse_table_resample(expr)?;
        } else {
            expr = self.parse_table_alias(expr)?;
            if self.is_resample_start() {
                expr = self.parse_table_resample(expr)?;
            }
        }

        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::DuckDB)
        ) && (self.check(TokenType::Before) || self.check_identifier("AT"))
        {
            if let Some(Expression::HistoricalData(historical)) = self.parse_historical_data()? {
                if let Expression::Table(ref mut table) = expr {
                    table.when = Some(historical);
                }
            }
        }

        expr = self.parse_table_late_indexed_by(expr)?;

        expr = self.parse_table_late_sample(expr)?;

        expr = self.parse_table_late_index_hints(expr)?;

        if self.check(TokenType::Pivot) && self.check_next(TokenType::LParen) {
            self.skip();
            expr = self.parse_pivot(expr)?;
        } else if self.check(TokenType::Unpivot) && self.is_unpivot_clause_start() {
            self.skip();
            expr = self.parse_unpivot(expr)?;
        }
        expr = self.parse_table_pivot_alias(expr)?;

        expr = self.parse_table_at_clause(expr)?;

        expr = self.parse_table_trailing_sample(expr)?;

        if has_only {
            if let Expression::Table(ref mut table) = expr {
                table.only = true;
            }
        }

        expr = self.parse_table_trailing_time_travel(expr);

        Ok(expr)
    }

    /// Whether a Snowflake `RESAMPLE (` clause comes next.
    fn is_resample_start(&self) -> bool {
        matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && self.check_keyword_text("RESAMPLE")
            && self.check_next(TokenType::LParen)
    }

    /// Parses `RESAMPLE ( USING col INCREMENT BY expr [PARTITION BY ...] [METADATA_COLUMNS f() [[AS] a], ...] )`.
    fn parse_table_resample(&mut self, expr: Expression) -> Result<Expression> {
        self.skip();
        self.expect_token(TokenType::LParen)?;
        if !self.match_token(TokenType::Using) {
            return Err(self.parse_error("Expected USING in RESAMPLE"));
        }
        let using = self.expect_identifier_with_quoted()?;
        if !self.match_text_seq(&["INCREMENT", "BY"]) {
            return Err(self.parse_error("Expected INCREMENT BY in RESAMPLE"));
        }
        let increment = self.parse_expression()?;
        let partition_by = if self.match_keywords(&[TokenType::Partition, TokenType::By]) {
            self.parse_expression_list()?
        } else {
            Vec::new()
        };
        let mut metadata_columns = Vec::new();
        if self.match_keyword("METADATA_COLUMNS") {
            loop {
                let is_call = matches!(
                    self.peek().token_type,
                    TokenType::Identifier | TokenType::Var | TokenType::QuotedIdentifier
                ) && self.check_next(TokenType::LParen);
                if !is_call {
                    return Err(self.parse_error("Expected a metadata column function in RESAMPLE"));
                }
                let call = self.parse_expression()?;
                let alias = if self.match_token(TokenType::As)
                    || matches!(
                        self.peek().token_type,
                        TokenType::Identifier | TokenType::Var | TokenType::QuotedIdentifier
                    ) {
                    Some(self.expect_identifier_with_quoted()?)
                } else {
                    None
                };
                metadata_columns.push((call, alias));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Resample(Box::new(Resample {
            this: Box::new(expr),
            using,
            increment,
            partition_by,
            metadata_columns,
        })))
    }

    fn parse_table_sample(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check(TokenType::TableSample) || self.check(TokenType::Sample) {
            if let Some(sample) = self.parse_table_level_sample()? {
                if let Expression::Table(ref mut table) = expr {
                    table.table_sample = Some(Box::new(sample));
                } else {
                    expr = Expression::TableSample(Box::new(
                        crate::expressions::statements::models::TableSample {
                            this: Some(Box::new(expr)),
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
                        },
                    ));
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_indexed_by(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check_identifier("INDEXED") {
            self.skip();
            self.expect_token(TokenType::By)?;
            let first_part = self.expect_identifier_or_keyword()?;
            let index_name = if self.match_token(TokenType::Dot) {
                let second_part = self.expect_identifier_or_keyword()?;
                format!("{}.{}", first_part, second_part)
            } else {
                first_part
            };
            if let Expression::Table(ref mut table) = expr {
                table.hints.push(Expression::Identifier(Identifier {
                    name: format!("INDEXED BY {}", index_name),
                    quoted: false,
                    trailing_comments: Vec::new(),
                    span: None,
                }));
            }
        } else if self.check(TokenType::Not) && self.check_next_identifier("INDEXED") {
            self.skip();
            self.skip();
            if let Expression::Table(ref mut table) = expr {
                table.hints.push(Expression::Identifier(Identifier {
                    name: "NOT INDEXED".to_string(),
                    quoted: false,
                    trailing_comments: Vec::new(),
                    span: None,
                }));
            }
        }
        Ok(expr)
    }

    fn parse_table_late_indexed_by(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check_identifier("INDEXED") {
            self.skip();
            self.expect_token(TokenType::By)?;
            let first_part = self.expect_identifier_or_keyword()?;
            let index_name = if self.match_token(TokenType::Dot) {
                let second_part = self.expect_identifier_or_keyword()?;
                format!("{}.{}", first_part, second_part)
            } else {
                first_part
            };
            if let Expression::Table(ref mut table) = expr {
                table.hints.push(Expression::Identifier(Identifier {
                    name: format!("INDEXED BY {}", index_name),
                    quoted: false,
                    trailing_comments: Vec::new(),
                    span: None,
                }));
            }
        }
        Ok(expr)
    }

    fn parse_table_late_sample(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check(TokenType::With) && self.check_next(TokenType::LParen) {
            if let Expression::Table(ref mut table) = expr {
                if let Some(hint_expr) = self.parse_table_hints()? {
                    match hint_expr {
                        Expression::Tuple(tuple) => {
                            table.hints = tuple.expressions;
                        }
                        other => {
                            table.hints = vec![other];
                        }
                    }
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_late_index_hints(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check_keyword_text("USE")
            || self.check(TokenType::Ignore)
            || self.check_keyword_text("FORCE")
        {
            let next_idx = self.current + 1;
            let is_index_hint = next_idx < self.tokens.len() && {
                let next_text = &self.tokens[next_idx].text;
                next_text.eq_ignore_ascii_case("INDEX") || next_text.eq_ignore_ascii_case("KEY")
            };
            if is_index_hint {
                if let Expression::Table(ref mut table) = expr {
                    if let Some(hint_expr) = self.parse_table_hints()? {
                        match hint_expr {
                            Expression::Tuple(tuple) => {
                                table.hints = tuple.expressions;
                            }
                            other => {
                                table.hints = vec![other];
                            }
                        }
                    }
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_trailing_sample(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check(TokenType::TableSample) || self.check(TokenType::Sample) {
            if let Some(sample) = self.parse_table_level_sample()? {
                let post_sample_comments = self.previous_trailing_comments().to_vec();
                if let Expression::Table(ref mut table) = expr {
                    table.table_sample = Some(Box::new(sample));
                    if !post_sample_comments.is_empty() {
                        table.trailing_comments.extend(post_sample_comments);
                    }
                } else {
                    expr = Expression::TableSample(Box::new(
                        crate::expressions::statements::models::TableSample {
                            this: Some(Box::new(expr)),
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
                        },
                    ));
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_for_clause(&mut self, mut expr: Expression) -> Result<Expression> {
        if (self.check(TokenType::For))
            && (self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1]
                    .text
                    .eq_ignore_ascii_case("SYSTEM_TIME")
                || self.current + 2 < self.tokens.len()
                    && self.tokens[self.current + 1]
                        .text
                        .eq_ignore_ascii_case("SYSTEM")
                    && self.tokens[self.current + 2]
                        .text
                        .eq_ignore_ascii_case("TIME"))
        {
            self.skip();
            if self.check_keyword_text("SYSTEM_TIME") {
                self.skip();
            } else {
                self.skip();
                self.skip();
            }
            let system_time_str = if self.match_token(TokenType::As) {
                if self.check_keyword_text("OF") {
                    self.skip();
                    let start = self.current;
                    while !self.is_at_end()
                        && !self.check(TokenType::Semicolon)
                        && !self.check(TokenType::Where)
                        && !self.check(TokenType::Join)
                        && !self.check(TokenType::Left)
                        && !self.check(TokenType::Right)
                        && !self.check(TokenType::Inner)
                        && !self.check(TokenType::Outer)
                        && !self.check(TokenType::Full)
                        && !self.check(TokenType::Cross)
                        && !self.check(TokenType::Order)
                        && !self.check(TokenType::Group)
                        && !self.check(TokenType::Having)
                        && !self.check(TokenType::Limit)
                        && !self.check(TokenType::Union)
                        && !self.check(TokenType::Except)
                        && !self.check(TokenType::Intersect)
                        && !self.check(TokenType::As)
                        && !self.check(TokenType::Comma)
                        && !self.check(TokenType::RParen)
                        && !self.check(TokenType::With)
                        && !self.check(TokenType::Pivot)
                        && !self.check(TokenType::Unpivot)
                    {
                        self.skip();
                    }
                    let expr_text = self.tokens_to_sql_uppercased(start, self.current);
                    format!("FOR SYSTEM_TIME AS OF {}", expr_text)
                } else {
                    "FOR SYSTEM_TIME AS".to_string()
                }
            } else if self.match_token(TokenType::Between) {
                let start = self.current;
                while !self.is_at_end() && !self.check(TokenType::And) {
                    self.skip();
                }
                let expr1_text = self.tokens_to_sql_uppercased(start, self.current);
                self.skip();
                let start2 = self.current;
                while !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Where)
                    && !self.check(TokenType::Join)
                    && !self.check(TokenType::Left)
                    && !self.check(TokenType::Right)
                    && !self.check(TokenType::Inner)
                    && !self.check(TokenType::Outer)
                    && !self.check(TokenType::Full)
                    && !self.check(TokenType::Cross)
                    && !self.check(TokenType::Order)
                    && !self.check(TokenType::Group)
                    && !self.check(TokenType::Having)
                    && !self.check(TokenType::Limit)
                    && !self.check(TokenType::Union)
                    && !self.check(TokenType::Except)
                    && !self.check(TokenType::Intersect)
                    && !self.check(TokenType::As)
                    && !self.check(TokenType::Comma)
                    && !self.check(TokenType::RParen)
                    && !self.check(TokenType::With)
                    && !self.check(TokenType::Pivot)
                    && !self.check(TokenType::Unpivot)
                {
                    self.skip();
                }
                let expr2_text = self.tokens_to_sql_uppercased(start2, self.current);
                format!("FOR SYSTEM_TIME BETWEEN {} AND {}", expr1_text, expr2_text)
            } else if self.match_token(TokenType::From) {
                let start = self.current;
                while !self.is_at_end() && !self.check(TokenType::To) {
                    self.skip();
                }
                let expr1_text = self.tokens_to_sql_uppercased(start, self.current);
                self.skip();
                let start2 = self.current;
                while !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Where)
                    && !self.check(TokenType::As)
                    && !self.check(TokenType::Comma)
                    && !self.check(TokenType::RParen)
                {
                    self.skip();
                }
                let expr2_text = self.tokens_to_sql_uppercased(start2, self.current);
                format!("FOR SYSTEM_TIME FROM {} TO {}", expr1_text, expr2_text)
            } else if self.check_identifier("CONTAINED") {
                self.skip();
                self.expect_token(TokenType::In)?;
                self.expect_token(TokenType::LParen)?;
                let start = self.current;
                let mut depth = 1;
                while !self.is_at_end() && depth > 0 {
                    if self.check(TokenType::LParen) {
                        depth += 1;
                    }
                    if self.check(TokenType::RParen) {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    self.skip();
                }
                let inner_text = self.tokens_to_sql_uppercased(start, self.current);
                self.expect_token(TokenType::RParen)?;
                format!("FOR SYSTEM_TIME CONTAINED IN ({})", inner_text)
            } else if self.match_token(TokenType::All) {
                "FOR SYSTEM_TIME ALL".to_string()
            } else {
                "FOR SYSTEM_TIME".to_string()
            };
            if let Expression::Table(ref mut table) = expr {
                table.system_time = Some(system_time_str);
            }
        }
        Ok(expr)
    }

    fn parse_table_for_system_time(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check(TokenType::For) && self.current + 1 < self.tokens.len() {
            let next_text = &self.tokens[self.current + 1].text;
            if next_text.eq_ignore_ascii_case("VERSION")
                || next_text.eq_ignore_ascii_case("TIMESTAMP")
            {
                self.skip();
                let version_kind = self.advance_text()?.to_ascii_uppercase();

                if self.match_token(TokenType::As) && self.check_keyword_text("OF") {
                    self.skip();

                    if let Some(value_expr) = self.parse_bitwise()? {
                        let version = crate::expressions::statements::models::Version {
                            this: Box::new(Expression::Identifier(Identifier::new(&version_kind))),
                            kind: "AS OF".to_string(),
                            expression: Some(Box::new(value_expr)),
                        };
                        if let Expression::Table(ref mut table) = expr {
                            table.version = Some(Box::new(version));
                        }
                    }
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_version_clause(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.current < self.tokens.len() {
            let current_text = &self.tokens[self.current].text;
            if (current_text.eq_ignore_ascii_case("TIMESTAMP")
                || current_text.eq_ignore_ascii_case("VERSION"))
                && self.current + 2 < self.tokens.len()
                && self.tokens[self.current + 1].token_type == TokenType::As
                && self.tokens[self.current + 2]
                    .text
                    .eq_ignore_ascii_case("OF")
            {
                let version_kind = self.advance_text()?.to_ascii_uppercase();
                self.skip();
                self.skip();

                if let Some(value_expr) = self.parse_bitwise()? {
                    let version = crate::expressions::statements::models::Version {
                        this: Box::new(Expression::Identifier(Identifier::new(&version_kind))),
                        kind: "AS OF".to_string(),
                        expression: Some(Box::new(value_expr)),
                    };
                    if let Expression::Table(ref mut table) = expr {
                        table.version = Some(Box::new(version));
                    }
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_index_hints(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check_keyword_text("USE")
            || self.check(TokenType::Ignore)
            || self.check_keyword_text("FORCE")
        {
            let next_idx = self.current + 1;
            let is_index_hint = next_idx < self.tokens.len() && {
                let next_text = &self.tokens[next_idx].text;
                next_text.eq_ignore_ascii_case("INDEX") || next_text.eq_ignore_ascii_case("KEY")
            };
            if is_index_hint {
                if let Expression::Table(ref mut table) = expr {
                    if let Some(hint_expr) = self.parse_table_hints()? {
                        match hint_expr {
                            Expression::Tuple(tuple) => {
                                table.hints = tuple.expressions;
                            }
                            other => {
                                table.hints = vec![other];
                            }
                        }
                    }
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_alias(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.match_token(TokenType::As) {
            let alias_keyword = if self.source.is_some() {
                let token = self.previous();
                Some(self.source_text_range(token.span.start, token.span.end))
            } else {
                Some(self.previous_text().to_string())
            };
            if self.check(TokenType::LParen) {
                self.skip();
                let mut column_aliases: Vec<Identifier> = Vec::new();
                loop {
                    if self.check(TokenType::RParen) {
                        break;
                    }
                    column_aliases
                        .push(self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                expr = Expression::Alias(Box::new(Alias {
                    this: expr,
                    alias: Identifier::new(String::new()),
                    column_aliases,
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));
            } else {
                expr = self.parse_table_explicit_alias_name(alias_keyword, expr)?;
            }
        } else if self.can_parse_implicit_table_alias() {
            expr = self.parse_table_implicit_alias(expr)?;
        }
        Ok(expr)
    }

    fn parse_table_explicit_alias_name(
        &mut self,
        alias_keyword: Option<String>,
        mut expr: Expression,
    ) -> Result<Expression> {
        let alias_ident_parsed = self.expect_identifier_or_alias_keyword_with_quoted()?;
        let alias_span = alias_ident_parsed.span;
        let alias = alias_ident_parsed.name;
        let alias_is_quoted = alias_ident_parsed.quoted;
        let mut alias_identifier = if alias_is_quoted {
            Identifier::quoted(alias)
        } else {
            Identifier::new(alias)
        };
        alias_identifier.span = alias_span;
        if self.match_token(TokenType::LParen) {
            let has_typed_columns = self.check_typed_column_list();

            if has_typed_columns {
                expr = self.parse_table_typed_column_aliases(alias_identifier, expr)?;
            } else {
                let mut aliases: Vec<Identifier> = Vec::new();
                loop {
                    if self.check(TokenType::RParen) {
                        break;
                    }
                    aliases.push(self.expect_identifier_or_keyword_with_quoted()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;

                expr = match expr {
                    Expression::Table(mut t) => {
                        t.alias = Some(alias_identifier);
                        t.alias_explicit_as = true;
                        t.column_aliases = aliases;
                        Expression::Table(t)
                    }
                    Expression::Subquery(mut s) => {
                        s.alias = Some(alias_identifier);
                        s.column_aliases = aliases;
                        s.alias_explicit_as = true;
                        s.alias_keyword = alias_keyword.clone();
                        Expression::Subquery(s)
                    }
                    Expression::Pivot(mut p) => {
                        p.alias = Some(alias_identifier);
                        p.alias_columns = aliases;
                        Expression::Pivot(p)
                    }
                    Expression::Unpivot(mut u) => {
                        u.alias = Some(alias_identifier);
                        u.alias_columns = aliases;
                        Expression::Unpivot(u)
                    }
                    Expression::MatchRecognize(mut mr) => {
                        mr.alias = Some(alias_identifier);
                        mr.alias_explicit_as = true;
                        Expression::MatchRecognize(mr)
                    }
                    Expression::JoinedTable(jt) => Expression::Alias(Box::new(Alias {
                        this: Expression::JoinedTable(jt),
                        alias: alias_identifier,
                        column_aliases: aliases,
                        alias_explicit_as: true,
                        alias_keyword: alias_keyword.clone(),
                        pre_alias_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })),
                    _ => Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias: alias_identifier,
                        column_aliases: aliases,
                        alias_explicit_as: true,
                        alias_keyword: alias_keyword.clone(),
                        pre_alias_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })),
                };
            }
        } else {
            expr = self.apply_table_alias(alias_identifier, alias_keyword, expr);
        }
        Ok(expr)
    }

    fn apply_table_alias(
        &self,
        alias_identifier: Identifier,
        alias_keyword: Option<String>,
        mut expr: Expression,
    ) -> Expression {
        let default_column_aliases = { Vec::new() };
        expr = match expr {
            Expression::Table(mut t) => {
                t.alias = Some(alias_identifier);
                t.alias_explicit_as = true;
                t.column_aliases = Vec::new();
                Expression::Table(t)
            }
            Expression::Subquery(mut s) => {
                s.alias = Some(alias_identifier);
                s.column_aliases = Vec::new();
                s.alias_explicit_as = true;
                s.alias_keyword = alias_keyword.clone();
                Expression::Subquery(s)
            }
            Expression::Pivot(mut p) => {
                p.alias = Some(alias_identifier);
                p.alias_columns = default_column_aliases;
                Expression::Pivot(p)
            }
            Expression::Unpivot(mut u) => {
                u.alias = Some(alias_identifier);
                u.alias_columns = default_column_aliases;
                Expression::Unpivot(u)
            }
            Expression::MatchRecognize(mut mr) => {
                mr.alias = Some(alias_identifier);
                mr.alias_explicit_as = true;
                Expression::MatchRecognize(mr)
            }
            Expression::JoinedTable(mut jt) => {
                jt.alias = Some(alias_identifier);
                Expression::JoinedTable(jt)
            }
            _ => Expression::Alias(Box::new(Alias {
                this: expr,
                alias: alias_identifier,
                column_aliases: default_column_aliases,
                alias_explicit_as: true,
                alias_keyword: alias_keyword.clone(),
                pre_alias_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            })),
        };

        expr
    }

    fn parse_table_typed_column_aliases(
        &mut self,
        alias_identifier: Identifier,
        mut expr: Expression,
    ) -> Result<Expression> {
        let mut typed_cols: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::RParen) {
                break;
            }
            let col_name = self.expect_identifier_or_keyword_with_quoted()?;
            let col_type = self.parse_data_type()?;
            let mut col_def = ColumnDef::new(col_name.name.clone(), col_type);
            col_def.name = col_name;
            typed_cols.push(Expression::ColumnDef(Box::new(col_def)));

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;

        let table_alias = Expression::TableAlias(Box::new(TableAlias {
            this: Some(Box::new(Expression::Identifier(alias_identifier))),
            columns: typed_cols,
        }));

        expr = Expression::Tuple(Box::new(Tuple {
            expressions: vec![expr, table_alias],
        }));
        Ok(expr)
    }

    fn parse_table_implicit_alias(&mut self, mut expr: Expression) -> Result<Expression> {
        let is_keyword_alias = self.peek().token_type.is_keyword();
        let is_quoted_alias = self.peek().token_type == TokenType::QuotedIdentifier;
        let alias_span = self.peek().span;
        let alias = self.advance_text()?;
        let column_aliases = if self.match_token(TokenType::LParen) {
            let mut aliases: Vec<Identifier> = Vec::new();
            loop {
                aliases.push(self.expect_identifier_or_keyword_with_quoted()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            aliases
        } else {
            Vec::new()
        };

        let make_alias_ident = |name: String| -> Identifier {
            if is_quoted_alias {
                Identifier::quoted(name).with_span(alias_span)
            } else {
                Identifier::new(name).with_span(alias_span)
            }
        };
        expr = match expr {
            Expression::Table(mut t) => {
                t.alias = Some(make_alias_ident(alias));
                t.alias_explicit_as = is_keyword_alias;
                t.column_aliases = column_aliases;
                Expression::Table(t)
            }
            Expression::Subquery(mut s) => {
                s.alias = Some(make_alias_ident(alias));
                s.column_aliases = column_aliases;
                Expression::Subquery(s)
            }
            Expression::Pivot(mut p) => {
                p.alias = Some(make_alias_ident(alias));
                p.alias_columns = column_aliases;
                Expression::Pivot(p)
            }
            Expression::Unpivot(mut u) => {
                u.alias = Some(make_alias_ident(alias));
                u.alias_columns = column_aliases;
                Expression::Unpivot(u)
            }
            Expression::MatchRecognize(mut mr) => {
                mr.alias = Some(make_alias_ident(alias));
                Expression::MatchRecognize(mr)
            }
            Expression::JoinedTable(jt) if !column_aliases.is_empty() => {
                Expression::Alias(Box::new(Alias {
                    this: Expression::JoinedTable(jt),
                    alias: make_alias_ident(alias),
                    column_aliases,
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            }
            Expression::JoinedTable(mut jt) => {
                jt.alias = Some(make_alias_ident(alias));
                Expression::JoinedTable(jt)
            }
            _ => Expression::Alias(Box::new(Alias {
                this: expr,
                alias: make_alias_ident(alias),
                column_aliases,
                alias_explicit_as: false,
                alias_keyword: None,
                pre_alias_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            })),
        };
        Ok(expr)
    }

    fn parse_table_pivot_alias(&mut self, mut expr: Expression) -> Result<Expression> {
        if matches!(&expr, Expression::Pivot(_) | Expression::Unpivot(_)) {
            if self.match_token(TokenType::As) {
                let alias = self.expect_identifier_or_alias_keyword_with_quoted()?;
                let alias_columns = self.parse_alias_column_list_if_present()?;
                match &mut expr {
                    Expression::Pivot(p) => {
                        p.alias = Some(alias);
                        p.alias_columns = alias_columns;
                    }
                    Expression::Unpivot(u) => {
                        u.alias = Some(alias);
                        u.alias_columns = alias_columns;
                    }
                    _ => {}
                }
            } else if !self.check_keyword()
                && (self.check(TokenType::Var) || self.check(TokenType::QuotedIdentifier))
            {
                let tok = self.advance()?;
                let alias = if tok.token_type == TokenType::QuotedIdentifier {
                    Identifier::quoted(tok.text.to_string()).with_span(tok.span)
                } else {
                    Identifier::new(tok.text.clone()).with_span(tok.span)
                };
                let alias_columns = self.parse_alias_column_list_if_present()?;
                match &mut expr {
                    Expression::Pivot(p) => {
                        p.alias = Some(alias);
                        p.alias_columns = alias_columns;
                    }
                    Expression::Unpivot(u) => {
                        u.alias = Some(alias);
                        u.alias_columns = alias_columns;
                    }
                    _ => {}
                }
            }
        }
        Ok(expr)
    }

    fn parse_table_at_clause(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.match_identifier("AT") {
            let index_alias = self.expect_identifier_or_keyword()?;
            let column_expr = match expr {
                Expression::Table(t) => {
                    let mut parts: Vec<String> = Vec::new();
                    if let Some(cat) = t.catalog {
                        parts.push(cat.name);
                    }
                    if let Some(schema) = t.schema {
                        parts.push(schema.name);
                    }
                    parts.push(t.name.name);
                    let col_name = parts.join(".");

                    if let Some(alias) = t.alias {
                        Expression::Alias(Box::new(Alias {
                            span: SourceSpan::default(),
                            this: Self::parsed_column(Column {
                                name: Identifier::new(&col_name),
                                table: None,
                                join_mark: false,
                                trailing_comments: Vec::new(),
                                span: None,
                                inferred_type: None,
                            }),
                            alias,
                            column_aliases: t.column_aliases,
                            alias_explicit_as: false,
                            alias_keyword: None,
                            pre_alias_comments: Vec::new(),
                            trailing_comments: t.trailing_comments,
                            inferred_type: None,
                        }))
                    } else {
                        Self::parsed_column(Column {
                            name: Identifier::new(&col_name),
                            table: None,
                            join_mark: false,
                            trailing_comments: t.trailing_comments,
                            span: None,
                            inferred_type: None,
                        })
                    }
                }
                other => other,
            };
            expr = Expression::AtIndex(Box::new(AtIndex {
                this: Box::new(column_expr),
                expression: Box::new(Expression::Identifier(Identifier::new(index_alias))),
            }));
        }
        Ok(expr)
    }

    fn parse_table_trailing_time_travel(&mut self, mut expr: Expression) -> Expression {
        if self.check(TokenType::For)
            && (self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1]
                    .text
                    .eq_ignore_ascii_case("SYSTEM_TIME")
                || self.current + 2 < self.tokens.len()
                    && self.tokens[self.current + 1]
                        .text
                        .eq_ignore_ascii_case("SYSTEM")
                    && self.tokens[self.current + 2]
                        .text
                        .eq_ignore_ascii_case("TIME"))
        {
            self.skip();
            if self.check_keyword_text("SYSTEM_TIME") {
                self.skip();
            } else {
                self.skip();
                self.skip();
            }
            if self.match_token(TokenType::As) && self.check_keyword_text("OF") {
                self.skip();
                let start = self.current;
                while !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Where)
                    && !self.check(TokenType::Join)
                    && !self.check(TokenType::Left)
                    && !self.check(TokenType::Right)
                    && !self.check(TokenType::Inner)
                    && !self.check(TokenType::Outer)
                    && !self.check(TokenType::Full)
                    && !self.check(TokenType::Cross)
                    && !self.check(TokenType::Order)
                    && !self.check(TokenType::Group)
                    && !self.check(TokenType::Having)
                    && !self.check(TokenType::Limit)
                    && !self.check(TokenType::Union)
                    && !self.check(TokenType::Except)
                    && !self.check(TokenType::Intersect)
                    && !self.check(TokenType::Comma)
                    && !self.check(TokenType::RParen)
                {
                    self.skip();
                }
                let expr_text = self.tokens_to_sql(start, self.current);
                let system_time_str = format!("FOR SYSTEM_TIME AS OF {}", expr_text);
                if let Expression::Table(ref mut table) = expr {
                    table.system_time = Some(system_time_str);
                }
            }
        }
        expr
    }

    /// Parses standard PIVOT(agg [AS alias], ... FOR col IN (value [AS alias], ...)).
    pub(crate) fn parse_pivot(&mut self, source: Expression) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;

        let mut expressions: Vec<Expression> = Vec::new();
        loop {
            if self.check(TokenType::For) || self.check(TokenType::RParen) {
                break;
            }
            let func = self.parse_primary()?;
            let expr = if self.match_token(TokenType::As) {
                let alias_name = self.expect_identifier_or_keyword()?;
                Expression::Alias(Box::new(Alias::new(func, Identifier::new(alias_name))))
            } else if !self.check(TokenType::Comma)
                && !self.check(TokenType::For)
                && !self.check(TokenType::RParen)
            {
                if let Some(id) = self.parse_id_var()? {
                    let alias_name = match &id {
                        Expression::Identifier(ident) => ident.name.clone(),
                        Expression::Column(col) => col.name.name.clone(),
                        _ => String::new(),
                    };
                    if !alias_name.is_empty() {
                        Expression::Alias(Box::new(Alias::new(func, Identifier::new(alias_name))))
                    } else {
                        func
                    }
                } else {
                    func
                }
            } else {
                func
            };
            expressions.push(expr);
            if !self.match_token(TokenType::Comma) {
                break;
            }
            if self.check(TokenType::For) {
                break;
            }
        }

        self.expect_token(TokenType::For)?;

        let mut fields: Vec<Expression> = Vec::new();
        loop {
            let field = self.parse_standard_pivot_in()?;
            fields.push(field);

            if !self.match_token(TokenType::For) {
                break;
            }
        }

        let default_on_null = if self.match_text_seq(&["DEFAULT", "ON", "NULL"]) {
            if self.match_token(TokenType::LParen) {
                let val = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                Some(Box::new(val))
            } else {
                None
            }
        } else {
            None
        };

        let group = self.parse_group()?;

        self.expect_token(TokenType::RParen)?;

        Ok(Expression::Pivot(Box::new(Pivot {
            this: source,
            expressions,
            fields,
            using: Vec::new(),
            group: group.map(Box::new),
            unpivot: false,
            into: None,
            alias: None,
            alias_columns: Vec::new(),
            include_nulls: None,
            default_on_null,
            with: None,
        })))
    }

    /// Parse FOR column IN (...) part of standard PIVOT
    pub(crate) fn parse_standard_pivot_in(&mut self) -> Result<Expression> {
        let column = self.parse_primary()?;

        self.expect_token(TokenType::In)?;

        if self.match_token(TokenType::LParen) {
            let in_exprs = if self.match_text_seq(&["ANY"]) {
                let order = self.parse_order()?;
                vec![Expression::PivotAny(Box::new(PivotAny {
                    this: order.map(Box::new),
                }))]
            } else {
                let mut vals: Vec<Expression> = Vec::new();
                loop {
                    if self.check(TokenType::RParen) {
                        break;
                    }
                    if let Some(val) = self.parse_select_or_expression()? {
                        let val = if self.match_token(TokenType::As) {
                            let alias_expr = self.parse_bitwise()?.ok_or_else(|| {
                                self.parse_error(
                                    "Expected expression after AS in PIVOT/UNPIVOT IN clause",
                                )
                            })?;
                            Expression::PivotAlias(Box::new(PivotAlias {
                                this: val,
                                alias: alias_expr,
                            }))
                        } else {
                            val
                        };
                        vals.push(val);
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                vals
            };
            self.expect_token(TokenType::RParen)?;
            Ok(Expression::In(Box::new(In {
                this: column,
                expressions: in_exprs,
                query: None,
                not: false,
                global: false,
                unnest: None,
                is_field: false,
                span: SourceSpan::default(),
            })))
        } else {
            let field_id = self.parse_id_var()?.unwrap_or(Expression::Null(Null));
            Ok(Expression::In(Box::new(In {
                this: column,
                expressions: Vec::new(),
                query: Some(field_id),
                not: false,
                global: false,
                unnest: None,
                is_field: true,
                span: SourceSpan::default(),
            })))
        }
    }

    /// Parses UNPIVOT [INCLUDE|EXCLUDE NULLS] (value FOR name IN (...)), single or multi-column.
    pub(crate) fn parse_unpivot(&mut self, source: Expression) -> Result<Expression> {
        let include_nulls = if self.match_text_seq(&["INCLUDE", "NULLS"]) {
            Some(true)
        } else if self.match_text_seq(&["EXCLUDE", "NULLS"]) {
            Some(false)
        } else {
            None
        };

        self.expect_token(TokenType::LParen)?;

        let (value_column, value_column_parenthesized, extra_value_columns) =
            if self.match_token(TokenType::LParen) {
                let col = self.expect_identifier_or_keyword()?;
                let mut extra_cols: Vec<Identifier> = Vec::new();
                while self.match_token(TokenType::Comma) {
                    extra_cols
                        .push(self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?);
                }
                self.expect_token(TokenType::RParen)?;
                (Identifier::new(col), true, extra_cols)
            } else {
                (
                    self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?,
                    false,
                    Vec::new(),
                )
            };

        self.expect_token(TokenType::For)?;
        let name_column = self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?;

        self.expect_token(TokenType::In)?;
        self.expect_token(TokenType::LParen)?;
        let snowflake = matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        );
        let columns = {
            let mut cols: Vec<Expression> = Vec::new();
            loop {
                if self.check(TokenType::RParen) {
                    if snowflake {
                        return Err(self.parse_error("Expected a column in the UNPIVOT IN list"));
                    }
                    break;
                }
                let col_expr = if self.check(TokenType::LParen) {
                    let saved = self.current;
                    self.skip();
                    let mut tuple_cols: Vec<Expression> = Vec::new();
                    let first = self.expect_identifier_or_keyword();
                    if let Ok(first_id) = first {
                        tuple_cols.push(Expression::column(first_id));
                        while self.match_token(TokenType::Comma) {
                            if let Ok(id) = self.expect_identifier_or_keyword() {
                                tuple_cols.push(Expression::column(id));
                            } else {
                                break;
                            }
                        }
                        if self.match_token(TokenType::RParen) && tuple_cols.len() > 1 {
                            Some(Expression::Tuple(Box::new(Tuple {
                                expressions: tuple_cols,
                            })))
                        } else {
                            self.current = saved;
                            self.parse_select_or_expression()?
                        }
                    } else {
                        self.current = saved;
                        self.parse_select_or_expression()?
                    }
                } else {
                    self.parse_select_or_expression()?
                };

                if let Some(col) = col_expr {
                    if snowflake && self.check(TokenType::As) && self.check_next(TokenType::String)
                    {
                        return Err(self.parse_error("Expected an identifier alias in UNPIVOT IN"));
                    }
                    let col = if self.match_token(TokenType::As) {
                        let alias_expr = self.parse_bitwise()?.ok_or_else(|| {
                            self.parse_error("Expected expression after AS in UNPIVOT IN clause")
                        })?;
                        Expression::PivotAlias(Box::new(PivotAlias {
                            this: col,
                            alias: alias_expr,
                        }))
                    } else {
                        col
                    };
                    cols.push(col);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            cols
        };
        self.expect_token(TokenType::RParen)?;

        self.expect_token(TokenType::RParen)?;

        Ok(Expression::Unpivot(Box::new(Unpivot {
            this: source,
            value_column,
            name_column,
            columns,
            alias: None,
            alias_columns: Vec::new(),
            value_column_parenthesized,
            include_nulls,
            extra_value_columns,
        })))
    }

    /// Parses Redshift FROM-clause UNPIVOT expr [AS value AT attribute] over SUPER objects.
    pub(crate) fn parse_redshift_unpivot_table(&mut self) -> Result<Expression> {
        let this = self.parse_primary()?;

        let alias = if self.match_token(TokenType::As) {
            let val_alias = self.expect_identifier_or_keyword()?;
            if self.match_text_seq(&["AT"]) {
                let attr_alias = self.expect_identifier_or_keyword()?;
                Some(Identifier::new(format!("{} AT {}", val_alias, attr_alias)))
            } else {
                Some(Identifier::new(val_alias))
            }
        } else {
            None
        };

        Ok(Expression::Pivot(Box::new(Pivot {
            this,
            expressions: Vec::new(),
            fields: Vec::new(),
            using: Vec::new(),
            group: None,
            unpivot: true,
            into: None,
            alias,
            alias_columns: Vec::new(),
            include_nulls: None,
            default_on_null: None,
            with: None,
        })))
    }

    /// Parses a BigQuery table part, merging hyphens (project-id) and quoting numeric parts.
    pub(crate) fn parse_bigquery_table_part(&mut self) -> Result<Identifier> {
        let ident = self.expect_identifier_or_keyword_with_quoted()?;

        Ok(ident)
    }

    /// Check if the current token is "connected" to the previous token (no whitespace)
    pub(crate) fn is_connected(&self) -> bool {
        if self.current == 0 || self.current >= self.tokens.len() {
            return false;
        }
        let prev_token = &self.tokens[self.current - 1];
        let curr_token = &self.tokens[self.current];
        prev_token.span.end == curr_token.span.start
    }

    /// Parse a table reference (schema.table format)
    pub(crate) fn parse_table_ref(&mut self) -> Result<TableRef> {
        let table_ref_leading_comments = self.current_leading_comments().to_vec();
        let mut result = self.parse_table_ref_inner()?;
        if !table_ref_leading_comments.is_empty() && result.leading_comments.is_empty() {
            result.leading_comments = table_ref_leading_comments;
        }
        Ok(result)
    }

    pub(crate) fn parse_table_ref_inner(&mut self) -> Result<TableRef> {
        if self.check_identifier("IDENTIFIER") && self.check_next(TokenType::LParen) {
            self.skip();
            self.skip();
            let arg = if self.check(TokenType::String) {
                let s = self.advance_text()?;
                Expression::Literal(Box::new(Literal::String(s)))
            } else if self.check(TokenType::Placeholder) || self.check(TokenType::Parameter) {
                self.skip();
                Expression::Placeholder(Placeholder { index: None })
            } else if self.check(TokenType::Dollar) {
                self.skip();
                let var_name = self.expect_identifier()?;
                Expression::Var(Box::new(crate::expressions::statements::models::Var {
                    this: format!("${}", var_name),
                }))
            } else {
                let ident = self.expect_identifier()?;
                Expression::Identifier(Identifier::new(ident))
            };
            self.expect_token(TokenType::RParen)?;
            let trailing_comments = self.previous_trailing_comments().to_vec();
            let identifier_func =
                Expression::Function(Box::new(crate::expressions::functions::models::Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: "IDENTIFIER".to_string(),
                    args: vec![arg],
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                }));
            return Ok(TableRef {
                catalog: None,
                schema: None,
                name: Identifier::empty(),
                alias: None,
                alias_explicit_as: false,
                column_aliases: Vec::new(),
                leading_comments: Vec::new(),
                trailing_comments,
                when: None,
                only: false,
                final_: false,
                table_sample: None,
                hints: Vec::new(),
                system_time: None,
                partitions: Vec::new(),
                identifier_func: Some(Box::new(identifier_func)),
                changes: None,
                version: None,
                span: None,
            });
        }

        let first = self.parse_bigquery_table_part()?;

        if self.match_token(TokenType::Dot) {
            if self.check(TokenType::Dot) {
                self.skip();
                let table = self.parse_bigquery_table_part()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Ok(TableRef {
                    catalog: Some(first),
                    schema: Some(Identifier::new("")),
                    name: table,
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments,
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                })
            } else {
                let table = self.parse_bigquery_table_part()?;
                if self.match_token(TokenType::Dot) {
                    let actual_table = self.parse_bigquery_table_part()?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(TableRef {
                        catalog: Some(first),
                        schema: Some(table),
                        name: actual_table,
                        alias: None,
                        alias_explicit_as: false,
                        column_aliases: Vec::new(),
                        leading_comments: Vec::new(),
                        trailing_comments,
                        when: None,
                        only: false,
                        final_: false,
                        table_sample: None,
                        hints: Vec::new(),
                        system_time: None,
                        partitions: Vec::new(),
                        identifier_func: None,
                        changes: None,
                        version: None,
                        span: None,
                    })
                } else {
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(TableRef {
                        catalog: None,
                        schema: Some(first),
                        name: table,
                        alias: None,
                        alias_explicit_as: false,
                        column_aliases: Vec::new(),
                        leading_comments: Vec::new(),
                        trailing_comments,
                        when: None,
                        only: false,
                        final_: false,
                        table_sample: None,
                        hints: Vec::new(),
                        system_time: None,
                        partitions: Vec::new(),
                        identifier_func: None,
                        changes: None,
                        version: None,
                        span: None,
                    })
                }
            }
        } else {
            let trailing_comments = self.previous_trailing_comments().to_vec();
            Ok(TableRef {
                catalog: None,
                schema: None,
                name: first,
                alias: None,
                alias_explicit_as: false,
                column_aliases: Vec::new(),
                leading_comments: Vec::new(),
                trailing_comments,
                when: None,
                only: false,
                final_: false,
                table_sample: None,
                hints: Vec::new(),
                system_time: None,
                partitions: Vec::new(),
                identifier_func: None,
                changes: None,
                version: None,
                span: None,
            })
        }
    }

    /// Parse a datetime field for EXTRACT function (YEAR, MONTH, DAY, etc.)
    pub(crate) fn parse_datetime_field(&mut self) -> Result<DateTimeField> {
        let token = self.advance()?;
        let original_name = token.text.to_string();
        let name = original_name.to_ascii_uppercase();
        match name.as_str() {
            "YEAR" => Ok(DateTimeField::Year),
            "MONTH" => Ok(DateTimeField::Month),
            "DAY" => Ok(DateTimeField::Day),
            "HOUR" => Ok(DateTimeField::Hour),
            "MINUTE" => Ok(DateTimeField::Minute),
            "SECOND" => Ok(DateTimeField::Second),
            "MILLISECOND" => Ok(DateTimeField::Millisecond),
            "MICROSECOND" => Ok(DateTimeField::Microsecond),
            "DOW" | "DAYOFWEEK" => Ok(DateTimeField::DayOfWeek),
            "DOY" | "DAYOFYEAR" => Ok(DateTimeField::DayOfYear),
            "WEEK" => {
                if self.match_token(TokenType::LParen) {
                    let modifier = self.expect_identifier_or_keyword()?;
                    self.expect_token(TokenType::RParen)?;
                    Ok(DateTimeField::WeekWithModifier(modifier))
                } else {
                    Ok(DateTimeField::Week)
                }
            }
            "QUARTER" => Ok(DateTimeField::Quarter),
            "EPOCH" => Ok(DateTimeField::Epoch),
            "TIMEZONE" => Ok(DateTimeField::Timezone),
            "TIMEZONE_HOUR" => Ok(DateTimeField::TimezoneHour),
            "TIMEZONE_MINUTE" => Ok(DateTimeField::TimezoneMinute),
            "DATE" => Ok(DateTimeField::Date),
            "TIME" => Ok(DateTimeField::Time),
            _ => Ok(DateTimeField::Custom(original_name)),
        }
    }
}
