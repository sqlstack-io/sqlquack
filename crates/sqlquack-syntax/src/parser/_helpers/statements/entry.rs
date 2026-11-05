//! Parser statements: entry.

use crate::tokens::models::SourceSpan;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Null;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::MethodCall;
use crate::expressions::queries::models::Cte;
use crate::expressions::queries::models::From;
use crate::expressions::queries::models::GroupBy;
use crate::expressions::queries::models::Having;
use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::NamedWindow;
use crate::expressions::queries::models::Offset;
use crate::expressions::queries::models::OrderBy;
use crate::expressions::queries::models::Qualify;
use crate::expressions::queries::models::Select;
use crate::expressions::queries::models::SelectInto;
use crate::expressions::queries::models::Subquery;
use crate::expressions::queries::models::Top;
use crate::expressions::queries::models::Where;
use crate::expressions::queries::models::With;
use crate::expressions::statements::models::Annotated;
use crate::expressions::statements::models::Command;
use crate::expressions::statements::models::VerticaSelectExtensions;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::guard::main::enforce_ast::enforce_ast;
use crate::guard::main::enforce_parser_token_stats::enforce_parser_token_stats;
use crate::guard::main::enforce_parser_tokens::enforce_parser_tokens;
#[cfg(test)]
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::models::TokenGuardStats;
use crate::parser::constants::OPTION_CLAUSE_OPEN;
use crate::parser::models::Parser;
use crate::parser::models::ParserConfig;
use crate::parser::models::ParserDepthScope;
use crate::parser::models::SelectBodyHead;
use crate::parser::models::SelectLimitClauses;
use crate::tokens::errors::Error;
use crate::tokens::models::ParserToken;
use crate::tokens::models::Span;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    #[cfg(test)]
    /// A parser over tokens with the default configuration; prefer [`Parser::parse_sql`] for text.
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        let (tokens, eof_followed_by) = Self::split_terminator(tokens);
        Self {
            tokens,
            eof_followed_by,
            current: 0,
            config: ParserConfig::default(),
            source: None,
            pending_leading_comments: Vec::new(),
            guards_checked: false,
            token_guard_stats: None,
            if_expr_ruled_out: HashSet::new(),
            parsing_duckdb_json_path: false,
            faithful_calls: HashMap::new(),
            recursion: Arc::default(),
        }
    }

    /// Create a parser from a pre-tokenized token stream with a custom [`ParserConfig`].
    pub(crate) fn with_config(tokens: Vec<Token>, config: ParserConfig) -> Self {
        let (tokens, eof_followed_by) = Self::split_terminator(tokens);
        Self {
            tokens,
            eof_followed_by,
            current: 0,
            config,
            source: None,
            pending_leading_comments: Vec::new(),
            guards_checked: false,
            token_guard_stats: None,
            if_expr_ruled_out: HashSet::new(),
            parsing_duckdb_json_path: false,
            faithful_calls: HashMap::new(),
            recursion: Arc::default(),
        }
    }

    #[cfg(test)]
    /// A parser keeping the source SQL, so `Command` statements can preserve it verbatim.
    pub(crate) fn with_source(tokens: Vec<Token>, config: ParserConfig, source: String) -> Self {
        let (tokens, eof_followed_by) = Self::split_terminator(tokens);
        Self {
            tokens,
            eof_followed_by,
            current: 0,
            config,
            source: Some(Arc::from(source)),
            pending_leading_comments: Vec::new(),
            guards_checked: false,
            token_guard_stats: None,
            if_expr_ruled_out: HashSet::new(),
            parsing_duckdb_json_path: false,
            faithful_calls: HashMap::new(),
            recursion: Arc::default(),
        }
    }

    pub(crate) fn with_parser_tokens(
        tokens: Vec<ParserToken>,
        token_guard_stats: TokenGuardStats,
        config: ParserConfig,
        source: Arc<str>,
    ) -> Self {
        let (tokens, eof_followed_by) = Self::split_parser_terminator(tokens);
        Self {
            tokens,
            eof_followed_by,
            current: 0,
            config,
            source: Some(source),
            pending_leading_comments: Vec::new(),
            guards_checked: false,
            token_guard_stats: Some(token_guard_stats),
            if_expr_ruled_out: HashSet::new(),
            parsing_duckdb_json_path: false,
            faithful_calls: HashMap::new(),
            recursion: Arc::default(),
        }
    }

    /// Splits a caller-supplied end-of-input terminator off the front of the stream.
    pub(crate) fn split_terminator(
        tokens: Vec<Token>,
    ) -> (Vec<ParserToken>, Option<(TokenType, Span)>) {
        Self::split_parser_terminator(tokens.into_iter().map(ParserToken::from).collect())
    }

    pub(crate) fn split_parser_terminator(
        mut tokens: Vec<ParserToken>,
    ) -> (Vec<ParserToken>, Option<(TokenType, Span)>) {
        let Some(at) = tokens
            .iter()
            .position(|token| token.token_type == TokenType::Eof)
        else {
            return (tokens, None);
        };
        let followed_by = tokens
            .get(at + 1)
            .map(|token| (token.token_type, token.span));
        tokens.truncate(at);
        (tokens, followed_by)
    }

    #[cfg(test)]
    /// Parses one or more statements from SQL text with the default tokenizer settings.
    pub(crate) fn parse_sql(sql: &str) -> Result<Vec<Expression>> {
        crate::guard::main::enforce_input::enforce_input(sql, &ComplexityGuardOptions::default())?;
        let tokenizer = crate::tokens::models::Tokenizer::default();
        let source: Arc<str> = Arc::from(sql);
        let (tokens, token_guard_stats) = tokenizer.tokenize_for_parser(&source)?;
        let mut parser =
            Parser::with_parser_tokens(tokens, token_guard_stats, ParserConfig::default(), source);
        parser.parse()
    }

    pub(crate) fn ensure_complexity_guards(&mut self) -> Result<()> {
        if self.guards_checked {
            return Ok(());
        }

        if let Some((followed_by, span)) = self.eof_followed_by {
            return Err(Error::parse(
                format!("Unexpected token after end of input: {followed_by:?}"),
                span,
            ));
        }
        if let Some(stats) = &self.token_guard_stats {
            enforce_parser_token_stats(&self.tokens, stats, &self.config.complexity_guard)?;
        } else {
            enforce_parser_tokens(&self.tokens, &self.config.complexity_guard)?;
        }
        self.guards_checked = true;
        Ok(())
    }

    pub(crate) fn enter_parser_depth(&self, levels: usize) -> Result<ParserDepthScope> {
        if let Some(error) = self.recursion.error() {
            return Err(error);
        }
        let depth = self
            .recursion
            .depth
            .load(Ordering::Relaxed)
            .saturating_add(levels);
        if let Some(limit) = self.config.complexity_guard.max_parser_depth {
            if depth > limit {
                let span = self
                    .tokens
                    .get(self.current)
                    .or_else(|| self.tokens.last())
                    .map(|token| token.span)
                    .unwrap_or_default();
                let _ = self.recursion.failure.set((depth, limit, span));
                return Err(self.recursion.error().ok_or_else(|| {
                    crate::tokens::errors::Error::internal("depth failure was recorded")
                })?);
            }
        }
        self.recursion.depth.fetch_add(levels, Ordering::Relaxed);
        Ok(ParserDepthScope {
            recursion: Arc::clone(&self.recursion),
            levels,
        })
    }

    /// Nesting checks precede stack growth and entry into the large grammar frame.
    #[inline(always)]
    pub(crate) fn with_parser_depth<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let _scope = self.enter_parser_depth(1)?;
        let result = self.with_recursive_stack(operation);
        match self.recursion.error() {
            Some(error) => Err(error),
            None => result,
        }
    }

    #[inline(always)]
    pub(crate) fn with_recursive_stack<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        #[cfg(feature = "stacker")]
        {
            let red_zone = if cfg!(debug_assertions) {
                4 * 1024 * 1024
            } else {
                1024 * 1024
            };
            stacker::maybe_grow(red_zone, 8 * 1024 * 1024, || operation(self))
        }

        #[cfg(not(feature = "stacker"))]
        {
            operation(self)
        }
    }

    pub(crate) fn parse_method_call_expression(
        &self,
        this: Expression,
        method: Identifier,
        args: Vec<Expression>,
    ) -> Expression {
        Expression::MethodCall(Box::new(MethodCall { this, method, args }))
    }

    /// Parses every remaining statement, split on semicolons, one `Expression` each.
    pub(crate) fn parse(&mut self) -> Result<Vec<Expression>> {
        self.ensure_complexity_guards()?;
        let mut statements: Vec<Expression> = Vec::new();

        while !self.is_at_end() {
            let start = self.current;
            let mut stmt = self.parse_statement()?;
            stmt = self.spanned(start, stmt);

            if self.check(TokenType::Semicolon) {
                let semi_comments = self.current_leading_comments().to_vec();
                if !semi_comments.is_empty() {
                    stmt = Expression::Annotated(Box::new(Annotated {
                        this: stmt,
                        trailing_comments: semi_comments,
                        type_assertion: None,
                        type_like: None,
                    }));
                }
            }

            if !self.is_at_end() && !self.check(TokenType::Semicolon) {
                return Err(self.parse_error("Invalid expression / Unexpected token"));
            }

            while self.match_token(TokenType::Semicolon) {}

            statements.push(stmt);
        }

        enforce_ast(&statements, &self.config.complexity_guard)?;
        if self.applies_snowflake_context_rules() {
            self.reject_snowflake_name_wrappers(&statements)?;
        }
        Ok(statements)
    }

    /// Parses a standalone data type when the first token can start one, and `None` otherwise.
    pub(crate) fn parse_standalone_data_type_if_present(&mut self) -> Result<Option<DataType>> {
        let starts_type = !self.is_at_end()
            && (self.check(TokenType::Geometry) || self.is_identifier_or_keyword_token());
        if !starts_type {
            return Ok(None);
        }
        self.parse_standalone_data_type().map(Some)
    }

    /// Parses exactly one standalone data type, such as `DECIMAL(10, 2)`.
    pub(crate) fn parse_standalone_data_type(&mut self) -> Result<DataType> {
        self.ensure_complexity_guards()?;
        if self.tokens.is_empty() {
            return Err(self.end_of_input_error());
        }
        let data_type = self.parse_data_type()?;

        if self.check(TokenType::Semicolon) {
            self.skip();
        }

        if !self.is_at_end() {
            return Err(self.parse_error(format!(
                "Unexpected token after data type: {}",
                self.peek_text()
            )));
        }

        let expression = Expression::DataType(data_type);
        enforce_ast(
            std::slice::from_ref(&expression),
            &self.config.complexity_guard,
        )?;
        let Expression::DataType(data_type) = expression else {
            return Err(crate::tokens::errors::Error::internal(
                "the parser reached a state its checks rule out",
            ));
        };
        Ok(data_type)
    }

    /// Parses one statement, dispatching on its leading keyword; unknown ones become `Command`.
    pub(crate) fn parse_statement(&mut self) -> Result<Expression> {
        self.ensure_complexity_guards()?;
        if self.tokens.is_empty() {
            return Err(self.end_of_input_error());
        }
        let result = self.with_parser_depth(|parser| parser.parse_statement_inner());

        match result {
            Ok(expr) => Ok(expr),
            Err(err) => Err(err),
        }
    }

    pub(crate) fn parse_statement_inner(&mut self) -> Result<Expression> {
        while self.match_token(TokenType::Semicolon) {}

        if self.is_at_end() {
            return Err(self.parse_error("Unexpected end of input"));
        }

        match self.peek().token_type {
            TokenType::Select => self.parse_select(),
            TokenType::With => self.parse_with(),
            TokenType::From => self.parse_from_first_query(),
            TokenType::Hint => self.parse_statement_with_leading_hint(),
            TokenType::LParen => self.parse_parenthesized_statement_or_expression(),
            _ => self.parse_statement_slow(),
        }
    }

    pub(crate) fn parse_statement_slow(&mut self) -> Result<Expression> {
        while self.match_token(TokenType::Semicolon) {}

        if self.is_at_end() {
            return Err(self.parse_error("Unexpected end of input"));
        }

        let statement_start = self.current;

        match self.peek().token_type {
            TokenType::Hint => self.parse_statement_hint(),
            TokenType::Select => self.parse_select(),
            TokenType::With => self.parse_with(),
            TokenType::Insert => self.parse_insert(),

            TokenType::Replace => self.parse_replace(),
            TokenType::Update => self.parse_update(),
            TokenType::Delete => self.parse_delete(),

            TokenType::Create => self.parse_create(),
            TokenType::Drop => self.parse_drop(),

            TokenType::Alter => self.parse_alter(),

            TokenType::Truncate => {
                if self.check_next(TokenType::LParen) && self.applies_snowflake_context_rules() {
                    Err(self.parse_error("Expected a statement, not an expression"))
                } else if self.check_next(TokenType::LParen) {
                    self.parse_expression()
                } else {
                    self.parse_truncate()
                }
            }
            TokenType::Values => {
                if self.check_next(TokenType::LParen)
                    || self.check_next(TokenType::Number)
                    || self.check_next(TokenType::String)
                {
                    self.parse_values()
                } else {
                    self.parse_expression()
                }
            }

            TokenType::Use => self.parse_use(),
            TokenType::Cache => self.parse_cache(),
            TokenType::Uncache => self.parse_uncache(),
            TokenType::Refresh => {
                self.skip();
                self.parse_refresh()?
                    .ok_or_else(|| self.parse_error("Failed to parse REFRESH statement"))
            }
            TokenType::Load => self.parse_load_data(),

            TokenType::Grant => self.parse_grant(),
            TokenType::Revoke => self.parse_revoke(),
            TokenType::Comment => self.parse_comment(),
            TokenType::Merge => {
                self.skip();
                self.parse_merge()?
                    .ok_or_else(|| self.parse_error("Failed to parse MERGE statement"))
            }

            TokenType::Set => self.parse_set(),

            TokenType::Command => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse COMMAND statement"))
            }

            TokenType::Pragma => self.parse_pragma(),
            TokenType::Rollback => self.parse_rollback(),
            TokenType::Commit => self.parse_commit(),
            TokenType::Begin
                if matches!(
                    self.config.dialect,
                    Some(crate::tokens::models::DialectType::Snowflake)
                ) && self.is_scripting_begin() =>
            {
                self.parse_scripting_block()
            }
            TokenType::Begin => self.parse_transaction(),
            TokenType::End
                if self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL) =>
            {
                self.parse_end_transaction()
            }
            TokenType::End => {
                self.skip();
                Ok(Expression::Command(Box::new(Command {
                    this: "END".to_string(),
                })))
            }
            TokenType::Start => self.parse_start_transaction(),

            TokenType::Describe | TokenType::Desc => self.parse_describe(),

            TokenType::Show => self.parse_show(),
            TokenType::Copy => self.parse_copy(),
            TokenType::Put => self.parse_put(),

            TokenType::Kill => self.parse_kill(),
            TokenType::Prepare => self.parse_prepare(),
            TokenType::Execute => {
                if self
                    .peek_nth(1)
                    .map(|t| t.text.eq_ignore_ascii_case("IMMEDIATE"))
                    == Some(true)
                {
                    self.skip();
                    self.parse_command()?.ok_or_else(|| {
                        self.parse_error("Failed to parse EXECUTE IMMEDIATE statement")
                    })
                } else {
                    self.parse_execute()
                }
            }
            TokenType::Declare => {
                if matches!(
                    self.config.dialect,
                    Some(crate::tokens::models::DialectType::Snowflake)
                ) && self.check_snowflake_scripting_block()
                {
                    self.parse_raw_scripting_block()
                } else {
                    self.skip();
                    self.parse_declare()?
                        .ok_or_else(|| self.parse_error("Failed to parse DECLARE statement"))
                }
            }
            _ => self.parse_statement_by_keyword(statement_start),
        }
    }

    fn parse_statement_by_keyword(&mut self, statement_start: usize) -> Result<Expression> {
        match self.peek().token_type {
            TokenType::Get
                if self.check_next(TokenType::DAt) || !self.check_next(TokenType::LParen) =>
            {
                self.parse_get_command()
            }
            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("RM")
                    || self.peek_text().eq_ignore_ascii_case("REMOVE") =>
            {
                self.parse_rm_command()
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("CALL") => self.parse_call(),
            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("UNSET")
                    && matches!(
                        self.config.dialect,
                        Some(crate::tokens::models::DialectType::Snowflake)
                    ) =>
            {
                self.parse_unset()
            }
            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("OPEN")
                    && matches!(
                        self.config.dialect,
                        Some(crate::tokens::models::DialectType::Snowflake)
                    ) =>
            {
                Err(self
                    .parse_error("OPEN is a Snowflake Scripting statement, valid only in a block"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("OPEN") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse OPEN statement"))
            }

            TokenType::Var if self.peek_text().eq_ignore_ascii_case("EXPLAIN") => {
                self.parse_describe()
            }
            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("LOCK")
                    || self.peek_text().eq_ignore_ascii_case("UNLOCK") =>
            {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse LOCK/UNLOCK statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("ANALYZE") => {
                self.skip();
                self.parse_analyze()?
                    .ok_or_else(|| self.parse_error("Failed to parse ANALYZE statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("PRINT") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse PRINT statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("WAITFOR") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse WAITFOR statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("BULK") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse BULK INSERT statement"))
            }

            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("OPTIMIZE")
                    && self.config.dialect.is_none() =>
            {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse OPTIMIZE statement"))
            }

            TokenType::Var if self.peek_text().eq_ignore_ascii_case("ATTACH") => {
                self.skip();
                self.parse_attach_detach(true)
            }
            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("UNDROP")
                    && matches!(
                        self.config.dialect,
                        Some(crate::tokens::models::DialectType::Snowflake)
                    ) =>
            {
                self.parse_undrop()
            }

            TokenType::Var if self.peek_text().eq_ignore_ascii_case("DETACH") => {
                self.skip();
                self.parse_attach_detach(false)
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("RESTORE") => {
                self.skip();
                self.parse_as_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse RESTORE statement"))
            }

            TokenType::Var if self.peek_text().eq_ignore_ascii_case("VACUUM") => {
                self.skip();
                self.parse_as_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse VACUUM statement"))
            }
            TokenType::Var
                if self.peek_text().eq_ignore_ascii_case("LIST")
                    || self.peek_text().eq_ignore_ascii_case("LS") =>
            {
                self.skip();
                self.parse_as_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse LIST/LS statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("INSTALL") => {
                self.skip();
                self.parse_install(false)
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("FORCE") => {
                self.skip();
                self.parse_force_statement()
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("SUMMARIZE") => {
                self.skip();
                self.parse_summarize_statement()
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("RESET") => {
                if self.check_text_seq(&["RESET", "SESSION", "AUTHORIZATION"]) {
                    self.fallback_to_command(statement_start)
                } else {
                    self.skip();
                    self.parse_as_command()?
                        .ok_or_else(|| self.parse_error("Failed to parse RESET statement"))
                }
            }
            TokenType::Pivot => {
                self.skip();
                self.parse_simplified_pivot(false)?
                    .ok_or_else(|| self.parse_error("Failed to parse PIVOT statement"))
            }
            TokenType::Unpivot => {
                self.skip();
                self.parse_simplified_pivot(true)?
                    .ok_or_else(|| self.parse_error("Failed to parse UNPIVOT statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("PIVOT_WIDER") => {
                self.skip();
                self.parse_simplified_pivot(false)?
                    .ok_or_else(|| self.parse_error("Failed to parse PIVOT_WIDER statement"))
            }
            TokenType::For => {
                self.skip();
                self.parse_for_in()
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("LOOP") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse LOOP statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("REPEAT") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse REPEAT statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("WHILE") => {
                self.skip();
                self.parse_command()?
                    .ok_or_else(|| self.parse_error("Failed to parse WHILE statement"))
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("UNLOAD") => {
                self.parse_unload()
            }
            TokenType::Using => self.parse_using_external_function(),
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("EXPORT") => {
                self.parse_export_data()
            }
            TokenType::Var if self.peek_text().eq_ignore_ascii_case("DEALLOCATE") => {
                self.parse_deallocate_prepare()
            }
            TokenType::From => self.parse_from_first_query(),
            TokenType::LParen => self.parse_statement_parenthesized(),
            _ => self.parse_statement_other(),
        }
    }

    fn parse_statement_hint(&mut self) -> Result<Expression> {
        let hint_token = self.advance()?;
        let hint_text = hint_token.text.to_string();
        let comment = format!("/* + {} */", hint_text.trim());

        let mut stmt = self.parse_statement()?;

        match &mut stmt {
            Expression::Select(select) => {
                select.leading_comments.insert(0, comment);
            }
            Expression::Insert(insert) => {
                insert.leading_comments.insert(0, comment);
            }
            Expression::Update(update) => {
                update.leading_comments.insert(0, comment);
            }
            Expression::Delete(delete) => {
                delete.leading_comments.insert(0, comment);
            }
            Expression::CreateTable(ct) => {
                ct.leading_comments.insert(0, comment);
            }
            _ => {}
        }
        Ok(stmt)
    }

    fn parse_undrop(&mut self) -> Result<Expression> {
        self.skip();
        let kind = if self.match_text_seq(&["DYNAMIC", "TABLE"]) {
            "DYNAMIC TABLE".to_string()
        } else if self.match_text_seq(&["EXTERNAL", "VOLUME"]) {
            "EXTERNAL VOLUME".to_string()
        } else if self.match_text_seq(&["ICEBERG", "TABLE"]) {
            "ICEBERG TABLE".to_string()
        } else if self.match_token(TokenType::Table) {
            "TABLE".to_string()
        } else if self.match_token(TokenType::Schema) {
            "SCHEMA".to_string()
        } else if self.match_token(TokenType::Database) {
            "DATABASE".to_string()
        } else if self.match_texts(crate::parser::constants::SNOWFLAKE_UNDROP_KINDS) {
            self.previous_text().to_ascii_uppercase()
        } else {
            return Err(self.parse_error(
                "Expected TABLE, SCHEMA, DATABASE, or Snowflake object kind after UNDROP",
            ));
        };
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name_ref()?;
        let rename_to = if self.match_keywords(&[TokenType::Rename, TokenType::To]) {
            Some(self.parse_object_name_ref()?)
        } else {
            None
        };
        Ok(Expression::Undrop(Box::new(
            crate::expressions::tables::models::Undrop {
                kind,
                name,
                if_exists,
                rename_to,
            },
        )))
    }

    fn parse_statement_parenthesized(&mut self) -> Result<Expression> {
        let next_is_explain = self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1].token_type == TokenType::Var
            && self.tokens[self.current + 1]
                .text
                .eq_ignore_ascii_case("EXPLAIN");
        if self.check_next(TokenType::Select)
            || self.check_next(TokenType::With)
            || self.check_next(TokenType::Pivot)
            || self.check_next(TokenType::Unpivot)
            || self.check_next(TokenType::From)
            || next_is_explain
        {
            self.skip();
            let inner = { self.parse_statement()? };
            self.expect_token(TokenType::RParen)?;
            let subquery = Expression::Subquery(Box::new(Subquery {
                this: inner,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: false,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }));
            let result = self.parse_set_operation(subquery)?;
            self.parse_query_modifiers(result)
        } else if self.check_next(TokenType::LParen) {
            self.skip();
            let inner = self.parse_statement()?;
            let result = self.parse_set_operation(inner)?;
            self.expect_token(TokenType::RParen)?;
            let subquery = Expression::Subquery(Box::new(Subquery {
                this: result,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: false,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }));
            let result = self.parse_set_operation(subquery)?;
            let pre_alias_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::As) || self.match_token(TokenType::Alias) {
                let token = self.previous();
                let alias_keyword = if self.source.is_some() {
                    self.source_text_range(token.span.start, token.span.end)
                } else {
                    token.text.to_string()
                };
                let alias = self.expect_identifier_or_keyword_with_quoted()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Ok(Expression::Alias(Box::new(Alias {
                    this: result,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: true,
                    alias_keyword: Some(alias_keyword),
                    pre_alias_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            } else {
                self.parse_query_modifiers(result)
            }
        } else {
            let expr = self.parse_expression()?;
            let pre_alias_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::As) || self.match_token(TokenType::Alias) {
                let token = self.previous();
                let alias_keyword = if self.source.is_some() {
                    self.source_text_range(token.span.start, token.span.end)
                } else {
                    token.text.to_string()
                };
                if self.match_token(TokenType::LParen) {
                    let mut column_aliases: Vec<Identifier> = Vec::new();
                    loop {
                        let col_alias = self.expect_identifier_or_keyword_with_quoted()?;
                        column_aliases.push(col_alias);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias: Identifier::empty(),
                        column_aliases,
                        alias_explicit_as: true,
                        alias_keyword: Some(alias_keyword.clone()),
                        pre_alias_comments,
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })))
                } else {
                    let alias = self.expect_identifier_or_keyword_with_quoted()?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias,
                        column_aliases: Vec::new(),
                        alias_explicit_as: true,
                        alias_keyword: Some(alias_keyword),
                        pre_alias_comments,
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })))
                }
            } else {
                Ok(expr)
            }
        }
    }

    fn parse_statement_other(&mut self) -> Result<Expression> {
        if self.applies_snowflake_context_rules() {
            return Err(self.parse_error("Expected a statement, not an expression"));
        }
        let leading_comments = self.current_leading_comments().to_vec();
        let expr = self.parse_expression()?;
        let pre_alias_comments = self.previous_trailing_comments().to_vec();
        if self.match_token(TokenType::As) || self.match_token(TokenType::Alias) {
            let token = self.previous();
            let alias_keyword = if self.source.is_some() {
                self.source_text_range(token.span.start, token.span.end)
            } else {
                token.text.to_string()
            };
            let as_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::LParen) {
                let mut column_aliases: Vec<Identifier> = Vec::new();
                loop {
                    let col_alias = self.expect_identifier_or_keyword_with_quoted()?;
                    column_aliases.push(col_alias);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                let mut trailing_comments = as_comments;
                trailing_comments.extend_from_slice(self.previous_trailing_comments());
                Ok(Expression::Alias(Box::new(Alias {
                    this: expr,
                    alias: Identifier::empty(),
                    column_aliases,
                    alias_explicit_as: true,
                    alias_keyword: Some(alias_keyword.clone()),
                    pre_alias_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            } else {
                let alias = self.expect_identifier_or_keyword_with_quoted()?;
                let mut trailing_comments = self.previous_trailing_comments().to_vec();
                trailing_comments.extend(leading_comments.iter().cloned());
                Ok(Expression::Alias(Box::new(Alias {
                    this: expr,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: true,
                    alias_keyword: Some(alias_keyword),
                    pre_alias_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            }
        } else if self.check(TokenType::Var) && !self.check_keyword()
            || self.is_command_keyword_as_alias()
        {
            let alias_text = self.advance_text()?;
            let trailing_comments = self.previous_trailing_comments().to_vec();
            Ok(Expression::Alias(Box::new(Alias {
                this: expr,
                alias: Identifier::new(alias_text),
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                pre_alias_comments,
                trailing_comments,
                inferred_type: None,
                span: SourceSpan::default(),
            })))
        } else if !pre_alias_comments.is_empty() {
            match &expr {
                Expression::Literal(_) | Expression::Boolean(_) | Expression::Null(_) => {
                    Ok(Expression::Annotated(Box::new(
                        crate::expressions::statements::models::Annotated {
                            this: expr,
                            trailing_comments: pre_alias_comments,
                            type_assertion: None,
                            type_like: None,
                        },
                    )))
                }
                _ => Ok(expr),
            }
        } else if !leading_comments.is_empty() {
            Ok(Expression::Annotated(Box::new(
                crate::expressions::statements::models::Annotated {
                    this: expr,
                    trailing_comments: leading_comments,
                    type_assertion: None,
                    type_like: None,
                },
            )))
        } else {
            Ok(expr)
        }
    }

    pub(crate) fn parse_statement_with_leading_hint(&mut self) -> Result<Expression> {
        let hint_token = self.advance()?;
        let hint_text = hint_token.text.to_string();
        let comment = format!("/* + {} */", hint_text.trim());

        let mut stmt = self.parse_statement()?;

        match &mut stmt {
            Expression::Select(select) => {
                select.leading_comments.insert(0, comment);
            }
            Expression::Insert(insert) => {
                insert.leading_comments.insert(0, comment);
            }
            Expression::Update(update) => {
                update.leading_comments.insert(0, comment);
            }
            Expression::Delete(delete) => {
                delete.leading_comments.insert(0, comment);
            }
            Expression::CreateTable(ct) => {
                ct.leading_comments.insert(0, comment);
            }
            _ => {}
        }

        Ok(stmt)
    }

    pub(crate) fn parse_parenthesized_statement_or_expression(&mut self) -> Result<Expression> {
        let next_is_explain = self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1].token_type == TokenType::Var
            && self.tokens[self.current + 1]
                .text
                .eq_ignore_ascii_case("EXPLAIN");

        if self.check_next(TokenType::Select)
            || self.check_next(TokenType::With)
            || self.check_next(TokenType::Pivot)
            || self.check_next(TokenType::Unpivot)
            || self.check_next(TokenType::From)
            || next_is_explain
        {
            self.skip();
            let inner = self.parse_statement()?;
            self.expect_token(TokenType::RParen)?;
            let subquery = Expression::Subquery(Box::new(Subquery {
                this: inner,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: false,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }));
            let result = self.parse_set_operation(subquery)?;
            self.parse_query_modifiers(result)
        } else if self.check_next(TokenType::LParen) {
            self.skip();
            let inner = self.parse_statement()?;
            let result = self.parse_set_operation(inner)?;
            self.expect_token(TokenType::RParen)?;
            let subquery = Expression::Subquery(Box::new(Subquery {
                this: result,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: false,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }));
            let result = self.parse_set_operation(subquery)?;
            let pre_alias_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::As) {
                let alias = self.expect_identifier_or_keyword_with_quoted()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Ok(Expression::Alias(Box::new(Alias {
                    this: result,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })))
            } else {
                self.parse_query_modifiers(result)
            }
        } else {
            let expr = self.parse_expression()?;
            let pre_alias_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::As) {
                if self.match_token(TokenType::LParen) {
                    let mut column_aliases: Vec<Identifier> = Vec::new();
                    loop {
                        let col_alias = self.expect_identifier_or_keyword_with_quoted()?;
                        column_aliases.push(col_alias);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias: Identifier::empty(),
                        column_aliases,
                        alias_explicit_as: true,
                        alias_keyword: Some("AS".to_string()),
                        pre_alias_comments,
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })))
                } else {
                    let alias = self.expect_identifier_or_keyword_with_quoted()?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Ok(Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        pre_alias_comments,
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    })))
                }
            } else {
                Ok(expr)
            }
        }
    }

    /// Parse a SELECT statement
    pub(crate) fn parse_select(&mut self) -> Result<Expression> {
        let result = self.parse_select_body()?;
        self.parse_set_operation(result)
    }

    #[inline(never)]
    pub(crate) fn parse_select_body_head(&mut self) -> Result<Box<SelectBodyHead>> {
        let select_token = self.expect_token(TokenType::Select)?;
        let leading_comments = select_token.comments;
        let post_select_comments = select_token.trailing_comments;

        let hint = if self.check(TokenType::Hint) {
            Some(self.parse_hint()?)
        } else {
            None
        };

        let top = self.parse_select_top()?;

        let is_distinct_token = self.match_token(TokenType::Distinct);
        let all = !is_distinct_token && self.check(TokenType::All);
        let all = all && self.is_snowflake() && !self.check_next(TokenType::Dot);
        let (distinct, distinct_on) = if is_distinct_token {
            if self.match_token(TokenType::On) {
                self.expect_token(TokenType::LParen)?;
                let exprs = self.parse_expression_list()?;
                self.expect_token(TokenType::RParen)?;
                (true, Some(exprs))
            } else {
                (true, None)
            }
        } else if self.check_identifier("DISTINCTROW") {
            self.skip();
            (true, None)
        } else {
            if self.check(TokenType::All) && !self.check_next(TokenType::Dot) {
                self.skip();
            }
            (false, None)
        };

        let top = self.parse_select_trailing_top(top)?;

        let operation_modifiers: Vec<String> = Vec::new();

        let kind = if self.match_token(TokenType::As) {
            if self.match_identifier("STRUCT") {
                Some("STRUCT".to_string())
            } else if self.match_identifier("VALUE") {
                Some("VALUE".to_string())
            } else {
                self.current -= 1;
                None
            }
        } else {
            None
        };

        let expressions = self.parse_select_expressions()?;
        if expressions.is_empty()
            && matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            )
        {
            return Err(self.parse_error("Snowflake requires at least one select item"));
        }

        let exclude = { None };

        let into = if self.match_text_seq(&["BULK", "COLLECT", "INTO"]) {
            let mut target_expressions = vec![self.parse_expression()?];
            while self.match_token(TokenType::Comma) {
                target_expressions.push(self.parse_expression()?);
            }
            if target_expressions.len() == 1 {
                Some(SelectInto {
                    this: target_expressions.remove(0),
                    temporary: false,
                    unlogged: false,
                    bulk_collect: true,
                    expressions: Vec::new(),
                })
            } else {
                Some(SelectInto {
                    this: Expression::Null(Null),
                    temporary: false,
                    unlogged: false,
                    bulk_collect: true,
                    expressions: target_expressions,
                })
            }
        } else if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && self.check(TokenType::Into)
            && self.check_next(TokenType::Colon)
        {
            self.skip();
            let mut target_expressions = vec![self.parse_primary()?];
            while self.match_token(TokenType::Comma) {
                target_expressions.push(self.parse_primary()?);
            }
            Some(match target_expressions.len() {
                1 => SelectInto {
                    this: target_expressions.remove(0),
                    temporary: false,
                    unlogged: false,
                    bulk_collect: false,
                    expressions: Vec::new(),
                },
                _ => SelectInto {
                    this: Expression::Null(Null),
                    temporary: false,
                    unlogged: false,
                    bulk_collect: false,
                    expressions: target_expressions,
                },
            })
        } else if self.match_token(TokenType::Into) {
            let temporary = self.match_token(TokenType::Temporary) || self.match_identifier("TEMP");
            let unlogged = !temporary && self.match_identifier("UNLOGGED");
            let table_name = self.parse_table_ref()?;
            if self.match_token(TokenType::Comma) {
                let mut target_expressions = vec![Expression::Table(Box::new(table_name))];
                target_expressions.push(self.parse_expression()?);
                while self.match_token(TokenType::Comma) {
                    target_expressions.push(self.parse_expression()?);
                }
                Some(SelectInto {
                    this: Expression::Null(Null),
                    temporary,
                    unlogged,
                    bulk_collect: false,
                    expressions: target_expressions,
                })
            } else {
                Some(SelectInto {
                    this: Expression::Table(Box::new(table_name)),
                    temporary,
                    unlogged,
                    bulk_collect: false,
                    expressions: Vec::new(),
                })
            }
        } else {
            None
        };

        Ok(Box::new(SelectBodyHead {
            leading_comments,
            post_select_comments,
            hint,
            top,
            distinct,
            all,
            distinct_on,
            kind,
            operation_modifiers,
            expressions,
            exclude,
            into,
        }))
    }

    /// Whether TOP starts a row count: in Snowflake only before an integer or a parenthesis.
    fn top_takes_count(&self) -> bool {
        !matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) || self.check_next(TokenType::LParen)
            || self.peek_nth(1).is_some_and(|token| {
                token.token_type == TokenType::Number
                    && token.text.bytes().all(|byte| byte.is_ascii_digit())
            })
    }

    fn parse_select_top(&mut self) -> Result<Option<Top>> {
        Ok(
            if self.check(TokenType::Top)
                && !self.check_next(TokenType::Dot)
                && self.top_takes_count()
                && self.match_token(TokenType::Top)
            {
                let (amount, parenthesized) = if self.match_token(TokenType::LParen) {
                    let expr = if self.check(TokenType::Select) || self.check(TokenType::With) {
                        let stmt = self.parse_statement()?;
                        Expression::Subquery(Box::new(Subquery {
                            this: stmt,
                            alias: None,
                            column_aliases: Vec::new(),
                            alias_explicit_as: false,
                            alias_keyword: None,
                            order_by: None,
                            limit: None,
                            offset: None,
                            distribute_by: None,
                            sort_by: None,
                            cluster_by: None,
                            lateral: false,
                            modifiers_inside: false,
                            trailing_comments: Vec::new(),
                            inferred_type: None,
                            span: SourceSpan::default(),
                        }))
                    } else {
                        self.parse_expression()?
                    };
                    self.expect_token(TokenType::RParen)?;
                    (expr, true)
                } else {
                    (self.parse_primary()?, false)
                };
                let percent = self.match_token(TokenType::Percent);
                let with_ties = self.match_keywords(&[TokenType::With, TokenType::Ties]);
                Some(Top {
                    this: amount,
                    percent,
                    with_ties,
                    parenthesized,
                })
            } else {
                None
            },
        )
    }

    fn parse_select_trailing_top(&mut self, top: Option<Top>) -> Result<Option<Top>> {
        Ok(
            if top.is_none()
                && self.check(TokenType::Top)
                && !self.check_next(TokenType::Dot)
                && self.top_takes_count()
                && self.match_token(TokenType::Top)
            {
                let (amount, parenthesized) = if self.match_token(TokenType::LParen) {
                    let expr = if self.check(TokenType::Select) || self.check(TokenType::With) {
                        let stmt = self.parse_statement()?;
                        Expression::Subquery(Box::new(Subquery {
                            this: stmt,
                            alias: None,
                            column_aliases: Vec::new(),
                            alias_explicit_as: false,
                            alias_keyword: None,
                            order_by: None,
                            limit: None,
                            offset: None,
                            distribute_by: None,
                            sort_by: None,
                            cluster_by: None,
                            lateral: false,
                            modifiers_inside: false,
                            trailing_comments: Vec::new(),
                            inferred_type: None,
                            span: SourceSpan::default(),
                        }))
                    } else {
                        self.parse_expression()?
                    };
                    self.expect_token(TokenType::RParen)?;
                    (expr, true)
                } else {
                    (self.parse_primary()?, false)
                };
                let percent = self.match_token(TokenType::Percent);
                let with_ties = self.match_keywords(&[TokenType::With, TokenType::Ties]);
                Some(Top {
                    this: amount,
                    percent,
                    with_ties,
                    parenthesized,
                })
            } else {
                top
            },
        )
    }

    /// Parses a SELECT body without trailing set operations, which `parse_set_operation` chains.
    pub(crate) fn parse_select_body(&mut self) -> Result<Expression> {
        let head = self.parse_select_body_head()?;

        let (from, joins) = self.parse_select_from_and_joins()?;

        let lateral_views = self.parse_lateral_views()?;

        let prewhere = if self.match_token(TokenType::Prewhere) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let mut where_clause = if self.match_token(TokenType::Where) {
            Some(Where {
                this: self.parse_expression()?,
            })
        } else {
            None
        };

        let timeseries = self.parse_vertica_timeseries()?;

        let connect = self.parse_connect()?;

        let group_by = self.parse_select_group_by()?;

        let having = self.parse_select_having()?;

        let match_clause = self.parse_vertica_match()?;

        let mut qualify = if self.match_token(TokenType::Qualify) {
            Some(Qualify {
                this: self.parse_expression()?,
            })
        } else {
            None
        };

        let windows = self.parse_select_windows()?;

        let qualify_after_window = if qualify.is_none() && self.match_token(TokenType::Qualify) {
            qualify = Some(Qualify {
                this: self.parse_expression()?,
            });
            true
        } else {
            false
        };

        let distribute_by = if self.match_keywords(&[TokenType::Distribute, TokenType::By]) {
            Some(self.parse_distribute_by()?)
        } else {
            None
        };

        let cluster_by = if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            Some(self.parse_cluster_by()?)
        } else {
            None
        };

        let sort_by = if self.match_keywords(&[TokenType::Sort, TokenType::By]) {
            Some(self.parse_sort_by()?)
        } else {
            None
        };

        let order_by = self.parse_select_order_by()?;

        let limits;
        (limits, where_clause) = self.parse_select_limit_offset(where_clause)?;
        let SelectLimitClauses {
            limit,
            offset,
            limit_over,
            limit_by,
        } = limits;

        let hana_total_rowcount = false;

        let fetch = if self.match_token(TokenType::Fetch) {
            Some(self.parse_fetch()?)
        } else {
            None
        };
        if limit.is_some() && fetch.is_some() {
            return Err(self.parse_error("LIMIT and FETCH cannot be combined in the same query"));
        }

        let sample = self.parse_sample_clause()?;

        let (locks, for_xml, for_json) = self.parse_locks_and_for_xml()?;
        let (result_serialization, query_collation, query_hints) =
            self.parse_hana_select_options()?;

        let option = { self.parse_select_option_clause() };
        let (settings, format) = self.parse_clickhouse_select_settings_and_format()?;

        let SelectBodyHead {
            leading_comments,
            post_select_comments,
            hint,
            top,
            distinct,
            all,
            distinct_on,
            kind,
            operation_modifiers,
            expressions,
            exclude,
            into,
        } = *head;

        let select = Select {
            vertica: if timeseries.is_some() || match_clause.is_some() || limit_over.is_some() {
                Some(Box::new(VerticaSelectExtensions {
                    timeseries,
                    match_clause,
                    limit_over,
                }))
            } else {
                None
            },
            expressions,
            from,
            joins,
            lateral_views,
            prewhere,
            where_clause,
            group_by,
            having,
            qualify,
            order_by,
            distribute_by,
            cluster_by,
            sort_by,
            limit,
            offset,
            limit_by,
            fetch,
            distinct,
            all,
            distinct_on,
            top,
            with: None,
            sample,
            settings,
            format,
            windows,
            hint,
            connect,
            into,
            for_xml,
            for_json,
            source_dialect: None,
            result_serialization,
            query_collation,
            query_hints,
            total_rowcount: hana_total_rowcount,
            locks,
            leading_comments,
            post_select_comments,
            kind,
            operation_modifiers,
            qualify_after_window,
            option,
            exclude,
            span: SourceSpan::default(),
        };

        Ok(Expression::Select(Box::new(select)))
    }

    fn parse_select_windows(&mut self) -> Result<Option<Vec<NamedWindow>>> {
        Ok(
            if self.check(TokenType::Window) && {
                let next_pos = self.current + 1;
                next_pos < self.tokens.len()
                    && (self.tokens[next_pos].token_type == TokenType::Var
                        || self.tokens[next_pos].token_type == TokenType::Identifier)
            } {
                self.skip();
                Some(self.parse_named_windows()?)
            } else {
                None
            },
        )
    }

    fn parse_select_having(&mut self) -> Result<Option<Having>> {
        Ok(if self.check(TokenType::Having) {
            let having_comments = self.current_leading_comments().to_vec();
            self.skip();
            Some(Having {
                this: self.parse_expression()?,
                comments: having_comments,
            })
        } else {
            None
        })
    }

    fn parse_select_group_by(&mut self) -> Result<Option<GroupBy>> {
        Ok(if self.check(TokenType::Group) {
            let group_comments = self.current_leading_comments().to_vec();
            if self.match_keywords(&[TokenType::Group, TokenType::By]) {
                let mut gb = self.parse_group_by()?;
                gb.comments = group_comments;
                Some(gb)
            } else {
                None
            }
        } else {
            None
        })
    }

    fn parse_select_order_by(&mut self) -> Result<Option<OrderBy>> {
        Ok(if self.check(TokenType::Order) {
            let order_comments = self.current_leading_comments().to_vec();
            if self.match_keywords(&[TokenType::Order, TokenType::Siblings, TokenType::By]) {
                let mut ob = self.parse_order_by_with_siblings(true)?;
                ob.comments = order_comments;
                Some(ob)
            } else if self.match_keywords(&[TokenType::Order, TokenType::By]) {
                let mut ob = self.parse_order_by()?;
                ob.comments = order_comments;
                Some(ob)
            } else {
                None
            }
        } else {
            None
        })
    }

    fn parse_select_from_and_joins(&mut self) -> Result<(Option<From>, Vec<Join>)> {
        let from = if self.match_token(TokenType::From) {
            Some(self.parse_from()?)
        } else {
            None
        };

        let mut joins = self.parse_joins()?;

        while self.check(TokenType::Pivot) || self.check(TokenType::Unpivot) {
            if !joins.is_empty() {
                let last_idx = joins.len() - 1;
                if self.match_token(TokenType::Pivot) {
                    let pivot = self
                        .parse_pivot(Expression::Null(crate::expressions::names::models::Null))?;
                    joins[last_idx].pivots.push(pivot);
                } else if self.match_token(TokenType::Unpivot) {
                    let unpivot = self
                        .parse_unpivot(Expression::Null(crate::expressions::names::models::Null))?;
                    joins[last_idx].pivots.push(unpivot);
                }
            } else {
                break;
            }
        }
        Ok((from, joins))
    }

    fn parse_select_limit_offset(
        &mut self,
        mut where_clause: Option<Where>,
    ) -> Result<(SelectLimitClauses, Option<Where>)> {
        let pre_limit_comments = if self.check(TokenType::Limit) {
            let mut comments = self.previous_trailing_comments().to_vec();
            comments.extend_from_slice(self.current_leading_comments());
            comments
        } else {
            Vec::new()
        };
        let (limit, offset) = if self.match_token(TokenType::Limit) {
            if !pre_limit_comments.is_empty() {
                if let Some(ref mut w) = where_clause {
                    Self::clear_rightmost_trailing_comments(&mut w.this);
                }
            }
            let (first_expr, has_percent) = self.parse_limit_value()?;
            if self.match_token(TokenType::Comma) {
                let second_expr = self.parse_expression()?;
                (
                    Some(Limit {
                        this: second_expr,
                        percent: false,
                        comments: pre_limit_comments.clone(),
                    }),
                    Some(Offset {
                        this: first_expr,
                        rows: None,
                    }),
                )
            } else {
                (
                    Some(Limit {
                        this: first_expr,
                        percent: has_percent,
                        comments: pre_limit_comments,
                    }),
                    None,
                )
            }
        } else {
            (None, None)
        };

        let limit_over = { None };

        if limit.is_some() {
            let _ = self.match_keywords(&[TokenType::With, TokenType::Ties]);
        }

        let (limit, offset) = if offset.is_none() && self.match_token(TokenType::Offset) {
            let expr = self.parse_expression()?;
            let rows = if self.match_token(TokenType::Row) || self.match_token(TokenType::Rows) {
                Some(true)
            } else {
                None
            };
            let offset = Some(Offset { this: expr, rows });

            let limit = if limit.is_none() && self.match_token(TokenType::Limit) {
                let (this, percent) = self.parse_limit_value()?;
                Some(Limit {
                    this,
                    percent,
                    comments: Vec::new(),
                })
            } else {
                limit
            };

            (limit, offset)
        } else {
            (limit, offset)
        };

        let limit_by = { None };

        let (limit, offset) = if limit_by.is_some() && self.match_token(TokenType::Limit) {
            let first_expr = self.parse_expression()?;
            if self.match_token(TokenType::Comma) {
                let count_expr = self.parse_expression()?;
                (
                    Some(Limit {
                        this: count_expr,
                        percent: false,
                        comments: Vec::new(),
                    }),
                    Some(Offset {
                        this: first_expr,
                        rows: None,
                    }),
                )
            } else {
                (
                    Some(Limit {
                        this: first_expr,
                        percent: false,
                        comments: Vec::new(),
                    }),
                    offset,
                )
            }
        } else {
            (limit, offset)
        };
        Ok((
            SelectLimitClauses {
                limit,
                offset,
                limit_over,
                limit_by,
            },
            where_clause,
        ))
    }

    pub(crate) fn parse_select_option_clause(&mut self) -> Option<String> {
        if !(self.check_identifier("OPTION") && self.check_next(TokenType::LParen)) {
            return None;
        }

        self.skip();
        self.skip();
        let mut content = String::from(OPTION_CLAUSE_OPEN);
        let mut depth = 1;
        while !self.is_at_end() && depth > 0 {
            let Ok(tok) = self.advance() else { break };
            if tok.token_type == TokenType::LParen {
                depth += 1;
            } else if tok.token_type == TokenType::RParen {
                depth -= 1;
            }
            if depth > 0 {
                if tok.token_type == TokenType::String {
                    if content.len() > OPTION_CLAUSE_OPEN.len()
                        && !content.ends_with('(')
                        && !content.ends_with(' ')
                    {
                        content.push(' ');
                    }
                    content.push('\'');
                    content.push_str(&tok.text.replace('\'', "''"));
                    content.push('\'');
                } else if tok.token_type == TokenType::Eq {
                    content.push_str(" = ");
                } else if tok.token_type == TokenType::Comma {
                    content.push_str(", ");
                } else {
                    if content.len() > OPTION_CLAUSE_OPEN.len()
                        && !content.ends_with('(')
                        && !content.ends_with(' ')
                    {
                        content.push(' ');
                    }
                    content.push_str(&tok.text);
                }
            }
        }
        content.push(')');
        Some(content)
    }

    pub(crate) fn parse_clickhouse_select_settings_and_format(
        &mut self,
    ) -> Result<(Option<Vec<Expression>>, Option<Expression>)> {
        Ok((None, None))
    }

    /// Parse a WITH clause (CTEs)
    pub(crate) fn parse_with(&mut self) -> Result<Expression> {
        let with_token = self.expect_token(TokenType::With)?;
        let leading_comments = with_token.comments;

        let recursive = self.match_token(TokenType::Recursive);
        let mut ctes: Vec<Cte> = Vec::new();
        let mut has_procedure = false;

        loop {
            let cte_start = self.current;
            let name = self.expect_identifier_or_alias_keyword_with_quoted()?;

            let columns = if self.check(TokenType::LParen) && !self.check_next(TokenType::Select) {
                self.skip();
                let cols = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                cols
            } else {
                Vec::new()
            };

            let key_expressions = if self.match_keywords(&[TokenType::Using, TokenType::Key]) {
                self.expect_token(TokenType::LParen)?;
                let keys = self.parse_identifier_list()?;
                self.expect_token(TokenType::RParen)?;
                keys
            } else {
                Vec::new()
            };

            let has_as = self.match_token(TokenType::As);
            let cte_comments = if has_as {
                self.previous_trailing_comments().to_vec()
            } else {
                Vec::new()
            };
            if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) {
                self.reject_reserved_name_part(&name)?;
                if !has_as {
                    return Err(self.parse_error("Expected AS before the CTE body"));
                }
            }
            if has_as && columns.is_empty() && self.at_anonymous_procedure() {
                let procedure = self.parse_anonymous_procedure(&name)?;
                has_procedure = true;
                ctes.push(Cte {
                    alias: name,
                    this: procedure,
                    columns,
                    materialized: None,
                    key_expressions,
                    alias_first: true,
                    comments: cte_comments,
                    span: self.source_range(cte_start),
                });
                if self.match_token(TokenType::Comma) {
                    continue;
                }
                break;
            }

            let materialized = if self.match_token(TokenType::Materialized) {
                Some(true)
            } else if self.match_token(TokenType::Not) {
                self.expect_token(TokenType::Materialized)?;
                Some(false)
            } else {
                None
            };

            self.expect_token(TokenType::LParen)?;
            if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) && self.check(TokenType::Values)
            {
                return Err(self.parse_error(format!(
                    "Snowflake does not accept a bare VALUES list as the body of CTE '{}'; \
                     select from it instead, as SELECT * FROM (VALUES ...) AS t(column, ...)",
                    name.name
                )));
            }
            let query_start = self.current;
            let query = self.parse_statement()?;
            let query = self.spanned(query_start, query);
            self.expect_token(TokenType::RParen)?;

            ctes.push(Cte {
                alias: name,
                this: query,
                columns,
                materialized,
                key_expressions,
                alias_first: true,
                comments: cte_comments,
                span: self.source_range(cte_start),
            });

            if !self.match_token(TokenType::Comma) {
                if self.check(TokenType::With) {
                    self.skip();
                    self.match_token(TokenType::Recursive);
                    continue;
                }
                break;
            }

            self.match_token(TokenType::With);
        }

        let search = self.parse_recursive_with_search()?;

        self.parse_with_statement(
            With {
                ctes,
                recursive,
                leading_comments,
                search,
            },
            has_procedure,
        )
    }
}
