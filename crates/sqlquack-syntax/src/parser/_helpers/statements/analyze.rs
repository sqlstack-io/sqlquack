//! Parser statements: analyze.

use crate::tokens::models::SourceSpan;
#[cfg(test)]
use std::sync::Arc;

use crate::expressions::names::main::identifier_from_token::identifier_from_token;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::DefaultKeyword;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::LambdaExpr;
use crate::expressions::operators::models::NamedArgSeparator;
use crate::expressions::operators::models::NamedArgument;
use crate::expressions::statements::models::AlterSession;
use crate::expressions::statements::models::Analyze;
use crate::expressions::statements::models::AnalyzeDelete;
use crate::expressions::statements::models::AnalyzeHistogram;
use crate::expressions::statements::models::AnalyzeListChainedRows;
use crate::expressions::statements::models::AnalyzeSample;
use crate::expressions::statements::models::AnalyzeStatistics;
use crate::expressions::statements::models::AnalyzeValidate;
use crate::expressions::statements::models::Annotated;
use crate::expressions::tables::models::IndexKeyPart;
use crate::expressions::tables::models::Property;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::parser::constants::BUFFER_USAGE_LIMIT;
use crate::parser::constants::PERCENT_SIGN;
use crate::parser::models::Parser;
#[cfg(test)]
use crate::parser::models::ParserConfig;
use crate::tokens::models::Span;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Check if current token is FROM keyword
    pub(crate) fn check_from_keyword(&self) -> bool {
        self.check(TokenType::From)
    }

    /// Check if next token matches type
    pub(crate) fn check_next(&self, token_type: TokenType) -> bool {
        if self.current + 1 >= self.tokens.len() {
            false
        } else {
            self.tokens[self.current + 1].token_type == token_type
        }
    }

    /// Check if next token is an identifier with specific name (case-insensitive)
    pub(crate) fn check_next_identifier(&self, name: &str) -> bool {
        if self.current + 1 >= self.tokens.len() {
            false
        } else {
            let token = &self.tokens[self.current + 1];
            (token.token_type == TokenType::Var || token.token_type == TokenType::Identifier)
                && token.text.eq_ignore_ascii_case(name)
        }
    }

    /// Consumes an unquoted identifier with this text, ignoring case.
    pub(crate) fn match_identifier(&mut self, text: &str) -> bool {
        if (self.check(TokenType::Identifier) || self.check(TokenType::Var))
            && self.peek_text().eq_ignore_ascii_case(text)
        {
            self.skip();
            true
        } else {
            false
        }
    }

    /// Whether the current token is an unquoted identifier with this text, ignoring case.
    pub(crate) fn check_identifier(&self, text: &str) -> bool {
        if self.is_at_end() {
            return false;
        }
        (self.check(TokenType::Identifier) || self.check(TokenType::Var))
            && self.peek_text().eq_ignore_ascii_case(text)
    }

    /// Whether the current Percent token is a PERCENT modifier, not a modulo operator.
    pub(crate) fn is_percent_modifier(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.peek_text().eq_ignore_ascii_case("PERCENT") {
            return true;
        }
        if self.peek_text() == PERCENT_SIGN {
            let next_idx = self.current + 1;
            if next_idx >= self.tokens.len() {
                return true;
            }
            let next_type = self.tokens[next_idx].token_type;
            return matches!(
                next_type,
                TokenType::Offset
                    | TokenType::Semicolon
                    | TokenType::RParen
                    | TokenType::From
                    | TokenType::Where
                    | TokenType::GroupBy
                    | TokenType::OrderBy
                    | TokenType::Having
                    | TokenType::Union
                    | TokenType::Intersect
                    | TokenType::Except
                    | TokenType::Comma
                    | TokenType::With
            ) || next_idx >= self.tokens.len();
        }
        false
    }

    /// Whether a keyword may be an identifier: structural ones (FROM, WHERE, ...) may not.
    pub(crate) fn is_safe_keyword_as_identifier(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token_type = self.peek().token_type;
        let is_structural = matches!(
            token_type,
            TokenType::From
                | TokenType::Where
                | TokenType::Select
                | TokenType::Insert
                | TokenType::Delete
                | TokenType::Create
                | TokenType::Drop
                | TokenType::Alter
                | TokenType::Join
                | TokenType::Inner
                | TokenType::Cross
                | TokenType::On
                | TokenType::GroupBy
                | TokenType::OrderBy
                | TokenType::Having
                | TokenType::With
                | TokenType::Union
                | TokenType::Intersect
                | TokenType::Except
                | TokenType::Qualify
                | TokenType::Into
                | TokenType::Set
                | TokenType::Using
                | TokenType::Lateral
                | TokenType::Natural
        );

        self.peek().token_type.is_keyword() && !is_structural
    }

    /// Whether the token here ends the expression, so IS or KEEP is an alias, not an operator.
    pub(crate) fn is_last_expression_token(&self, _token_type: TokenType) -> bool {
        let next_idx = self.current + 1;
        if next_idx >= self.tokens.len() {
            return true;
        }
        if self.tokens[next_idx].token_type == TokenType::Eof {
            return true;
        }
        let next_type = self.tokens[next_idx].token_type;
        matches!(
            next_type,
            TokenType::From
                | TokenType::Where
                | TokenType::GroupBy
                | TokenType::OrderBy
                | TokenType::Having
                | TokenType::Limit
                | TokenType::Union
                | TokenType::Intersect
                | TokenType::Except
                | TokenType::Semicolon
                | TokenType::RParen
                | TokenType::Comma
        )
    }

    /// Check if current token is a type keyword (for lambda type annotations)
    pub(crate) fn is_type_keyword(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token = self.peek();
        let text_upper = token.text.to_ascii_uppercase();
        DataType::from_unsigned_name(&text_upper).is_some()
            || matches!(
                text_upper.as_str(),
                "INT"
                    | "INTEGER"
                    | "BIGINT"
                    | "SMALLINT"
                    | "TINYINT"
                    | "DOUBLE"
                    | "FLOAT"
                    | "DECIMAL"
                    | "NUMERIC"
                    | "REAL"
                    | "VARCHAR"
                    | "CHAR"
                    | "TEXT"
                    | "STRING"
                    | "NVARCHAR"
                    | "NCHAR"
                    | "BOOLEAN"
                    | "BOOL"
                    | "DATE"
                    | "TIME"
                    | "TIMESTAMP"
                    | "DATETIME"
                    | "INTERVAL"
                    | "BINARY"
                    | "VARBINARY"
                    | "BLOB"
                    | "ARRAY"
                    | "MAP"
                    | "STRUCT"
                    | "OBJECT"
                    | "VARIANT"
                    | "JSON"
                    | "NUMBER"
                    | "VARCHAR2"
            )
    }

    /// Whether an implicit relation alias follows, respecting clause and join boundaries.
    pub(crate) fn can_parse_implicit_table_alias(&self) -> bool {
        if self.is_at_end() {
            return false;
        }

        let dialect_keyword_alias = match self.config.dialect {
            Some(crate::tokens::models::DialectType::DuckDB) => matches!(
                self.peek().token_type,
                TokenType::Top
                    | TokenType::First
                    | TokenType::Last
                    | TokenType::Begin
                    | TokenType::Type
                    | TokenType::Owned
                    | TokenType::Match
                    | TokenType::Final
            ),

            _ => false,
        };

        self.check(TokenType::QuotedIdentifier)
            || ((self.check(TokenType::Var)
                && !self.check_keyword()
                && !self.check_identifier("MATCH_CONDITION"))
                && !(self.check_identifier("OPTION") && self.check_next(TokenType::LParen))
                && !(self.check_identifier("LOCK") && self.check_next(TokenType::In)))
                && !(self.check_identifier("POSITIONAL") && self.check_next(TokenType::Join))
            || self.is_command_keyword_as_alias()
            || dialect_keyword_alias
            || self.check(TokenType::Pivot) && !self.check_next(TokenType::LParen)
            || self.check(TokenType::Unpivot) && !self.is_unpivot_clause_start()
            || self.check(TokenType::Partition)
            || self.check(TokenType::Window) && {
                let next_pos = self.current + 1;
                next_pos >= self.tokens.len()
                    || self.tokens[next_pos].token_type != TokenType::Var
                        && self.tokens[next_pos].token_type != TokenType::Identifier
            }
    }

    /// Whether a command keyword (GET, PUT, COPY, SHOW, ...) here is an implicit alias.
    pub(crate) fn is_command_keyword_as_alias(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token_type = self.peek().token_type;
        if matches!(token_type, TokenType::Format) {
            return !false;
        }
        if matches!(
            token_type,
            TokenType::Get
                | TokenType::Put
                | TokenType::Copy
                | TokenType::Show
                | TokenType::Rename
                | TokenType::Enum
                | TokenType::Sample
                | TokenType::Collate
                | TokenType::Add
        ) {
            return true;
        }

        false
    }

    /// Whether the current token is an unquoted Snowflake reserved keyword in a Snowflake parse.
    pub(crate) fn is_snowflake_reserved_keyword(&self) -> bool {
        matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && !self.is_at_end()
            && self.peek().token_type != TokenType::QuotedIdentifier
            && crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS
                .iter()
                .any(|keyword| self.peek().text.eq_ignore_ascii_case(keyword))
    }

    /// Whether a keyword may be a table alias; LEFT, RIGHT, OUTER and FULL may.
    pub(crate) fn can_be_alias_keyword(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token_type = self.peek().token_type;
        matches!(
            token_type,
            TokenType::Left
                | TokenType::Right
                | TokenType::Outer
                | TokenType::Full
                | TokenType::Only
                | TokenType::Next
                | TokenType::All
                | TokenType::If
        ) || self.is_safe_keyword_as_identifier()
    }

    /// Match and consume a token type
    pub(crate) fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.skip();
            true
        } else {
            false
        }
    }

    /// Match a sequence of keywords
    pub(crate) fn match_keywords(&mut self, keywords: &[TokenType]) -> bool {
        for (i, &kw) in keywords.iter().enumerate() {
            if self.current + i >= self.tokens.len() {
                return false;
            }
            if self.tokens[self.current + i].token_type != kw {
                return false;
            }
        }

        self.current += keywords.len();
        true
    }

    /// Expect a specific token type
    pub(crate) fn expect_token(&mut self, token_type: TokenType) -> Result<Token> {
        if self.check(token_type) {
            self.advance()
        } else {
            let got = if self.is_at_end() {
                "end of input".to_string()
            } else {
                format!("{:?}", self.peek().token_type)
            };
            let got_text = if self.is_at_end() {
                "".to_string()
            } else {
                self.peek_text().to_string()
            };
            let start = self.current.saturating_sub(3);
            let end = (self.current + 4).min(self.tokens.len());
            let context = self.tokens_to_sql(start, end).replace('\n', " ");
            Err(self.parse_error(format!(
                "Expected {:?}, got {} ('{}') near [{}]",
                token_type, got, got_text, context
            )))
        }
    }

    /// Expects `>`, splitting a `>>` token, for nested generics like `ARRAY<ARRAY<INT>>`.
    pub(crate) fn expect_gt(&mut self) -> Result<Token> {
        if self.check(TokenType::Gt) {
            self.advance()
        } else if self.check(TokenType::GtGt) {
            self.if_expr_ruled_out.clear();
            let token = self.peek().clone();
            self.tokens[self.current] = Token {
                token_type: TokenType::Gt,
                text: ">".to_string(),
                span: Span {
                    start: token.span.start + 1,
                    end: token.span.end,
                    line: token.span.line,
                    column: token.span.column + 1,
                },
                comments: Vec::new(),
                trailing_comments: Vec::new(),
            }
            .into();
            Ok(Token {
                token_type: TokenType::Gt,
                text: ">".to_string(),
                span: Span {
                    start: token.span.start,
                    end: token.span.start + 1,
                    line: token.span.line,
                    column: token.span.column,
                },
                comments: token.comments,
                trailing_comments: Vec::new(),
            })
        } else {
            Err(self.parse_error(format!(
                "Expected Gt, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Expect a string literal and return its value
    pub(crate) fn expect_string(&mut self) -> Result<String> {
        if self.check(TokenType::String) || self.check(TokenType::DollarString) {
            self.advance_text()
        } else {
            Err(self.parse_error(format!(
                "Expected string, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Check if the current token is any kind of identifier (regular, quoted, or var)
    pub(crate) fn is_identifier_token(&self) -> bool {
        self.check(TokenType::Var)
            || self.check(TokenType::Identifier)
            || self.check(TokenType::QuotedIdentifier)
    }

    /// Whether the current token is a stage reference starting with @.
    pub(crate) fn is_stage_reference(&self) -> bool {
        self.check(TokenType::DAt)
            || self.check(TokenType::Var) && self.peek_text().starts_with('@')
    }

    /// Whether a number token joined to a name token starts a MySQL identifier like `1d`.
    pub(crate) fn is_mysql_numeric_identifier(&self) -> bool {
        false
    }

    /// Parses a MySQL identifier starting with digits, such as `00f`.
    pub(crate) fn parse_mysql_numeric_identifier(&mut self) -> Result<Identifier> {
        let start = self.peek().span;
        let num_token = self.advance()?;
        let mut name = num_token.text.to_string();
        while !self.is_at_end()
            && self.is_connected()
            && (self.check(TokenType::Var) || self.check(TokenType::Identifier))
        {
            let Ok(tok) = self.advance() else { break };
            name.push_str(&tok.text);
        }
        Ok(Identifier {
            name,
            quoted: true,
            trailing_comments: Vec::new(),
            span: Some(start.through(self.previous().span)),
        })
    }

    /// Check if the current token can be used as an identifier (includes keywords)
    pub(crate) fn is_identifier_or_keyword_token(&self) -> bool {
        self.is_identifier_token() || self.check_keyword()
    }

    /// Preserve the range when a legacy grammar path returns an unquoted name.
    pub(crate) fn parse_unquoted_identifier(
        &mut self,
        parse_name: fn(&mut Self) -> Result<String>,
    ) -> Result<Identifier> {
        let start = self.peek().span;
        let name = parse_name(self)?;
        Ok(Identifier::new(name).with_span(start.through(self.previous().span)))
    }

    /// An identifier parsed by `parse_name` that keeps its span and whether it was quoted.
    pub(crate) fn parse_quoting_identifier(
        &mut self,
        parse_name: fn(&mut Self) -> Result<String>,
    ) -> Result<Identifier> {
        let quoted = self.check(TokenType::QuotedIdentifier);
        let mut identifier = self.parse_unquoted_identifier(parse_name)?;
        identifier.quoted = quoted;
        Ok(identifier)
    }

    /// A column whose range comes only from source-backed identifier parts.
    pub(crate) fn parsed_column(mut column: Column) -> Expression {
        if column.span.is_none() {
            column.span = match (&column.table, column.name.span) {
                (None, span) => span,
                (Some(table), Some(end)) => table.span.map(|start| start.through(end)),
                _ => None,
            };
        }
        Expression::boxed_column(column)
    }

    /// Range of a source-backed reference, including chained struct fields.
    pub(crate) fn reference_span(expr: &Expression) -> Option<Span> {
        match expr {
            Expression::Column(column) => column.span,
            Expression::Identifier(identifier) => identifier.span,
            Expression::Dot(dot) => Self::reference_span(&dot.this)
                .zip(dot.field.span)
                .map(|(start, end)| start.through(end)),
            _ => None,
        }
    }

    /// Expect an identifier and return an Identifier struct with quoted flag
    pub(crate) fn expect_identifier_with_quoted(&mut self) -> Result<Identifier> {
        if self.is_mysql_numeric_identifier() {
            return self.parse_mysql_numeric_identifier();
        }
        if self.is_identifier_token() {
            let token = self.advance()?;
            let quoted = token.token_type == TokenType::QuotedIdentifier;
            Ok(identifier_from_token(token, quoted))
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Parse a possibly dot-qualified identifier into parts (e.g. "mydb.hr" → [mydb, hr]).
    pub(crate) fn parse_identifier_parts(&mut self) -> Result<Vec<Identifier>> {
        let first = self.expect_identifier_with_quoted()?;
        let mut parts = vec![first];
        while self.match_token(TokenType::Dot) {
            parts.push(self.expect_identifier_with_quoted()?);
        }
        Ok(parts)
    }

    /// Whether the active dialect supports string literals as projection aliases.
    pub(crate) fn supports_string_aliases(&self, explicit_as: bool) -> bool {
        self.config
            .dialect
            .is_some_and(|dialect| dialect.supports_string_aliases(explicit_as))
    }

    /// Parse a SELECT projection alias without widening identifier parsing elsewhere.
    pub(crate) fn parse_projection_alias_identifier(
        &mut self,
        explicit_as: bool,
    ) -> Result<Identifier> {
        if self.check(TokenType::String) && self.supports_string_aliases(explicit_as) {
            let token = self.advance()?;
            Ok(identifier_from_token(token, true))
        } else {
            self.expect_identifier_or_keyword_with_quoted()
        }
    }

    /// Expect an identifier or keyword (for column names, field names, etc.)
    pub(crate) fn expect_identifier_or_keyword_with_quoted(&mut self) -> Result<Identifier> {
        if self.is_mysql_numeric_identifier() {
            return self.parse_mysql_numeric_identifier();
        }
        if self.check(TokenType::Parameter) {
            let token = self.advance()?;
            let name = if token.text.chars().all(|c| c.is_ascii_digit()) && !token.text.is_empty() {
                format!("${}", token.text)
            } else {
                "?".to_string()
            };
            return Ok(Identifier {
                name,
                quoted: false,
                trailing_comments: Vec::new(),
                span: Some(token.span),
            });
        }
        if self.is_identifier_or_keyword_token() {
            let token = self.advance()?;
            let quoted = token.token_type == TokenType::QuotedIdentifier;
            Ok(identifier_from_token(token, quoted))
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Expect an identifier
    pub(crate) fn expect_identifier(&mut self) -> Result<String> {
        if self.is_identifier_token() {
            self.advance_text()
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Expect an identifier or keyword (for aliases, column names, etc.)
    pub(crate) fn expect_identifier_or_keyword(&mut self) -> Result<String> {
        if self.is_identifier_or_keyword_token() {
            self.advance_text()
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Expects an identifier or a non-structural keyword, for CTE and column names.
    pub(crate) fn expect_identifier_or_safe_keyword(&mut self) -> Result<String> {
        if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            self.advance_text()
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Expect an identifier or safe keyword, preserving quoted flag
    pub(crate) fn expect_identifier_or_safe_keyword_with_quoted(&mut self) -> Result<Identifier> {
        if self.is_mysql_numeric_identifier() {
            return self.parse_mysql_numeric_identifier();
        }
        if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            let token = self.advance()?;
            let quoted = token.token_type == TokenType::QuotedIdentifier;
            Ok(identifier_from_token(token, quoted))
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    pub(crate) fn expect_identifier_or_alias_keyword_with_quoted(&mut self) -> Result<Identifier> {
        if self.is_identifier_token()
            || self.can_be_alias_keyword()
            || self.is_safe_keyword_as_identifier()
        {
            let token = self.advance()?;
            let quoted = token.token_type == TokenType::QuotedIdentifier;
            Ok(identifier_from_token(token, quoted))
        } else if self.check(TokenType::String)
            && matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::DuckDB)
            )
        {
            let token = self.advance()?;
            Ok(identifier_from_token(token, true))
        } else {
            Err(self.parse_error(format!(
                "Expected identifier, got {:?}",
                if self.is_at_end() {
                    "end of input".to_string()
                } else {
                    format!("{:?}", self.peek().token_type)
                }
            )))
        }
    }

    /// Expect a number
    pub(crate) fn expect_number(&mut self) -> Result<i64> {
        let negative = self.match_token(TokenType::Dash);
        if self.check(TokenType::Number) {
            let text = self.advance_text()?;
            let val = text
                .parse::<i64>()
                .map_err(|_| self.parse_error(format!("Invalid number: {}", text)))?;
            Ok(if negative { -val } else { val })
        } else {
            Err(self.parse_error("Expected number"))
        }
    }

    #[cfg(test)]
    pub(crate) fn tidb_fragment_parser(&self, payload: &str) -> Result<Parser> {
        let mut tokenizer_config = crate::tokens::models::TokenizerConfig::default();
        tokenizer_config.identifiers.insert('`', '`');
        tokenizer_config.nested_comments = false;
        let tokens = crate::tokens::models::Tokenizer::new(tokenizer_config).tokenize(payload)?;
        let mut parser = Parser::with_config(
            tokens,
            ParserConfig {
                dialect: None,
                complexity_guard: self.config.complexity_guard,
                ..Default::default()
            },
        );
        parser.recursion = Arc::clone(&self.recursion);
        Ok(parser)
    }

    /// Parses a comma-separated expression list with named arguments, preallocated.
    pub(crate) fn parse_expression_list_with_capacity(
        &mut self,
        capacity_hint: usize,
    ) -> Result<Vec<Expression>> {
        let mut expressions = Vec::with_capacity(capacity_hint);

        loop {
            let expr = if let Some(star_like) = self.parse_clickhouse_bare_star_like()? {
                star_like
            } else if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let start_pos = self.current;
                let name = self.expect_identifier_or_keyword_with_quoted()?;

                if self.match_token(TokenType::FArrow) {
                    let value = self.parse_expression()?;
                    Expression::NamedArgument(Box::new(NamedArgument {
                        name,
                        value,
                        separator: NamedArgSeparator::DArrow,
                    }))
                } else if self.match_token(TokenType::ColonEq) {
                    let value = self.parse_expression()?;
                    Expression::NamedArgument(Box::new(NamedArgument {
                        name,
                        value,
                        separator: NamedArgSeparator::ColonEq,
                    }))
                } else {
                    self.current = start_pos;
                    self.parse_expression()?
                }
            } else {
                self.parse_expression()?
            };

            let expr = if self.check(TokenType::As) {
                let as_pos = self.current;
                self.skip();
                if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                    let alias = self.expect_identifier_or_keyword_with_quoted()?;
                    let alias_expr = Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        pre_alias_comments: Vec::new(),
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }));
                    {
                        alias_expr
                    }
                } else {
                    self.current = as_pos;
                    expr
                }
            } else {
                expr
            };

            let trailing_comments = self.previous_trailing_comments().to_vec();
            let expr = if trailing_comments.is_empty() {
                expr
            } else {
                match &expr {
                    Expression::Literal(_) | Expression::Boolean(_) | Expression::Null(_) => {
                        Expression::Annotated(Box::new(Annotated {
                            this: expr,
                            trailing_comments,
                            type_assertion: None,
                            type_like: None,
                        }))
                    }
                    _ => expr,
                }
            };
            expressions.push(expr);

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(expressions)
    }

    /// Parses a comma-separated expression list with named arguments (`=>`, `:=`).
    pub(crate) fn parse_expression_list(&mut self) -> Result<Vec<Expression>> {
        self.parse_expression_list_with_capacity(0)
    }

    /// Estimate wide SELECT-list capacity without adding a scan to ordinary queries.
    pub(crate) fn estimate_select_expression_capacity(&self) -> usize {
        const MIN_TOKENS_FOR_PREALLOCATION: usize = 128;

        if self.tokens.len().saturating_sub(self.current) < MIN_TOKENS_FOR_PREALLOCATION {
            return 0;
        }

        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut commas = 0usize;
        let mut has_any_token = false;

        for token in &self.tokens[self.current..] {
            let token_type = token.token_type;
            if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                if matches!(
                    token_type,
                    TokenType::From
                        | TokenType::Where
                        | TokenType::Into
                        | TokenType::Union
                        | TokenType::Intersect
                        | TokenType::Except
                        | TokenType::Order
                        | TokenType::Limit
                        | TokenType::Qualify
                        | TokenType::Semicolon
                ) {
                    break;
                }
                if token_type == TokenType::Comma {
                    commas += 1;
                }
            }

            match token_type {
                TokenType::LParen => paren_depth += 1,
                TokenType::RParen => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        break;
                    }
                    paren_depth = paren_depth.saturating_sub(1);
                }
                TokenType::LBracket => bracket_depth += 1,
                TokenType::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::LBrace => brace_depth += 1,
                TokenType::RBrace => brace_depth = brace_depth.saturating_sub(1),
                _ => {}
            }
            has_any_token = true;
        }

        usize::from(has_any_token) + commas
    }

    /// Estimates the top-level expression count up to the next unmatched `)`, to preallocate.
    pub(crate) fn estimate_expression_list_capacity_until_rparen(&self) -> usize {
        if self.current >= self.tokens.len() || self.check(TokenType::RParen) {
            return 0;
        }

        let mut idx = self.current;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut commas = 0usize;
        let mut has_any_token = false;

        while idx < self.tokens.len() {
            let token_type = self.tokens[idx].token_type;
            match token_type {
                TokenType::LParen => paren_depth += 1,
                TokenType::RParen => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        break;
                    }
                    paren_depth = paren_depth.saturating_sub(1);
                }
                TokenType::LBracket => bracket_depth += 1,
                TokenType::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::LBrace => brace_depth += 1,
                TokenType::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenType::Comma if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    commas += 1;
                }
                _ => {}
            }
            has_any_token = true;
            idx += 1;
        }

        if has_any_token {
            commas + 1
        } else {
            0
        }
    }

    /// Parses function arguments that may be lambdas, typed ones like `a int -> a + 1` included.
    pub(crate) fn parse_function_args_with_lambda(&mut self) -> Result<Vec<Expression>> {
        let mut expressions: Vec<Expression> = Vec::new();

        loop {
            let expr = if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let saved_pos = self.current;
                let ident_token = self.advance()?;
                let quoted = ident_token.token_type == TokenType::QuotedIdentifier;

                if self.match_token(TokenType::Arrow) {
                    let body = self.parse_expression()?;
                    Expression::Lambda(Box::new(LambdaExpr {
                        parameters: vec![identifier_from_token(ident_token, quoted)],
                        body,
                        colon: false,
                        parameter_types: Vec::new(),
                    }))
                } else if !self.is_at_end()
                    && self.is_type_keyword()
                    && !self.check(TokenType::FArrow)
                    && !self.check(TokenType::ColonEq)
                {
                    let type_annotation = self.parse_data_type()?;
                    if self.match_token(TokenType::Arrow) {
                        let body = self.parse_expression()?;
                        Expression::Lambda(Box::new(LambdaExpr {
                            parameters: vec![identifier_from_token(ident_token, quoted)],
                            body,
                            colon: false,
                            parameter_types: vec![Some(type_annotation)],
                        }))
                    } else {
                        self.current = saved_pos;
                        self.parse_expression()?
                    }
                } else {
                    self.current = saved_pos;
                    self.parse_expression()?
                }
            } else {
                self.parse_expression()?
            };

            expressions.push(expr);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(expressions)
    }

    /// Parses a VALUES tuple's expressions, each with an optional Hive AS alias.
    pub(crate) fn parse_values_expression_list(&mut self) -> Result<Vec<Expression>> {
        let mut expressions: Vec<Expression> = Vec::new();

        loop {
            let expr = if self.match_token(TokenType::Default) {
                if self.is_snowflake() {
                    Expression::DefaultKeyword(DefaultKeyword)
                } else {
                    Expression::Var(Box::new(crate::expressions::statements::models::Var {
                        this: "DEFAULT".to_string(),
                    }))
                }
            } else {
                self.parse_expression()?
            };

            let trailing_comments = self.previous_trailing_comments().to_vec();
            let expr = if !trailing_comments.is_empty() {
                match &expr {
                    Expression::Literal(_) | Expression::Boolean(_) | Expression::Null(_) => {
                        Expression::Annotated(Box::new(
                            crate::expressions::statements::models::Annotated {
                                this: expr,
                                trailing_comments,
                                type_assertion: None,
                                type_like: None,
                            },
                        ))
                    }
                    _ => expr,
                }
            } else {
                expr
            };

            let expr_with_alias = if self.match_token(TokenType::As) {
                let alias = self.expect_identifier_or_keyword_with_quoted()?;
                Expression::Alias(Box::new(Alias::new(expr, alias)))
            } else {
                expr
            };

            expressions.push(expr_with_alias);

            if !self.match_token(TokenType::Comma) {
                break;
            }
            if self.check(TokenType::RParen) && !self.is_snowflake() {
                break;
            }
        }

        Ok(expressions)
    }

    /// Parse a comma-separated list of identifiers
    pub(crate) fn parse_identifier_list(&mut self) -> Result<Vec<Identifier>> {
        let mut identifiers: Vec<Identifier> = Vec::new();

        loop {
            if self.is_snowflake_reserved_keyword() {
                return Err(self.parse_error("Expected an identifier"));
            }
            let start = self.peek().span;
            let quoted = self.check(TokenType::QuotedIdentifier);
            let name = self.expect_identifier_or_safe_keyword()?;

            let trailing_comments = self.previous_trailing_comments().to_vec();
            identifiers.push(Identifier {
                name,
                quoted,
                trailing_comments,
                span: Some(start.through(self.previous().span)),
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(identifiers)
    }

    /// Parses a USING column list, keeping only the column part of qualified names.
    pub(crate) fn parse_using_column_list(&mut self) -> Result<Vec<Identifier>> {
        let mut identifiers: Vec<Identifier> = Vec::new();

        loop {
            let mut start = self.peek().span;
            let quoted = self.check(TokenType::QuotedIdentifier);
            let mut name = self.expect_identifier_or_safe_keyword()?;
            let mut final_quoted = quoted;

            while self.match_token(TokenType::Dot) {
                start = self.peek().span;
                final_quoted = self.check(TokenType::QuotedIdentifier);
                name = self.expect_identifier_or_safe_keyword()?;
            }

            let trailing_comments = self.previous_trailing_comments().to_vec();
            identifiers.push(Identifier {
                name,
                quoted: final_quoted,
                trailing_comments,
                span: Some(start.through(self.previous().span)),
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(identifiers)
    }

    /// Parses constraint columns, keeping plain names apart from prefix and order syntax.
    pub(crate) fn parse_constraint_index_columns(
        &mut self,
    ) -> Result<(Vec<Identifier>, Vec<IndexKeyPart>)> {
        let mut identifiers: Vec<Identifier> = Vec::new();
        let mut key_parts: Vec<IndexKeyPart> = Vec::new();

        loop {
            let start = self.peek().span;
            let quoted = self.check(TokenType::QuotedIdentifier);
            let name = self.expect_identifier_or_safe_keyword()?;
            let trailing_comments = self.previous_trailing_comments().to_vec();

            let identifier = Identifier {
                name,
                quoted,
                trailing_comments,
                span: Some(start.through(self.previous().span)),
            };
            let prefix_length = if self.match_token(TokenType::LParen) {
                if !self.check(TokenType::Number) {
                    return Err(self.parse_error("Expected a numeric index prefix length"));
                }
                let length = self.advance_text()?;
                self.expect_token(TokenType::RParen)?;
                Some(length)
            } else {
                None
            };
            let desc = self.match_token(TokenType::Desc);
            let asc = !desc && self.match_token(TokenType::Asc);
            key_parts.push(IndexKeyPart {
                expression: Box::new(Expression::Identifier(identifier.clone())),
                prefix_length,
                desc,
                asc,
                nulls_first: None,
                opclass: None,
            });
            identifiers.push(identifier);

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        if !identifiers.iter().any(|column| column.quoted)
            && key_parts
                .iter()
                .all(|part| part.prefix_length.is_none() && !part.desc && !part.asc)
        {
            key_parts.clear();
        }
        Ok((identifiers, key_parts))
    }

    /// Parses MySQL index key parts: `column[(length)]` or functional `(expression)`.
    pub(crate) fn parse_mysql_index_key_parts(&mut self) -> Result<Vec<IndexKeyPart>> {
        let mut key_parts: Vec<IndexKeyPart> = Vec::new();

        loop {
            let (expression, prefix_length) = if self.check(TokenType::LParen) {
                (self.parse_expression()?, None)
            } else {
                let start = self.peek().span;
                let quoted = self.check(TokenType::QuotedIdentifier);
                let name = self.expect_identifier_or_safe_keyword()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                let identifier = Identifier {
                    name,
                    quoted,
                    trailing_comments,
                    span: Some(start.through(self.previous().span)),
                };
                let prefix_length = if self.match_token(TokenType::LParen) {
                    if !self.check(TokenType::Number) {
                        return Err(self.parse_error(
                            "Expected a numeric prefix length or a parenthesized index expression",
                        ));
                    }
                    let length = self.advance_text()?;
                    self.expect_token(TokenType::RParen)?;
                    Some(length)
                } else {
                    None
                };
                (Expression::Identifier(identifier), prefix_length)
            };

            let desc = self.match_token(TokenType::Desc);
            let asc = if desc {
                false
            } else {
                self.match_token(TokenType::Asc)
            };
            key_parts.push(IndexKeyPart {
                expression: Box::new(expression),
                prefix_length,
                desc,
                asc,
                nulls_first: None,
                opclass: None,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(key_parts)
    }

    /// parse_alias_with_expr - Wraps an expression with an alias if present
    pub(crate) fn parse_alias_with_expr(
        &mut self,
        this: Option<Expression>,
    ) -> Result<Option<Expression>> {
        if this.is_none() {
            return Ok(None);
        }
        let expr = this.ok_or_else(|| {
            crate::tokens::errors::Error::internal("a value the parser checked is missing")
        })?;

        let has_as = self.match_token(TokenType::Alias) || self.match_token(TokenType::As);
        let alias_keyword = if has_as {
            let token = self.previous();
            Some(if self.source.is_some() {
                self.source_text_range(token.span.start, token.span.end)
            } else {
                token.text.to_string()
            })
        } else {
            None
        };

        if has_as && self.match_token(TokenType::LParen) {
            let mut column_aliases: Vec<Identifier> = Vec::new();
            loop {
                if let Some(col_expr) = self.parse_id_var()? {
                    if let Expression::Identifier(id) = col_expr {
                        column_aliases.push(id);
                    }
                } else {
                    break;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.match_token(TokenType::RParen);

            if !column_aliases.is_empty() {
                return Ok(Some(Expression::Alias(Box::new(Alias {
                    this: expr,
                    alias: Identifier::new(String::new()),
                    column_aliases,
                    alias_explicit_as: has_as,
                    alias_keyword: alias_keyword.clone(),
                    pre_alias_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))));
            }
        }

        if let Some(alias_expr) = self.parse_id_var()? {
            let alias_ident = match alias_expr {
                Expression::Identifier(id) => id,
                _ => return Ok(Some(expr)),
            };
            return Ok(Some(Expression::Alias(Box::new(Alias {
                this: expr,
                alias: alias_ident,
                column_aliases: Vec::new(),
                alias_explicit_as: has_as,
                alias_keyword,
                pre_alias_comments: Vec::new(),
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }))));
        }

        Ok(Some(expr))
    }

    /// Parses ALTER SESSION SET/UNSET.
    pub(crate) fn parse_alter_session(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::Set) {
            let mut expressions: Vec<Expression> = Vec::new();
            loop {
                if let Some(item) = self.parse_set_item_assignment()? {
                    expressions.push(item);
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            return Ok(Some(Expression::AlterSession(Box::new(AlterSession {
                expressions,
                unset: None,
            }))));
        }

        if self.match_text_seq(&["UNSET"]) {
            let snowflake = matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            );
            let mut expressions: Vec<Expression> = Vec::new();
            loop {
                if snowflake && self.check(TokenType::String) {
                    return Err(self.parse_error("Expected a session parameter name"));
                }
                match self.parse_id_var()? {
                    Some(var) => expressions.push(var),
                    None if snowflake => {
                        return Err(self.parse_error("Expected a session parameter name"));
                    }
                    None => {}
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            return Ok(Some(Expression::AlterSession(Box::new(AlterSession {
                expressions,
                unset: Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                }))),
            }))));
        }

        Ok(None)
    }

    /// Parses ANALYZE.
    pub(crate) fn parse_analyze(&mut self) -> Result<Option<Expression>> {
        if self.is_at_end() {
            return Ok(Some(Expression::Analyze(Box::new(Analyze {
                kind: None,
                this: None,
                options: Vec::new(),
                mode: None,
                partition: None,
                expression: None,
                properties: Vec::new(),
                columns: Vec::new(),
            }))));
        }

        let mut options: Vec<Expression> = Vec::new();
        let analyze_styles = [
            "VERBOSE",
            "SKIP_LOCKED",
            "BUFFER_USAGE_LIMIT",
            "FULL",
            "SAMPLE",
        ];
        while self.match_texts(&analyze_styles) {
            let style = self.previous_text().to_ascii_uppercase();
            if style == BUFFER_USAGE_LIMIT {
                if let Some(num) = self.parse_number()? {
                    options.push(Expression::Identifier(Identifier::new(format!(
                        "BUFFER_USAGE_LIMIT {}",
                        if let Expression::Literal(lit) = &num {
                            if let Literal::Number(n) = lit.as_ref() {
                                n.clone()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    ))));
                }
            } else {
                options.push(Expression::Identifier(Identifier::new(style)));
            }
        }

        let mut this: Option<Expression> = None;
        let mut kind: Option<String> = None;
        let mut inner_expression: Option<Expression> = None;

        if self.match_token(TokenType::Table) {
            kind = Some("TABLE".to_string());
            this = self.parse_table_parts()?;
        } else if self.match_token(TokenType::Index) {
            kind = Some("INDEX".to_string());
            this = self.parse_table_parts()?;
        } else if self.match_text_seq(&["TABLES"]) {
            kind = Some("TABLES".to_string());
            if self.match_token(TokenType::From) || self.match_token(TokenType::In) {
                let dir = self.previous_text().to_ascii_uppercase();
                kind = Some(format!("TABLES {}", dir));
                let db_name = self.expect_identifier()?;
                this = Some(Expression::Identifier(Identifier::new(db_name)));
            }
        } else if self.match_text_seq(&["DATABASE"]) {
            kind = Some("DATABASE".to_string());
            this = self.parse_table_parts()?;
        } else if self.match_text_seq(&["CLUSTER"]) {
            kind = Some("CLUSTER".to_string());
            this = self.parse_table_parts()?;
        } else if self.match_texts(&["LOCAL", "NO_WRITE_TO_BINLOG"]) {
            let opt_text = self.previous_text().to_ascii_uppercase();
            options.push(Expression::Identifier(Identifier::new(opt_text)));
            if self.match_token(TokenType::Table) {
                kind = Some("TABLE".to_string());
            }
            this = self.parse_table_parts()?;
        } else if self.match_text_seq(&["COMPUTE"]) {
            inner_expression = self.parse_analyze_statistics()?;
        } else if self.match_text_seq(&["DELETE"]) {
            inner_expression = self.parse_analyze_delete()?;
        } else if self.match_text_seq(&["VALIDATE"]) {
            inner_expression = self.parse_analyze_validate()?;
        } else if self.match_text_seq(&["LIST"]) {
            inner_expression = self.parse_analyze_list()?;
        } else if self.match_text_seq(&["DROP"]) || self.match_text_seq(&["UPDATE"]) {
            inner_expression = self.parse_analyze_histogram()?;
        } else if self.match_texts(&["ALL", "PREDICATE"]) {
            inner_expression = self.parse_analyze_columns()?;
        } else {
            this = self.parse_table_parts()?;
        }

        let columns = if this.is_some() && self.match_token(TokenType::LParen) {
            let mut cols: Vec<String> = Vec::new();
            loop {
                cols.push(self.expect_identifier_or_keyword()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            cols
        } else {
            Vec::new()
        };

        let partition = self.parse_partition()?;

        let mut mode = None;
        let mut properties: Vec<Expression> = Vec::new();

        (mode, properties) = self.parse_analyze_mode_and_properties(mode, properties)?;

        inner_expression = self.parse_analyze_trailing_kind(inner_expression)?;

        if properties.is_empty()
            && self.match_text_seq(&["PROPERTIES"])
            && self.match_token(TokenType::LParen)
        {
            loop {
                let key = if self.check(TokenType::String) {
                    self.skip();
                    let key_str = self.previous_text().to_string();
                    Expression::Literal(Box::new(Literal::String(key_str)))
                } else {
                    self.parse_id_var()?
                        .unwrap_or(Expression::Identifier(Identifier::new(String::new())))
                };

                if self.match_token(TokenType::Eq) {
                    let value = self.parse_primary()?;
                    properties.push(Expression::Property(Box::new(Property {
                        this: Box::new(key),
                        value: Some(Box::new(value)),
                    })));
                } else {
                    properties.push(Expression::Property(Box::new(Property {
                        this: Box::new(key),
                        value: None,
                    })));
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }

        Ok(Some(Expression::Analyze(Box::new(Analyze {
            kind,
            this: this.map(Box::new),
            options,
            mode,
            partition: partition.map(Box::new),
            expression: inner_expression.map(Box::new),
            properties,
            columns,
        }))))
    }

    fn parse_analyze_trailing_kind(
        &mut self,
        mut inner_expression: Option<Expression>,
    ) -> Result<Option<Expression>> {
        if inner_expression.is_none() {
            if self.match_text_seq(&["COMPUTE"]) {
                inner_expression = self.parse_analyze_statistics()?;
            } else if self.match_text_seq(&["DELETE"]) {
                inner_expression = self.parse_analyze_delete()?;
            } else if self.match_text_seq(&["VALIDATE"]) {
                inner_expression = self.parse_analyze_validate()?;
            } else if self.match_text_seq(&["LIST"]) {
                inner_expression = self.parse_analyze_list()?;
            } else if self.match_text_seq(&["DROP"]) || self.match_text_seq(&["UPDATE"]) {
                inner_expression = self.parse_analyze_histogram()?;
            } else if self.match_texts(&["ALL", "PREDICATE"]) {
                inner_expression = self.parse_analyze_columns()?;
            }
        }
        Ok(inner_expression)
    }

    fn parse_analyze_mode_and_properties(
        &mut self,
        mut mode: Option<Box<Expression>>,
        mut properties: Vec<Expression>,
    ) -> Result<(Option<Box<Expression>>, Vec<Expression>)> {
        if self.match_text_seq(&["WITH", "SYNC", "MODE"]) {
            mode = Some(Box::new(Expression::Identifier(Identifier::new(
                "WITH SYNC MODE".to_string(),
            ))));
        } else if self.match_text_seq(&["WITH", "ASYNC", "MODE"]) {
            mode = Some(Box::new(Expression::Identifier(Identifier::new(
                "WITH ASYNC MODE".to_string(),
            ))));
        } else if self.match_text_seq(&["WITH"]) && self.match_token(TokenType::LParen) {
            loop {
                let key = self.parse_id_var()?;
                if key.is_none() {
                    break;
                }

                if self.match_token(TokenType::Eq) {
                    let value = self.parse_primary()?;
                    if let Some(k) = key {
                        properties.push(Expression::Property(Box::new(Property {
                            this: Box::new(k),
                            value: Some(Box::new(value)),
                        })));
                    }
                } else if let Some(k) = key {
                    properties.push(Expression::Property(Box::new(Property {
                        this: Box::new(k),
                        value: None,
                    })));
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        Ok((mode, properties))
    }

    /// Parses ANALYZE ... COLUMNS, as an identifier.
    pub(crate) fn parse_analyze_columns(&mut self) -> Result<Option<Expression>> {
        let prev_text = self.previous_text().to_ascii_uppercase();
        if self.match_text_seq(&["COLUMNS"]) {
            return Ok(Some(Expression::Identifier(Identifier::new(format!(
                "{} COLUMNS",
                prev_text
            )))));
        }
        Ok(None)
    }

    /// Parses ANALYZE DELETE STATISTICS.
    pub(crate) fn parse_analyze_delete(&mut self) -> Result<Option<Expression>> {
        let kind = if self.match_text_seq(&["SYSTEM"]) {
            Some("SYSTEM".to_string())
        } else {
            None
        };

        if self.match_text_seq(&["STATISTICS"]) {
            return Ok(Some(Expression::AnalyzeDelete(Box::new(AnalyzeDelete {
                kind,
            }))));
        }

        Ok(None)
    }

    /// Parses ANALYZE ... HISTOGRAM ON.
    pub(crate) fn parse_analyze_histogram(&mut self) -> Result<Option<Expression>> {
        let action = self.previous_text().to_ascii_uppercase();
        let mut expressions: Vec<Expression> = Vec::new();
        let mut update_options: Option<Box<Expression>> = None;
        let mut expression: Option<Box<Expression>> = None;

        if !self.match_text_seq(&["HISTOGRAM", "ON"]) {
            return Ok(None);
        }

        loop {
            if let Some(col) = self.parse_column_reference()? {
                expressions.push(col);
            } else {
                break;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        if self.match_text_seq(&["USING", "DATA"]) {
            if self.check(TokenType::String) {
                let tok = self.advance()?;
                expression = Some(Box::new(Expression::Identifier(Identifier::new(format!(
                    "USING DATA '{}'",
                    tok.text
                )))));
            } else {
                expression = Some(Box::new(Expression::Identifier(Identifier::new(
                    "USING DATA".to_string(),
                ))));
            }
        }

        let mut mode_str: Option<String> = None;
        let mut buckets_str: Option<String> = None;

        if self.match_token(TokenType::With) {
            if self.match_texts(&["SYNC", "ASYNC"]) {
                let mode = self.previous_text().to_ascii_uppercase();
                if self.match_text_seq(&["MODE"]) {
                    mode_str = Some(format!("WITH {} MODE", mode));
                }
                if self.match_token(TokenType::With) {
                    if let Some(num) = self.parse_number()? {
                        if self.match_text_seq(&["BUCKETS"]) {
                            let num_str = if let Expression::Literal(lit) = &num {
                                if let Literal::Number(n) = lit.as_ref() {
                                    n.clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            buckets_str = Some(format!("WITH {} BUCKETS", num_str));
                        }
                    }
                }
            } else if let Some(num) = self.parse_number()? {
                if self.match_text_seq(&["BUCKETS"]) {
                    let num_str = if let Expression::Literal(lit) = &num {
                        if let Literal::Number(n) = lit.as_ref() {
                            n.clone()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    buckets_str = Some(format!("WITH {} BUCKETS", num_str));
                }
            }
        }

        match (mode_str, buckets_str) {
            (Some(m), Some(b)) => {
                expression = Some(Box::new(Expression::Identifier(Identifier::new(format!(
                    "{} {}",
                    m, b
                )))));
            }
            (Some(m), None) => {
                expression = Some(Box::new(Expression::Identifier(Identifier::new(m))));
            }
            (None, Some(b)) => {
                expression = Some(Box::new(Expression::Identifier(Identifier::new(b))));
            }
            (None, None) => {}
        }

        if self.match_texts(&["MANUAL", "AUTO"]) {
            let mode = self.previous_text().to_ascii_uppercase();
            if self.check(TokenType::Update) {
                update_options = Some(Box::new(Expression::Identifier(Identifier::new(mode))));
                self.skip();
            }
        }

        Ok(Some(Expression::AnalyzeHistogram(Box::new(
            AnalyzeHistogram {
                this: Box::new(Expression::Identifier(Identifier::new(action))),
                expressions,
                expression,
                update_options,
            },
        ))))
    }

    /// Parses ANALYZE LIST CHAINED ROWS.
    pub(crate) fn parse_analyze_list(&mut self) -> Result<Option<Expression>> {
        if self.match_text_seq(&["CHAINED", "ROWS"]) {
            let expression = self.parse_into()?.map(Box::new);
            return Ok(Some(Expression::AnalyzeListChainedRows(Box::new(
                AnalyzeListChainedRows { expression },
            ))));
        }
        Ok(None)
    }

    /// Parses ANALYZE ... STATISTICS.
    pub(crate) fn parse_analyze_statistics(&mut self) -> Result<Option<Expression>> {
        let kind = self.previous_text().to_ascii_uppercase();
        let option = if self.match_text_seq(&["DELTA"]) {
            Some(Box::new(Expression::Identifier(Identifier::new(
                "DELTA".to_string(),
            ))))
        } else {
            None
        };

        if !self.match_text_seq(&["STATISTICS"]) {
            return Ok(None);
        }

        let mut this: Option<Box<Expression>> = None;
        let mut expressions: Vec<Expression> = Vec::new();

        if self.match_text_seq(&["NOSCAN"]) {
            this = Some(Box::new(Expression::Identifier(Identifier::new(
                "NOSCAN".to_string(),
            ))));
        } else if self.match_token(TokenType::For) {
            if self.match_text_seq(&["ALL", "COLUMNS"]) {
                this = Some(Box::new(Expression::Identifier(Identifier::new(
                    "FOR ALL COLUMNS".to_string(),
                ))));
            } else if self.match_text_seq(&["COLUMNS"]) {
                this = Some(Box::new(Expression::Identifier(Identifier::new(
                    "FOR COLUMNS".to_string(),
                ))));
                loop {
                    if let Some(col) = self.parse_column_reference()? {
                        expressions.push(col);
                    } else {
                        break;
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
        } else if self.match_text_seq(&["SAMPLE"]) {
            if let Some(sample) = self.parse_number()? {
                let sample_kind = if self.match_token(TokenType::Percent) {
                    Some("PERCENT".to_string())
                } else {
                    None
                };
                expressions.push(Expression::AnalyzeSample(Box::new(AnalyzeSample {
                    kind: sample_kind.unwrap_or_default(),
                    sample: Some(Box::new(sample)),
                })));
            }
        }

        Ok(Some(Expression::AnalyzeStatistics(Box::new(
            AnalyzeStatistics {
                kind,
                option,
                this,
                expressions,
            },
        ))))
    }

    /// Parses ANALYZE VALIDATE.
    pub(crate) fn parse_analyze_validate(&mut self) -> Result<Option<Expression>> {
        let mut kind = String::new();
        let mut this: Option<Box<Expression>> = None;
        let mut expression: Option<Box<Expression>> = None;

        if self.match_text_seq(&["REF", "UPDATE"]) {
            kind = "REF".to_string();
            this = Some(Box::new(Expression::Identifier(Identifier::new(
                "UPDATE".to_string(),
            ))));
            if self.match_text_seq(&["SET", "DANGLING", "TO", "NULL"]) {
                this = Some(Box::new(Expression::Identifier(Identifier::new(
                    "UPDATE SET DANGLING TO NULL".to_string(),
                ))));
            }
        } else if self.match_text_seq(&["STRUCTURE"]) {
            kind = "STRUCTURE".to_string();
            if self.match_text_seq(&["CASCADE", "FAST"]) {
                this = Some(Box::new(Expression::Identifier(Identifier::new(
                    "CASCADE FAST".to_string(),
                ))));
            } else if self.match_text_seq(&["CASCADE", "COMPLETE"])
                && self.match_texts(&["ONLINE", "OFFLINE"])
            {
                let mode = self.previous_text().to_ascii_uppercase();
                this = Some(Box::new(Expression::Identifier(Identifier::new(format!(
                    "CASCADE COMPLETE {}",
                    mode
                )))));
                expression = self.parse_into()?.map(Box::new);
            }
        }

        if kind.is_empty() {
            return Ok(None);
        }

        Ok(Some(Expression::AnalyzeValidate(Box::new(
            AnalyzeValidate {
                kind,
                this,
                expression,
            },
        ))))
    }
}
