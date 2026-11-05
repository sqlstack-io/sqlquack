//! Parser ddl: scripting.

use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Placeholder;
use crate::expressions::queries::models::Limit;
use crate::expressions::statements::models::CopyParameter;
use crate::expressions::statements::models::CopyStmt;
use crate::expressions::statements::models::Credentials;
use crate::expressions::statements::models::Describe;
use crate::expressions::statements::models::PutStmt;
use crate::expressions::statements::models::Schema;
use crate::expressions::statements::models::Show;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::constants::ENGINE;
use crate::parser::constants::MUTEX;
use crate::parser::constants::PLAN;
use crate::parser::constants::REPRODUCTION;
use crate::parser::constants::SECOND_PREVIOUS_OFFSET;
use crate::parser::constants::STATUS;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    pub(crate) fn collect_tsql_block_body_tokens(
        &mut self,
        block_kind: &str,
    ) -> Result<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut depth = 1usize;

        while !self.is_at_end() {
            if self.check(TokenType::Begin)
                && self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1]
                    .text
                    .eq_ignore_ascii_case(block_kind)
            {
                depth += 1;
                tokens.push(self.advance()?);
                tokens.push(self.advance()?);
                continue;
            }

            if self.check(TokenType::End)
                && self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1]
                    .text
                    .eq_ignore_ascii_case(block_kind)
            {
                depth -= 1;
                if depth == 0 {
                    self.skip();
                    self.skip();
                    return Ok(tokens);
                }

                tokens.push(self.advance()?);
                tokens.push(self.advance()?);
                continue;
            }

            tokens.push(self.advance()?);
        }

        Err(self.parse_error(format!("Expected END {}", block_kind)))
    }

    /// Parses DESCRIBE [EXTENDED|FORMATTED|ANALYZE] target, and EXPLAIN.
    pub(crate) fn parse_describe(&mut self) -> Result<Expression> {
        let is_explain =
            self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("EXPLAIN");
        let leading_comments = if self.check(TokenType::Describe)
            || self.check(TokenType::Desc)
            || (self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("EXPLAIN"))
        {
            let token = self.advance()?;
            token.comments
        } else {
            return Err(self.parse_error("Expected DESCRIBE, DESC, or EXPLAIN"));
        };

        if is_explain && self.is_snowflake() && self.match_token(TokenType::Using) {
            return self.parse_snowflake_explain_using(leading_comments);
        }

        if !is_explain && self.is_snowflake() {
            if let Some(describe) = self.parse_snowflake_describe(&leading_comments)? {
                return Ok(describe);
            }
        }

        let analyze_option: bool = is_explain && self.parse_postgres_explain_options()?;

        let extended = self.match_identifier("EXTENDED");
        let formatted = if !extended {
            self.match_identifier("FORMATTED")
        } else {
            false
        };

        let style = self.parse_describe_style(extended, formatted);

        let style = if analyze_option && style.is_none() {
            Some("ANALYZE".to_string())
        } else {
            style
        };

        let kind = self.parse_describe_kind();

        let (mut properties, target) = self.parse_describe_target()?;

        let params = self.parse_describe_params(&kind)?;

        let partition = self.parse_describe_partition()?;

        let as_json = if self.check(TokenType::As)
            && self
                .peek_nth(1)
                .map(|t| t.text.eq_ignore_ascii_case("JSON"))
                == Some(true)
        {
            self.skip();
            self.skip();
            true
        } else {
            false
        };

        if properties.is_empty() {
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                if (self.check(TokenType::Var)
                    || self.check(TokenType::Type)
                    || self.check_keyword())
                    && (!self.is_snowflake()
                        || self.peek_nth(1).map(|token| token.token_type) == Some(TokenType::Eq))
                {
                    let name = self.advance_text()?.to_lowercase();
                    if self.match_token(TokenType::Eq) {
                        let value = self.advance_text()?;
                        properties.push((name, value));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        Ok(Expression::Describe(Box::new(Describe {
            target,
            extended,
            formatted,
            kind,
            properties,
            style,
            partition,
            leading_comments,
            as_json,
            params,
        })))
    }

    fn parse_describe_style(&mut self, extended: bool, formatted: bool) -> Option<String> {
        if !extended && !formatted && self.match_identifier("ANALYZE") {
            Some("ANALYZE".to_string())
        } else {
            if !extended
                && !formatted
                && self.check_identifier("HISTORY")
                && self.peek_nth(1).map(|t| t.token_type) != Some(TokenType::Dot)
            {
                self.skip();
                Some("HISTORY".to_string())
            } else if !extended
                && !formatted
                && self.check_identifier("DETAIL")
                && self.peek_nth(1).map(|t| t.token_type) != Some(TokenType::Dot)
            {
                self.skip();
                Some("DETAIL".to_string())
            } else {
                None
            }
        }
    }

    fn parse_describe_kind(&mut self) -> Option<String> {
        if self.match_identifier("SEMANTIC") {
            if self.match_token(TokenType::View) {
                Some("SEMANTIC VIEW".to_string())
            } else {
                Some("SEMANTIC".to_string())
            }
        } else if self.match_token(TokenType::Table) {
            Some("TABLE".to_string())
        } else if self.match_token(TokenType::View) {
            Some("VIEW".to_string())
        } else if self.match_identifier("DATABASE") {
            Some("DATABASE".to_string())
        } else if self.match_identifier("SCHEMA") {
            Some("SCHEMA".to_string())
        } else if self.match_token(TokenType::Procedure) {
            Some("PROCEDURE".to_string())
        } else if self.match_token(TokenType::Function) {
            Some("FUNCTION".to_string())
        } else if self.match_token(TokenType::Input) {
            Some("INPUT".to_string())
        } else if self.match_token(TokenType::Output) {
            Some("OUTPUT".to_string())
        } else {
            None
        }
    }

    pub(crate) fn parse_describe_target(&mut self) -> Result<(Vec<(String, String)>, Expression)> {
        let properties: Vec<(String, String)> = Vec::new();

        let target = if self.check(TokenType::Select)
            || self.check(TokenType::With)
            || self.check(TokenType::LParen) && {
                let mut depth = 0usize;
                let mut found_select = false;
                for i in 0..100 {
                    match self.peek_nth(i).map(|t| t.token_type) {
                        Some(TokenType::LParen) => depth += 1,
                        Some(TokenType::Select) | Some(TokenType::With) if depth > 0 => {
                            found_select = true;
                            break;
                        }
                        _ => break,
                    }
                }
                found_select
            } {
            self.parse_statement()?
        } else {
            {
                let table = self.parse_table_ref()?;
                Expression::Table(Box::new(table))
            }
        };
        Ok((properties, target))
    }

    fn parse_describe_params(&mut self, kind: &Option<String>) -> Result<Vec<String>> {
        Ok(
            if matches!(kind.as_deref(), Some("PROCEDURE") | Some("FUNCTION"))
                && self.match_token(TokenType::LParen)
            {
                let mut type_args: Vec<String> = Vec::new();
                if !self.check(TokenType::RParen) {
                    loop {
                        let mut parts: Vec<String> = Vec::new();
                        let mut paren_depth = 0usize;
                        while !self.is_at_end() {
                            if self.is_snowflake() && self.check(TokenType::String) {
                                return Err(self.parse_error("Expected an argument data type"));
                            }
                            if self.check(TokenType::LParen) {
                                paren_depth += 1;
                                parts.push(self.advance_text()?);
                            } else if self.check(TokenType::RParen) {
                                if paren_depth == 0 {
                                    break;
                                }
                                paren_depth -= 1;
                                parts.push(self.advance_text()?);
                            } else if self.check(TokenType::Comma) && paren_depth == 0 {
                                break;
                            } else {
                                parts.push(self.advance_text()?);
                            }
                        }
                        type_args.push(parts.join(" ").trim().to_uppercase());
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }
                self.expect_token(TokenType::RParen)?;
                type_args
            } else {
                Vec::new()
            },
        )
    }

    fn parse_describe_partition(&mut self) -> Result<Option<Box<Expression>>> {
        Ok(if self.match_token(TokenType::Partition) {
            self.expect_token(TokenType::LParen)?;
            let mut partition_exprs: Vec<Expression> = Vec::new();
            loop {
                if let Some(expr) = self.parse_conjunction()? {
                    partition_exprs.push(expr);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            let partition =
                Expression::Partition(Box::new(crate::expressions::tables::models::Partition {
                    expressions: partition_exprs,
                    subpartition: false,
                }));
            Some(Box::new(partition))
        } else {
            None
        })
    }

    /// Parses SHOW [TERSE] object_type with its LIKE, IN, STARTS WITH, LIMIT and FROM options.
    pub(crate) fn parse_show(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Show)?;

        let terse = self.match_identifier("TERSE");
        if self.is_snowflake() {
            if let Some(show) = self.parse_snowflake_show(terse)? {
                return Ok(show);
            }
        }

        let (this_parts, target, mutex) = self.parse_show_object_kind()?;

        let this = this_parts.join(" ");
        if self.is_snowflake() && this.is_empty() {
            return Err(self.parse_error("Expected an object kind after SHOW"));
        }

        let history = self.match_identifier("HISTORY");

        let for_target = self.parse_show_for_target()?;

        let like = if self.match_token(TokenType::Like) {
            self.expect_snowflake_like_pattern()?;
            Some(self.parse_primary()?)
        } else {
            None
        };

        let (scope_kind, scope) = self.parse_show_scope(&this)?;

        let starts_with = if self.match_keyword("STARTS") {
            self.match_token(TokenType::With);
            Some(self.parse_primary()?)
        } else {
            None
        };

        let limit = if self.match_token(TokenType::Limit) {
            Some(Box::new(Limit {
                this: self.parse_expression()?,
                percent: false,
                comments: Vec::new(),
            }))
        } else {
            None
        };

        let mut from = if self.match_token(TokenType::From) {
            Some(self.parse_primary()?)
        } else {
            None
        };

        let mut db = if from.is_some() && self.match_token(TokenType::From) {
            Some(self.parse_primary()?)
        } else {
            None
        };

        (db, from) = self.parse_show_index_or_columns_source(db, from, &this);

        let like = if like.is_none() && self.match_token(TokenType::Like) {
            self.expect_snowflake_like_pattern()?;
            Some(self.parse_primary()?)
        } else {
            like
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let privileges = self.parse_show_privileges()?;

        Ok(Expression::Show(Box::new(Show {
            this,
            terse,
            history,
            like,
            scope_kind,
            scope,
            starts_with,
            limit,
            from,
            where_clause,
            for_target,
            db,
            target,
            mutex,
            privileges,
        })))
    }

    fn parse_show_object_kind(
        &mut self,
    ) -> Result<(Vec<String>, Option<Expression>, Option<bool>)> {
        let mut this_parts: Vec<String> = Vec::new();
        let mut target: Option<Expression> = None;
        let mut mutex: Option<bool> = None;

        while !self.is_at_end() {
            let current = self.peek();
            if matches!(
                current.token_type,
                TokenType::Like
                    | TokenType::In
                    | TokenType::From
                    | TokenType::Limit
                    | TokenType::Semicolon
                    | TokenType::Eof
                    | TokenType::Where
                    | TokenType::For
                    | TokenType::Offset
                    | TokenType::Settings
            ) {
                break;
            }
            if current.token_type == TokenType::Comma {
                if !this_parts.is_empty() {
                    let last = this_parts.pop().ok_or_else(|| {
                        crate::tokens::errors::Error::internal(
                            "a value the parser checked is missing",
                        )
                    })?;
                    this_parts.push(format!("{},", last));
                }
                self.skip();
                continue;
            }
            if !this_parts.is_empty() && current.text.eq_ignore_ascii_case("HISTORY") {
                break;
            }
            if current.text.eq_ignore_ascii_case("STARTS") {
                break;
            }
            let joined_check = this_parts.join(" ");
            if joined_check == PLAN && current.token_type == TokenType::Number {
                let id = self.advance_text()?;
                target = Some(Expression::Literal(Box::new(Literal::Number(id))));
                break;
            }
            if current.token_type == TokenType::Var || current.token_type.is_keyword() {
                let joined = this_parts.join(" ");

                if matches!(
                    joined.as_str(),
                    "CREATE AGGREGATE" | "CREATE PIPELINE" | "CREATE PROJECTION"
                ) {
                    let name = self.advance_text()?;
                    target = Some(Expression::Identifier(Identifier::new(name)));
                    break;
                }

                if current.text.eq_ignore_ascii_case("ON") && !this_parts.is_empty() {
                    (target, this_parts) = self.parse_show_on_table(target, this_parts)?;
                    break;
                }

                if current.text.eq_ignore_ascii_case("INTO") && joined == REPRODUCTION {
                    (target, this_parts) = self.parse_show_into_outfile(target, this_parts)?;
                    break;
                }

                if joined == PLAN {
                    if current.text.eq_ignore_ascii_case("JSON") {
                        this_parts.push("JSON".to_string());
                        self.skip();
                        if !self.is_at_end() && self.check(TokenType::Number) {
                            let id = self.advance_text()?;
                            target = Some(Expression::Literal(Box::new(Literal::Number(id))));
                        }
                        break;
                    }
                    if current.token_type == TokenType::Number {
                        let id = self.advance_text()?;
                        target = Some(Expression::Literal(Box::new(Literal::Number(id))));
                        break;
                    }
                }

                this_parts.push(current.text.to_ascii_uppercase());
                self.skip();

                let joined = this_parts.join(" ");
                if matches!(
                    joined.as_str(),
                    "CREATE TABLE"
                        | "CREATE VIEW"
                        | "CREATE DICTIONARY"
                        | "CREATE DATABASE"
                        | "CREATE MATERIALIZED VIEW"
                        | "CREATE LIVE VIEW"
                ) {
                    target = self.parse_show_create_target(target)?;
                    break;
                }

                if joined == ENGINE {
                    (mutex, target) = self.parse_show_engine_target(mutex, target)?;
                    break;
                }
            } else {
                break;
            }
        }
        Ok((this_parts, target, mutex))
    }

    fn parse_show_engine_target(
        &mut self,
        mut mutex: Option<bool>,
        mut target: Option<Expression>,
    ) -> Result<(Option<bool>, Option<Expression>)> {
        if !self.is_at_end() {
            let engine_tok = self.peek();
            if engine_tok.token_type == TokenType::Var || engine_tok.token_type.is_keyword() {
                let engine_name = self.advance_text()?;
                target = Some(Expression::Identifier(Identifier::new(engine_name)));
                if !self.is_at_end() {
                    let next = self.peek();
                    let next_upper = next.text.to_ascii_uppercase();
                    if next_upper == STATUS {
                        self.skip();
                        mutex = Some(false);
                    } else if next_upper == MUTEX {
                        self.skip();
                        mutex = Some(true);
                    }
                }
            }
        }
        Ok((mutex, target))
    }

    fn parse_show_create_target(
        &mut self,
        mut target: Option<Expression>,
    ) -> Result<Option<Expression>> {
        if !self.is_at_end()
            && (self.check(TokenType::Var)
                || self.check(TokenType::QuotedIdentifier)
                || self.is_safe_keyword_as_identifier())
        {
            let table = self.parse_table_ref()?;
            target = Some(Expression::Table(Box::new(table)));
        }
        Ok(target)
    }

    fn parse_show_into_outfile(
        &mut self,
        mut target: Option<Expression>,
        mut this_parts: Vec<String>,
    ) -> Result<(Option<Expression>, Vec<String>)> {
        this_parts.push("INTO".to_string());
        self.skip();
        if !self.is_at_end() && self.peek_text().eq_ignore_ascii_case("OUTFILE") {
            this_parts.push("OUTFILE".to_string());
            self.skip();
            if !self.is_at_end() && self.check(TokenType::String) {
                let filename = self.advance_text()?;
                target = Some(Expression::Literal(Box::new(Literal::String(filename))));
            }
        }
        Ok((target, this_parts))
    }

    fn parse_show_on_table(
        &mut self,
        mut target: Option<Expression>,
        mut this_parts: Vec<String>,
    ) -> Result<(Option<Expression>, Vec<String>)> {
        this_parts.push("ON".to_string());
        self.skip();
        if !self.is_at_end() {
            let next = self.peek();
            if next.text.eq_ignore_ascii_case("TABLE") {
                this_parts.push("TABLE".to_string());
                self.skip();
            }
            if !self.is_at_end() {
                let name_tok = self.peek();
                if name_tok.token_type == TokenType::Var || name_tok.token_type.is_keyword() {
                    let name = self.advance_text()?;
                    target = Some(Expression::Identifier(Identifier::new(name)));
                }
            }
        }
        Ok((target, this_parts))
    }

    fn parse_show_for_target(&mut self) -> Result<Option<Expression>> {
        Ok(if self.match_token(TokenType::For) {
            let mut parts: Vec<String> = Vec::new();
            while !self.is_at_end() {
                let tok = self.peek();
                if matches!(
                    tok.token_type,
                    TokenType::Like
                        | TokenType::In
                        | TokenType::From
                        | TokenType::Limit
                        | TokenType::Semicolon
                        | TokenType::Eof
                        | TokenType::Where
                ) {
                    break;
                }
                if tok.token_type == TokenType::Var
                    || tok.token_type.is_keyword()
                    || tok.token_type == TokenType::Number
                {
                    parts.push(self.advance_text()?);
                } else if tok.token_type == TokenType::String {
                    let text = self.advance_text()?;
                    parts.push(format!("'{}'", text));
                } else {
                    break;
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(Expression::Identifier(Identifier::new(parts.join(" "))))
            }
        } else {
            None
        })
    }

    fn parse_show_scope(&mut self, this: &str) -> Result<(Option<String>, Option<Expression>)> {
        Ok(if self.match_token(TokenType::In) {
            let (kind, scope_obj) = if self.match_keyword("ACCOUNT") {
                (Some("ACCOUNT".to_string()), None)
            } else if self.match_token(TokenType::Database) {
                let scope_obj = if !self.is_at_end()
                    && !self.check(TokenType::Like)
                    && !self.check(TokenType::Limit)
                    && !self.check(TokenType::Semicolon)
                    && !self.check_keyword_text("STARTS")
                {
                    let table = self.parse_table_ref()?;
                    Some(Expression::Table(Box::new(table)))
                } else {
                    None
                };
                (Some("DATABASE".to_string()), scope_obj)
            } else if self.match_token(TokenType::Schema) {
                let scope_obj = if !self.is_at_end()
                    && !self.check(TokenType::Like)
                    && !self.check(TokenType::Limit)
                    && !self.check(TokenType::Semicolon)
                    && !self.check_keyword_text("STARTS")
                {
                    let table = self.parse_table_ref()?;
                    Some(Expression::Table(Box::new(table)))
                } else {
                    None
                };
                (Some("SCHEMA".to_string()), scope_obj)
            } else if self.match_token(TokenType::Table) {
                let scope_obj = if !self.is_at_end()
                    && !self.check(TokenType::Like)
                    && !self.check(TokenType::Limit)
                    && !self.check(TokenType::Semicolon)
                    && !self.check_keyword_text("STARTS")
                {
                    let table = self.parse_table_ref()?;
                    Some(Expression::Table(Box::new(table)))
                } else {
                    None
                };
                (Some("TABLE".to_string()), scope_obj)
            } else if self.match_token(TokenType::View) {
                let scope_obj = if !self.is_at_end()
                    && !self.check(TokenType::Like)
                    && !self.check(TokenType::Limit)
                    && !self.check(TokenType::Semicolon)
                    && !self.check_keyword_text("STARTS")
                {
                    let table = self.parse_table_ref()?;
                    Some(Expression::Table(Box::new(table)))
                } else {
                    None
                };
                (Some("VIEW".to_string()), scope_obj)
            } else if self.match_keyword("CLASS") {
                let scope_obj = if !self.is_at_end() {
                    let table = self.parse_table_ref()?;
                    Some(Expression::Table(Box::new(table)))
                } else {
                    None
                };
                (Some("CLASS".to_string()), scope_obj)
            } else if self.match_keyword("APPLICATION") {
                let kind = if self.match_keyword("PACKAGE") {
                    "APPLICATION PACKAGE".to_string()
                } else {
                    "APPLICATION".to_string()
                };
                let scope_obj = if !self.is_at_end() {
                    let table = self.parse_table_ref()?;
                    Some(Expression::Table(Box::new(table)))
                } else {
                    None
                };
                (Some(kind), scope_obj)
            } else {
                let table = self.parse_table_ref()?;
                let inferred_kind = match this {
                    "SCHEMAS" | "DATABASES" => None,
                    "OBJECTS" | "TABLES" | "VIEWS" | "SEQUENCES" | "UNIQUE KEYS"
                    | "IMPORTED KEYS" => Some("SCHEMA"),
                    "PRIMARY KEYS" => Some("TABLE"),
                    _ => Some("SCHEMA"),
                };
                (
                    inferred_kind.map(|s| s.to_string()),
                    Some(Expression::Table(Box::new(table))),
                )
            };
            (kind, scope_obj)
        } else {
            (None, None)
        })
    }

    fn parse_show_index_or_columns_source(
        &mut self,
        mut db: Option<Expression>,
        mut from: Option<Expression>,
        this: &str,
    ) -> (Option<Expression>, Option<Expression>) {
        if matches!(this, "INDEX" | "COLUMNS") && db.is_none() {
            if let Some(from_expr) = from.take() {
                match from_expr {
                    Expression::Table(mut t) => {
                        if let Some(db_ident) = t.schema.take().or(t.catalog.take()) {
                            db = Some(Expression::Identifier(db_ident));
                            from = Some(Expression::Identifier(t.name));
                        } else {
                            from = Some(Expression::Table(t));
                        }
                    }
                    Expression::Column(c) => {
                        if let Some(table_ident) = c.table {
                            db = Some(Expression::Identifier(table_ident));
                            from = Some(Expression::Identifier(c.name));
                        } else {
                            from = Some(Expression::Column(c));
                        }
                    }
                    Expression::Identifier(id) => {
                        if let Some((db_name, table_name)) = id.name.split_once('.') {
                            db = Some(Expression::Identifier(Identifier::new(db_name)));
                            from = Some(Expression::Identifier(Identifier {
                                name: table_name.to_string(),
                                quoted: id.quoted,
                                trailing_comments: id.trailing_comments,
                                span: None,
                            }));
                        } else {
                            from = Some(Expression::Identifier(id));
                        }
                    }
                    other => {
                        from = Some(other);
                    }
                }
            }
        }
        (db, from)
    }

    fn parse_show_privileges(&mut self) -> Result<Vec<String>> {
        Ok(
            if self.match_token(TokenType::With) && self.match_keyword("PRIVILEGES") {
                let mut privs: Vec<String> = Vec::new();
                loop {
                    if self.is_at_end() || self.check(TokenType::Semicolon) {
                        break;
                    }
                    let tok = self.peek();
                    if tok.token_type == TokenType::Var || tok.token_type.is_keyword() {
                        privs.push(self.advance_text()?.to_ascii_uppercase());
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                privs
            } else {
                Vec::new()
            },
        )
    }

    /// Parses COPY INTO a table or a location (Snowflake, PostgreSQL).
    pub(crate) fn parse_copy(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Copy)?;

        let is_into = self.match_token(TokenType::Into);

        let this = self.parse_copy_target()?;

        let kind = self.match_token(TokenType::From);
        let has_to = if !kind {
            self.match_token(TokenType::To)
        } else {
            false
        };

        let files = self.parse_copy_files(has_to, kind)?;

        let mut params: Vec<CopyParameter> = Vec::new();
        let mut credentials = None;
        let mut with_wrapped = false;

        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let had_with = self.match_token(TokenType::With);

            if self.match_token(TokenType::LParen) {
                if had_with {
                    with_wrapped = true;
                }
                while !self.check(TokenType::RParen) && !self.is_at_end() {
                    let param = self.parse_copy_parameter()?;
                    params.push(param);
                    self.match_token(TokenType::Comma);
                }
                self.expect_token(TokenType::RParen)?;
                break;
            }

            if self.check(TokenType::Var) || self.check_keyword() {
                let param = self.parse_copy_parameter()?;

                if param.name.eq_ignore_ascii_case("CREDENTIALS") {
                    if let Some(Expression::Literal(lit)) = &param.value {
                        if let Literal::String(s) = lit.as_ref() {
                            let creds = Credentials {
                                credentials: vec![("".to_string(), s.clone())],
                                storage: None,
                                encryption: None,
                            };
                            credentials = Some(Box::new(creds));
                        }
                    } else {
                        let creds = Credentials {
                            credentials: param
                                .values
                                .iter()
                                .filter_map(|v| {
                                    if let Expression::Eq(eq) = v {
                                        let key = if let Expression::Column(c) = &eq.left {
                                            c.name.name.clone()
                                        } else {
                                            return None;
                                        };
                                        let val = if let Expression::Literal(lit) = &eq.right {
                                            if let Literal::String(s) = lit.as_ref() {
                                                s.clone()
                                            } else {
                                                String::new()
                                            }
                                        } else {
                                            return None;
                                        };
                                        Some((key, val))
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                            storage: None,
                            encryption: None,
                        };
                        credentials = Some(Box::new(creds));
                    }
                } else {
                    params.push(param);
                }
            } else {
                break;
            }
        }

        Ok(Expression::Copy(Box::new(CopyStmt {
            this,
            kind,
            files,
            params,
            credentials,
            is_into,
            with_wrapped,
        })))
    }

    fn parse_copy_target(&mut self) -> Result<Expression> {
        Ok(if self.check(TokenType::LParen) {
            self.parse_primary()?
        } else if self.check(TokenType::DAt)
            || self.check(TokenType::String)
            || self.is_stage_reference()
        {
            self.parse_file_location()?
        } else {
            let table = self.parse_table_ref()?;
            if self.check(TokenType::LParen) {
                let has_column_list = {
                    let start = self.current;
                    self.skip();
                    let is_select = self.check(TokenType::Select);
                    self.current = start;
                    !is_select
                };
                if has_column_list {
                    self.skip();
                    let mut columns: Vec<String> = Vec::new();
                    loop {
                        let col_name = self.expect_identifier_or_keyword()?;
                        columns.push(col_name);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    Expression::Schema(Box::new(Schema {
                        this: Some(Box::new(Expression::Table(Box::new(table)))),
                        expressions: columns
                            .into_iter()
                            .map(|c| {
                                Self::parsed_column(Column {
                                    name: Identifier::new(c),
                                    table: None,
                                    join_mark: false,
                                    trailing_comments: Vec::new(),
                                    span: None,
                                    inferred_type: None,
                                })
                            })
                            .collect(),
                    }))
                } else {
                    Expression::Table(Box::new(table))
                }
            } else {
                Expression::Table(Box::new(table))
            }
        })
    }

    fn parse_copy_files(&mut self, has_to: bool, kind: bool) -> Result<Vec<Expression>> {
        let mut files: Vec<Expression> = Vec::new();
        if kind
            || has_to
            || self.check(TokenType::String)
            || self.is_stage_reference()
            || self.check(TokenType::LParen)
            || self.check(TokenType::Parameter)
        {
            if self.check(TokenType::Parameter) {
                self.skip();
                files.push(Expression::Placeholder(Placeholder { index: None }));
            } else if self.check(TokenType::LParen) {
                let start = self.current;
                self.skip();
                let is_select = self.check(TokenType::Select);
                self.current = start;
                if is_select {
                    let subquery = self.parse_primary()?;
                    files.push(subquery);
                }
            }
            while !self.is_at_end() && !self.check(TokenType::Semicolon) && files.is_empty()
                || self.check(TokenType::Comma) && !files.is_empty()
            {
                if !files.is_empty() && !self.match_token(TokenType::Comma) {
                    break;
                }
                if (self.check(TokenType::Var) || self.check_keyword())
                    && !self.is_stage_reference()
                {
                    let lookahead = self.current + 1;
                    if lookahead < self.tokens.len()
                        && self.tokens[lookahead].token_type == TokenType::Eq
                    {
                        break;
                    }
                }
                if self.check(TokenType::With) {
                    break;
                }
                if !self.check(TokenType::String)
                    && !self.is_stage_reference()
                    && !self.check(TokenType::Var)
                    && !self.check_keyword()
                    && !self.check(TokenType::QuotedIdentifier)
                    && !self.check(TokenType::Parameter)
                {
                    break;
                }
                if self.check(TokenType::Parameter) {
                    self.skip();
                    files.push(Expression::Placeholder(Placeholder { index: None }));
                    continue;
                }
                if (self.check(TokenType::Var) || self.is_identifier_token())
                    && !self.is_stage_reference()
                {
                    let lookahead = self.current + 1;
                    let has_dot = lookahead < self.tokens.len()
                        && self.tokens[lookahead].token_type == TokenType::Dot;
                    if has_dot {
                        let table = self.parse_table_ref()?;
                        files.push(Expression::Table(Box::new(table)));
                        continue;
                    }
                }
                let location = self.parse_file_location()?;
                files.push(location);
            }
        }
        Ok(files)
    }

    /// Parse a single COPY parameter: NAME = value, NAME = (nested values), or NAME value (no =)
    pub(crate) fn parse_copy_parameter(&mut self) -> Result<CopyParameter> {
        let name = self.expect_identifier_or_keyword()?;

        let mut value = None;
        let mut values: Vec<Expression> = Vec::new();

        let has_eq = self.match_token(TokenType::Eq);

        if has_eq {
            if self.match_token(TokenType::LParen) {
                if self.check(TokenType::String) || self.check(TokenType::Number) {
                    while !self.check(TokenType::RParen) && !self.is_at_end() {
                        values.push(self.parse_primary()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                } else {
                    while !self.check(TokenType::RParen) && !self.is_at_end() {
                        let nested_key = self.expect_identifier_or_keyword()?.to_ascii_uppercase();
                        if self.match_token(TokenType::Eq) {
                            let nested_value = self.parse_copy_param_value()?;
                            values.push(Expression::Eq(Box::new(BinaryOp {
                                span: SourceSpan::default(),
                                left: Self::parsed_column(Column {
                                    name: Identifier::new(nested_key),
                                    table: None,
                                    join_mark: false,
                                    trailing_comments: Vec::new(),
                                    span: None,
                                    inferred_type: None,
                                }),
                                right: nested_value,
                                left_comments: Vec::new(),
                                operator_comments: Vec::new(),
                                trailing_comments: Vec::new(),
                                inferred_type: None,
                            })));
                        } else {
                            values.push(Self::parsed_column(Column {
                                name: Identifier::new(nested_key),
                                table: None,
                                join_mark: false,
                                trailing_comments: Vec::new(),
                                span: None,
                                inferred_type: None,
                            }));
                        }
                        self.match_token(TokenType::Comma);
                    }
                }
                self.expect_token(TokenType::RParen)?;
            } else {
                value = Some(self.parse_copy_param_value()?);
            }
        } else {
            if self.check(TokenType::LParen) {
                let is_varlen_option = matches!(
                    name.as_str(),
                    "FORMAT_OPTIONS" | "COPY_OPTIONS" | "FILE_FORMAT" | "CREDENTIAL"
                );

                self.skip();

                if is_varlen_option {
                    while !self.check(TokenType::RParen) && !self.is_at_end() {
                        if self.check(TokenType::String) {
                            let key_token = self.advance()?;
                            let key = key_token.text.to_string();
                            if self.match_token(TokenType::Eq) {
                                let val = self.parse_copy_param_value()?;
                                values.push(Expression::Eq(Box::new(BinaryOp {
                                    left: Expression::Literal(Box::new(Literal::String(key))),
                                    right: val,
                                    left_comments: Vec::new(),
                                    operator_comments: Vec::new(),
                                    trailing_comments: Vec::new(),
                                    inferred_type: None,
                                    span: SourceSpan::default(),
                                })));
                            } else {
                                values.push(Expression::Literal(Box::new(Literal::String(key))));
                            }
                        } else if self.check(TokenType::Var)
                            || self.check_keyword()
                            || self.is_identifier_token()
                        {
                            let key = self.advance_text()?;
                            if self.match_token(TokenType::Eq) {
                                let val = self.parse_copy_param_value()?;
                                values.push(Expression::Eq(Box::new(BinaryOp {
                                    span: SourceSpan::default(),
                                    left: Self::parsed_column(Column {
                                        name: Identifier::new(key),
                                        table: None,
                                        join_mark: false,
                                        trailing_comments: Vec::new(),
                                        span: None,
                                        inferred_type: None,
                                    }),
                                    right: val,
                                    left_comments: Vec::new(),
                                    operator_comments: Vec::new(),
                                    trailing_comments: Vec::new(),
                                    inferred_type: None,
                                })));
                            } else {
                                values.push(Self::parsed_column(Column {
                                    name: Identifier::new(key),
                                    table: None,
                                    join_mark: false,
                                    trailing_comments: Vec::new(),
                                    span: None,
                                    inferred_type: None,
                                }));
                            }
                        } else {
                            break;
                        }
                        self.match_token(TokenType::Comma);
                    }
                } else {
                    let mut items: Vec<Expression> = Vec::new();
                    while !self.check(TokenType::RParen) && !self.is_at_end() {
                        items.push(self.parse_primary()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    value = Some(Expression::Tuple(Box::new(Tuple { expressions: items })));
                }
                self.expect_token(TokenType::RParen)?;
            } else if self.check(TokenType::LBrace) {
                value = Some(self.parse_primary()?);
            } else if self.check(TokenType::String)
                || self.check(TokenType::Number)
                || self.check(TokenType::True)
                || self.check(TokenType::False)
            {
                value = Some(self.parse_copy_param_value()?);
            } else if !self.check(TokenType::Comma)
                && !self.check(TokenType::RParen)
                && !self.is_at_end()
                && !self.check(TokenType::Semicolon)
            {
                let name_upper = name.to_ascii_uppercase();
                let is_flag_param = matches!(
                    name_upper.as_str(),
                    "EMPTYASNULL"
                        | "BLANKSASNULL"
                        | "ACCEPTINVCHARS"
                        | "COMPUPDATE"
                        | "STATUPDATE"
                        | "NOLOAD"
                        | "ESCAPE"
                        | "REMOVEQUOTES"
                        | "EXPLICIT_IDS"
                        | "FILLRECORD"
                        | "TRIMBLANKS"
                        | "TRUNCATECOLUMNS"
                        | "ROUNDEC"
                        | "IGNOREHEADER"
                        | "IGNOREBLANKLINES"
                        | "ACCEPTANYDATE"
                );
                if !is_flag_param && (self.check(TokenType::Var) || self.check_keyword()) {
                    value = Some(self.parse_copy_param_value()?);
                }
            }
        }

        Ok(CopyParameter {
            name,
            value,
            values,
            eq: has_eq,
        })
    }

    /// Parse a value for COPY parameters (handles strings, identifiers, numbers, lists)
    pub(crate) fn parse_copy_param_value(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::LParen) {
            let mut items: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) && !self.is_at_end() {
                items.push(self.parse_primary()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Tuple(Box::new(Tuple { expressions: items })));
        }

        if self.check(TokenType::String) {
            let token = self.advance()?;
            return Ok(Expression::Literal(Box::new(Literal::String(
                token.text.to_string(),
            ))));
        }
        if self.check(TokenType::QuotedIdentifier) {
            let token = self.advance()?;
            return Ok(Self::parsed_column(Column {
                name: Identifier::quoted(token.text.to_string()).with_span(token.span),
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }));
        }
        if self.check(TokenType::Number) {
            let token = self.advance()?;
            return Ok(Expression::Literal(Box::new(Literal::Number(
                token.text.to_string(),
            ))));
        }
        if self.match_token(TokenType::True) {
            return Ok(Expression::Boolean(BooleanLiteral { value: true }));
        }
        if self.match_token(TokenType::False) {
            return Ok(Expression::Boolean(BooleanLiteral { value: false }));
        }
        if self.check(TokenType::Var) || self.check_keyword() {
            let first = self.advance_text()?;
            if self.match_token(TokenType::Dot) {
                let second = self.expect_identifier_or_keyword()?;
                if self.match_token(TokenType::Dot) {
                    let third = self.expect_identifier_or_keyword()?;
                    return Ok(Self::parsed_column(Column {
                        name: Identifier::new(format!("{}.{}.{}", first, second, third)),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    }));
                }
                return Ok(Self::parsed_column(Column {
                    name: Identifier::new(format!("{}.{}", first, second)),
                    table: None,
                    join_mark: false,
                    trailing_comments: Vec::new(),
                    span: None,
                    inferred_type: None,
                }));
            }
            return Ok(Self::parsed_column(Column {
                name: Identifier::new(first),
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }));
        }

        if self.match_token(TokenType::Parameter) {
            return Ok(Expression::Placeholder(Placeholder { index: None }));
        }

        Err(self.parse_error("Expected value for COPY parameter"))
    }

    /// Parses a Snowflake stage reference tokenized as a string, such as '@stage/path'.
    pub(crate) fn parse_stage_reference_from_string(&mut self) -> Result<Expression> {
        use crate::expressions::statements::models::StageReference;

        let string_token = self.advance()?;
        let full_path = string_token.text.to_string();

        let parts: Vec<&str> = full_path.splitn(2, '/').collect();
        let name = parts[0].to_string();
        let path = if parts.len() > 1 {
            Some(format!("/{}", parts[1]))
        } else {
            None
        };

        let (file_format, pattern) = if self.match_token(TokenType::LParen) {
            let mut ff = None;
            let mut pat = None;

            loop {
                if self.match_identifier("FILE_FORMAT") {
                    self.expect_token(TokenType::FArrow)?;
                    ff = Some(self.parse_primary()?);
                } else if self.match_identifier("PATTERN") || self.match_token(TokenType::Pattern) {
                    self.expect_token(TokenType::FArrow)?;
                    if let Expression::Literal(lit) = self.parse_primary()? {
                        if let Literal::String(s) = lit.as_ref() {
                            pat = Some(s.clone());
                        }
                    }
                } else {
                    break;
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }

            self.expect_token(TokenType::RParen)?;
            (ff, pat)
        } else {
            (None, None)
        };

        Ok(Expression::StageReference(Box::new(StageReference {
            name,
            path,
            file_format,
            pattern,
            quoted: true,
        })))
    }

    /// Parses a Snowflake stage reference tokenized as a Var, such as @stage/path/file.csv.
    pub(crate) fn parse_stage_reference_from_var(&mut self) -> Result<Expression> {
        use crate::expressions::statements::models::StageReference;

        let var_token = self.advance()?;
        let mut name = var_token.text.to_string();

        while self.match_token(TokenType::Dot) {
            name.push('.');
            if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                name.push_str(&self.advance_text()?);
            } else if self.check(TokenType::Percent) {
                self.skip();
                name.push('%');
                if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                    name.push_str(&self.advance_text()?);
                }
            } else {
                break;
            }
        }

        let path = if self.match_token(TokenType::Slash) {
            let mut path_str = String::from("/");
            while !self.is_at_end() {
                if self.check(TokenType::Identifier)
                    || self.check(TokenType::Var)
                    || self.check(TokenType::Number)
                    || self.check(TokenType::Dot)
                    || self.check(TokenType::Dash)
                    || self.check(TokenType::Star)
                    || self.check(TokenType::To)
                    || self.is_safe_keyword_as_identifier()
                {
                    path_str.push_str(&self.advance_text()?);
                } else if self.match_token(TokenType::Slash) {
                    path_str.push('/');
                } else {
                    break;
                }
            }
            Some(path_str)
        } else {
            None
        };

        let (file_format, pattern) = if self.match_token(TokenType::LParen) {
            let mut ff = None;
            let mut pat = None;

            loop {
                if self.match_identifier("FILE_FORMAT") {
                    self.expect_token(TokenType::FArrow)?;
                    ff = Some(self.parse_primary()?);
                } else if self.match_identifier("PATTERN") || self.match_token(TokenType::Pattern) {
                    self.expect_token(TokenType::FArrow)?;
                    if let Expression::Literal(lit) = self.parse_primary()? {
                        if let Literal::String(s) = lit.as_ref() {
                            pat = Some(s.clone());
                        }
                    }
                } else {
                    break;
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }

            self.expect_token(TokenType::RParen)?;
            (ff, pat)
        } else {
            (None, None)
        };

        Ok(Expression::StageReference(Box::new(StageReference {
            name,
            path,
            file_format,
            pattern,
            quoted: false,
        })))
    }

    /// Parses a FROM-clause stage reference: @stage, @ns.stage, @stage/path, @~, @%table.
    pub(crate) fn parse_stage_reference(&mut self) -> Result<Expression> {
        use crate::expressions::statements::models::StageReference;

        self.expect_token(TokenType::DAt)?;

        let mut name = String::from("@");

        if self.check(TokenType::Tilde) {
            self.skip();
            name.push('~');
        } else if self.check(TokenType::Percent) {
            self.skip();
            name.push('%');
            loop {
                if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                    name.push_str(&self.advance_text()?);
                } else {
                    break;
                }
                if self.match_token(TokenType::Dot) {
                    name.push('.');
                } else {
                    break;
                }
            }
        } else {
            loop {
                if self.check(TokenType::QuotedIdentifier) {
                    let text = self.advance_text()?;
                    name.push('"');
                    name.push_str(&text);
                    name.push('"');
                } else if self.check(TokenType::Percent) {
                    self.skip();
                    name.push('%');
                    if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                        name.push_str(&self.advance_text()?);
                    }
                } else if self.check(TokenType::Identifier)
                    || self.check(TokenType::Var)
                    || self.is_safe_keyword_as_identifier()
                {
                    name.push_str(&self.advance_text()?);
                } else {
                    break;
                }

                if self.match_token(TokenType::Dot) {
                    name.push('.');
                } else {
                    break;
                }
            }
        }

        let path = if self.match_token(TokenType::Slash) {
            let mut path_str = String::from("/");
            while !self.is_at_end() {
                if self.check(TokenType::Identifier)
                    || self.check(TokenType::Var)
                    || self.check(TokenType::Number)
                    || self.check(TokenType::Dot)
                    || self.check(TokenType::Dash)
                    || self.check(TokenType::Star)
                    || self.check(TokenType::To)
                    || self.is_safe_keyword_as_identifier()
                {
                    path_str.push_str(&self.advance_text()?);
                } else if self.match_token(TokenType::Slash) {
                    path_str.push('/');
                } else {
                    break;
                }
            }
            Some(path_str)
        } else {
            None
        };

        let (file_format, pattern) = if self.match_token(TokenType::LParen) {
            let mut ff = None;
            let mut pat = None;

            loop {
                if self.match_identifier("FILE_FORMAT") {
                    self.expect_token(TokenType::FArrow)?;
                    ff = Some(self.parse_primary()?);
                } else if self.match_identifier("PATTERN") || self.match_token(TokenType::Pattern) {
                    self.expect_token(TokenType::FArrow)?;
                    if let Expression::Literal(lit) = self.parse_primary()? {
                        if let Literal::String(s) = lit.as_ref() {
                            pat = Some(s.clone());
                        }
                    }
                } else {
                    break;
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }

            self.expect_token(TokenType::RParen)?;
            (ff, pat)
        } else {
            (None, None)
        };

        Ok(Expression::StageReference(Box::new(StageReference {
            name,
            path,
            file_format,
            pattern,
            quoted: false,
        })))
    }

    /// Parses a COPY/PUT file location: a stage, a path, 's3://...' or file:///...
    pub(crate) fn parse_file_location(&mut self) -> Result<Expression> {
        if self.check(TokenType::DAt) {
            self.skip();
            let mut stage_path = String::from("@");

            if self.check(TokenType::Percent) || self.check(TokenType::Mod) {
                stage_path.push('%');
                self.skip();
            } else if self.check(TokenType::Tilde) {
                stage_path.push('~');
                self.skip();
            }

            if self.check(TokenType::Var) || self.check_keyword() || self.is_identifier_token() {
                stage_path.push_str(&self.advance_text()?);
            }
            while self.check(TokenType::Dot) {
                self.skip();
                stage_path.push('.');
                if self.check(TokenType::Var) || self.check_keyword() || self.is_identifier_token()
                {
                    stage_path.push_str(&self.advance_text()?);
                }
            }
            if self.match_token(TokenType::Slash) {
                stage_path.push('/');
                while !self.is_at_end() {
                    if (self.check(TokenType::Var)
                        || self.check(TokenType::Identifier)
                        || self.check(TokenType::Number)
                        || self.check(TokenType::Dot)
                        || self.check(TokenType::Dash)
                        || self.check(TokenType::Star)
                        || self.check(TokenType::To)
                        || self.is_safe_keyword_as_identifier())
                        && !self.check_next(TokenType::Eq)
                    {
                        stage_path.push_str(&self.advance_text()?);
                    } else if self.match_token(TokenType::Slash) {
                        stage_path.push('/');
                    } else {
                        break;
                    }
                }
            }
            return Ok(Expression::Literal(Box::new(Literal::String(stage_path))));
        }

        if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
            let mut stage_path = self.advance_text()?;
            while self.check(TokenType::Dot) {
                self.skip();
                stage_path.push('.');
                if self.check(TokenType::Var) || self.check_keyword() || self.is_identifier_token()
                {
                    stage_path.push_str(&self.advance_text()?);
                }
            }
            if self.match_token(TokenType::Slash) {
                stage_path.push('/');
                while !self.is_at_end() {
                    if (self.check(TokenType::Var)
                        || self.check(TokenType::Identifier)
                        || self.check(TokenType::Number)
                        || self.check(TokenType::Dot)
                        || self.check(TokenType::Dash)
                        || self.check(TokenType::Star)
                        || self.check(TokenType::To)
                        || self.is_safe_keyword_as_identifier())
                        && !self.check_next(TokenType::Eq)
                    {
                        stage_path.push_str(&self.advance_text()?);
                    } else if self.match_token(TokenType::Slash) {
                        stage_path.push('/');
                    } else {
                        break;
                    }
                }
            }
            return Ok(Expression::Literal(Box::new(Literal::String(stage_path))));
        }

        if self.check(TokenType::String) {
            let token = self.advance()?;
            return Ok(Expression::Literal(Box::new(Literal::String(
                token.text.to_string(),
            ))));
        }

        if self.check(TokenType::QuotedIdentifier) {
            let token = self.advance()?;
            return Ok(Expression::Identifier(Identifier::quoted(
                token.text.to_string(),
            )));
        }

        if self.check(TokenType::Var) || self.check_keyword() {
            let ident = self.advance_text()?;
            return Ok(Self::parsed_column(Column {
                name: Identifier::new(ident),
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }));
        }

        Err(self.parse_error("Expected file location"))
    }

    /// Parses a Snowflake stage reference for PUT/GET/COPY as a string literal.
    pub(crate) fn parse_stage_reference_as_string(&mut self) -> Result<Expression> {
        if self.check(TokenType::DAt) {
            self.skip();
            let mut stage_path = String::from("@");

            if self.check(TokenType::Percent) || self.check(TokenType::Mod) {
                stage_path.push('%');
                self.skip();
            } else if self.check(TokenType::Tilde) {
                stage_path.push('~');
                self.skip();
                while self.check(TokenType::Slash) {
                    self.skip();
                    stage_path.push('/');
                    while (self.check(TokenType::Var)
                        || self.check_keyword()
                        || self.is_identifier_token()
                        || self.check(TokenType::Number)
                        || self.check(TokenType::Dash))
                        && !self.check_next(TokenType::Eq)
                    {
                        stage_path.push_str(&self.advance_text()?);
                    }
                }
                return Ok(Expression::Literal(Box::new(Literal::String(stage_path))));
            }

            if self.check(TokenType::QuotedIdentifier) {
                let text = &self.peek_text();
                stage_path.push('"');
                stage_path.push_str(text);
                stage_path.push('"');
                self.skip();
            } else if self.check(TokenType::Var)
                || self.check_keyword()
                || self.check(TokenType::Identifier)
            {
                stage_path.push_str(&self.advance_text()?);
            }

            while self.check(TokenType::Dot) {
                self.skip();
                stage_path.push('.');
                if self.check(TokenType::QuotedIdentifier) {
                    let text = &self.peek_text();
                    stage_path.push('"');
                    stage_path.push_str(text);
                    stage_path.push('"');
                    self.skip();
                } else if self.check(TokenType::Var)
                    || self.check_keyword()
                    || self.check(TokenType::Identifier)
                {
                    stage_path.push_str(&self.advance_text()?);
                }
            }

            while self.check(TokenType::Slash) {
                self.skip();
                stage_path.push('/');
                while (self.check(TokenType::Var)
                    || self.check_keyword()
                    || self.is_identifier_token()
                    || self.check(TokenType::Number)
                    || self.check(TokenType::Dash))
                    && !self.check_next(TokenType::Eq)
                {
                    stage_path.push_str(&self.advance_text()?);
                }
            }
            return Ok(Expression::Literal(Box::new(Literal::String(stage_path))));
        }

        if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
            let mut stage_path = self.advance_text()?;

            while self.check(TokenType::Dot) {
                self.skip();
                stage_path.push('.');
                if self.check(TokenType::QuotedIdentifier) {
                    let text = &self.peek_text();
                    stage_path.push('"');
                    stage_path.push_str(text);
                    stage_path.push('"');
                    self.skip();
                } else if self.check(TokenType::Var)
                    || self.check_keyword()
                    || self.check(TokenType::Identifier)
                {
                    stage_path.push_str(&self.advance_text()?);
                }
            }

            while self.check(TokenType::Slash) {
                self.skip();
                stage_path.push('/');
                while (self.check(TokenType::Var)
                    || self.check_keyword()
                    || self.is_identifier_token()
                    || self.check(TokenType::Number)
                    || self.check(TokenType::Dash))
                    && !self.check_next(TokenType::Eq)
                {
                    stage_path.push_str(&self.advance_text()?);
                }
            }
            return Ok(Expression::Literal(Box::new(Literal::String(stage_path))));
        }

        Err(self.parse_error("Expected stage reference starting with @"))
    }

    /// Parses Snowflake PUT file://path @stage [options].
    pub(crate) fn parse_put(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Put)?;

        let (source, source_quoted) = if self.check(TokenType::String) {
            (self.advance_text()?, true)
        } else {
            let mut source_parts: Vec<String> = Vec::new();
            while !self.is_at_end() {
                if self.check(TokenType::DAt) {
                    break;
                }
                if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
                    break;
                }
                if self.check(TokenType::Parameter)
                    || self.check(TokenType::String)
                    || self.check(TokenType::DollarString)
                    || self.check(TokenType::Semicolon)
                {
                    break;
                }
                let token = self.advance()?;
                source_parts.push(token.text.to_string());
            }
            (source_parts.join(""), false)
        };

        let target = if self.match_token(TokenType::Parameter) {
            Expression::Placeholder(Placeholder { index: None })
        } else if self.check(TokenType::String) || self.check(TokenType::DollarString) {
            let tok = self.advance()?;
            Expression::Literal(Box::new(Literal::String(tok.text.to_string())))
        } else {
            self.parse_stage_reference_as_string()?
        };

        let mut params: Vec<CopyParameter> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let is_param_name = self.check(TokenType::Var)
                || self.check_keyword()
                || self.check(TokenType::Overwrite);
            if is_param_name {
                let name = self.advance_text()?;
                let value = if self.match_token(TokenType::Eq) {
                    Some(self.parse_primary()?)
                } else {
                    None
                };
                params.push(CopyParameter {
                    name,
                    value,
                    values: Vec::new(),
                    eq: true,
                });
            } else {
                break;
            }
        }

        Ok(Expression::Put(Box::new(PutStmt {
            source,
            source_quoted,
            target,
            params,
        })))
    }

    /// Joins command tokens, keeping file paths and stage references intact.
    pub(crate) fn join_command_tokens(&self, tokens: Vec<(String, TokenType)>) -> String {
        let mut result = String::new();
        let mut prev_token_type: Option<TokenType> = None;
        let mut prev_prev_token_type: Option<TokenType> = None;

        for (i, (text, token_type)) in tokens.iter().enumerate() {
            let needs_space = if result.is_empty() {
                false
            } else {
                match (prev_token_type, *token_type) {
                    (Some(TokenType::DAt), _) => false,
                    (Some(TokenType::Dot), _) => false,
                    (_, TokenType::Dot) => false,
                    (Some(TokenType::LParen), _) => false,
                    (_, TokenType::LParen) => false,
                    (_, TokenType::RParen) => false,
                    (Some(TokenType::LBracket), _) => false,
                    (_, TokenType::LBracket) => false,
                    (_, TokenType::RBracket) => false,
                    (_, TokenType::Comma) => false,
                    (Some(TokenType::Slash), _) => false,
                    (_, TokenType::Slash) => false,
                    (Some(TokenType::Colon), _) => false,
                    (_, TokenType::Colon) => false,
                    (Some(TokenType::Mod), _) => false,
                    (_, TokenType::Mod) => false,
                    (Some(TokenType::Percent), _) => false,
                    (_, TokenType::Percent) => false,
                    (Some(TokenType::Var), TokenType::Eq) => {
                        if i >= 1 && tokens[i - 1].0.starts_with('@') {
                            true
                        } else if i + 1 < tokens.len() {
                            let next_type = tokens[i + 1].1;
                            let is_terminal_value =
                                i + 2 >= tokens.len() || tokens[i + 2].1 == TokenType::Var;
                            match next_type {
                                TokenType::Number | TokenType::True | TokenType::False => {
                                    !is_terminal_value
                                }
                                TokenType::String => !is_terminal_value,
                                _ => true,
                            }
                        } else {
                            true
                        }
                    }
                    (Some(TokenType::Eq), TokenType::Number)
                    | (Some(TokenType::Eq), TokenType::True)
                    | (Some(TokenType::Eq), TokenType::False)
                    | (Some(TokenType::Eq), TokenType::String) => {
                        let is_terminal =
                            i + 1 >= tokens.len() || tokens[i + 1].1 == TokenType::Var;
                        match prev_prev_token_type {
                            Some(TokenType::Var) => {
                                if i >= SECOND_PREVIOUS_OFFSET
                                    && tokens[i - SECOND_PREVIOUS_OFFSET].0.starts_with('@')
                                {
                                    true
                                } else {
                                    !is_terminal
                                }
                            }
                            _ => true,
                        }
                    }
                    (Some(TokenType::Eq), TokenType::Var) => true,
                    (Some(TokenType::DColon), _) => false,
                    (_, TokenType::DColon) => false,
                    _ => true,
                }
            };

            if needs_space {
                result.push(' ');
            }
            result.push_str(text);
            prev_prev_token_type = prev_token_type;
            prev_token_type = Some(*token_type);
        }
        result
    }

    pub(crate) fn raw_sql_token_text(&self, token: &crate::tokens::models::Token) -> String {
        match token.token_type {
            TokenType::QuotedIdentifier => format!("`{}`", token.text),
            TokenType::String => format!("'{}'", token.text),
            _ => token.text.to_string(),
        }
    }

    pub(crate) fn format_identifier_for_raw_sql(&self, ident: &Identifier) -> String {
        if ident.quoted || ident.name.chars().any(char::is_whitespace) {
            format!("`{}`", ident.name)
        } else {
            ident.name.clone()
        }
    }

    pub(crate) fn format_charset_for_raw_sql(&self, ident: &Identifier) -> String {
        if ident.name.chars().any(char::is_whitespace) {
            format!("`{}`", ident.name)
        } else {
            ident.name.clone()
        }
    }
}
