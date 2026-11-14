//! Parser ddl: commands.

use crate::parser::constants::FUNCTION;
use crate::parser::constants::PROCEDURE;
use crate::parser::constants::QUALIFIED_NAME_PARTS;
use crate::parser::constants::VAR;
use crate::parser::constants::VARIABLE;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::main::identifier_from_token::identifier_from_token;
use sqlquack_syntax::expressions::names::models::BooleanLiteral;
use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::queries::models::Fetch;
use sqlquack_syntax::expressions::schema_objects::models::CreateDatabase;
use sqlquack_syntax::expressions::schema_objects::models::CreateSchema;
use sqlquack_syntax::expressions::schema_objects::models::DatabaseOption;
use sqlquack_syntax::expressions::schema_objects::models::DropDatabase;
use sqlquack_syntax::expressions::schema_objects::models::DropSchema;
use sqlquack_syntax::expressions::schema_objects::models::Grant;
use sqlquack_syntax::expressions::schema_objects::models::GrantPrincipal;
use sqlquack_syntax::expressions::schema_objects::models::Privilege;
use sqlquack_syntax::expressions::schema_objects::models::Revoke;
use sqlquack_syntax::expressions::statements::models::Command;
use sqlquack_syntax::expressions::statements::models::Comment;
use sqlquack_syntax::expressions::statements::models::ExecuteParameter;
use sqlquack_syntax::expressions::statements::models::ExecuteStatement;
use sqlquack_syntax::expressions::statements::models::Kill;
use sqlquack_syntax::expressions::statements::models::PrepareStatement;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::statements::models::SetItem;
use sqlquack_syntax::expressions::statements::models::SetStatement;
use sqlquack_syntax::expressions::statements::models::UnsetStatement;
use sqlquack_syntax::expressions::tables::models::CollateProperty;
use sqlquack_syntax::expressions::tables::models::Property;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parses Snowflake RM/REMOVE @stage.
    pub(crate) fn parse_rm_command(&mut self) -> Result<Expression> {
        self.check_snowflake_stage_command(false)?;
        let command_token = self.advance()?;
        let command_name = command_token.text.to_ascii_uppercase();

        let mut tokens = vec![(command_name, command_token.token_type)];
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let token = self.advance()?;
            tokens.push((token.text.to_string(), token.token_type));
        }

        Ok(Expression::Command(Box::new(Command {
            this: self.join_command_tokens(tokens),
        })))
    }

    /// Parses Snowflake GET @stage 'file:///path'.
    pub(crate) fn parse_get_command(&mut self) -> Result<Expression> {
        self.check_snowflake_stage_command(true)?;
        let get_token = self.advance()?;

        let mut tokens = vec![("GET".to_string(), get_token.token_type)];
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let token = self.advance()?;
            let text = match token.token_type {
                TokenType::String => format!("'{}'", token.text),
                TokenType::QuotedIdentifier => format!("\"{}\"", token.text),
                _ => token.text.to_string(),
            };
            tokens.push((text, token.token_type));
        }

        Ok(Expression::Command(Box::new(Command {
            this: self.join_command_tokens(tokens),
        })))
    }

    /// Parses CALL procedure(args, ...).
    pub(crate) fn parse_call(&mut self) -> Result<Expression> {
        let call_token = self.advance()?;
        if let Some(call) = self.parse_snowflake_method_call()? {
            return Ok(call);
        }
        if self.applies_snowflake_context_rules() {
            return self.parse_call_target(None);
        }

        let mut tokens = vec![("CALL".to_string(), call_token.token_type)];
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let token = self.advance()?;
            tokens.push((token.text.to_string(), token.token_type));
        }

        Ok(Expression::Command(Box::new(Command {
            this: self.join_command_tokens(tokens),
        })))
    }

    /// Parses MySQL/MariaDB KILL [CONNECTION | QUERY] id.
    pub(crate) fn parse_kill(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Kill)?;

        let kind = if self.match_identifier("CONNECTION") {
            Some("CONNECTION".to_string())
        } else if self.match_identifier("QUERY") {
            Some("QUERY".to_string())
        } else {
            None
        };

        let this = self.parse_primary()?;

        Ok(Expression::Kill(Box::new(Kill { this, kind })))
    }

    /// Parses PREPARE name [(type, ...)] AS statement.
    pub(crate) fn parse_prepare(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Prepare)?;

        let name = self.expect_identifier_or_keyword_with_quoted()?;

        let mut parameter_types: Vec<DataType> = Vec::new();
        if self.match_token(TokenType::LParen) {
            if !self.check(TokenType::RParen) {
                loop {
                    parameter_types.push(self.parse_data_type()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
        }

        self.expect_token(TokenType::As)?;
        let statement = self.parse_statement()?;

        Ok(Expression::Prepare(Box::new(PrepareStatement {
            name,
            parameter_types,
            statement,
        })))
    }

    pub(crate) fn is_postgres_prepared_execute_dialect(&self) -> bool {
        matches!(
            self.config.dialect,
            None | Some(sqlquack_syntax::tokens::models::DialectType::Generic)
                | Some(sqlquack_syntax::tokens::models::DialectType::PostgreSQL)
        )
    }

    /// Parses TSQL EXEC/EXECUTE [schema.]procedure [@param = value, ...].
    pub(crate) fn parse_execute(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Execute)?;
        if matches!(
            self.config.dialect,
            Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
        ) {
            if let Some(statement) = self.parse_snowflake_execute_object()? {
                return Ok(statement);
            }
            if self.applies_snowflake_context_rules()
                && self.match_text_seq(&["USING", "POLICY_CONTEXT"])
            {
                return self.parse_execute_using_policy_context();
            }
            if self.applies_snowflake_context_rules()
                || self.current + 1 >= self.tokens.len()
                || self.check_next(TokenType::Semicolon)
            {
                return Err(self.parse_error("Expected an object kind and name after EXECUTE"));
            }
        }

        let return_status = { None };

        let this = if self.check(TokenType::LParen) {
            self.skip();
            let expr = self.parse_disjunction()?.unwrap_or(Expression::Null(
                sqlquack_syntax::expressions::names::models::Null,
            ));
            self.expect_token(TokenType::RParen)?;
            Expression::Paren(Box::new(
                sqlquack_syntax::expressions::statements::models::Paren {
                    span: SourceSpan::default(),
                    this: expr,
                    trailing_comments: Vec::new(),
                },
            ))
        } else {
            let proc_name = self.parse_table_ref()?;
            Expression::Table(Box::new(proc_name))
        };

        if self.is_postgres_prepared_execute_dialect() && self.check(TokenType::LParen) {
            let arguments = self.parse_wrapped_csv_expressions()?;
            return Ok(Expression::Execute(Box::new(ExecuteStatement {
                this,
                return_status,
                parameters: Vec::new(),
                arguments,
                prepared: true,
                suffix: None,
            })));
        }
        if self.is_postgres_prepared_execute_dialect()
            && (self.check(TokenType::Semicolon) || self.is_at_end())
        {
            return Ok(Expression::Execute(Box::new(ExecuteStatement {
                this,
                return_status,
                parameters: Vec::new(),
                arguments: Vec::new(),
                prepared: true,
                suffix: None,
            })));
        }

        let mut parameters: Vec<ExecuteParameter> = Vec::new();

        while self.check(TokenType::Var) || self.check(TokenType::Parameter) {
            let token = self.advance()?;
            let param_name = if token.text.starts_with('@') {
                token.text.to_string()
            } else {
                format!("@{}", token.text)
            };

            if self.match_token(TokenType::Eq) {
                let value = self.parse_primary()?;
                let output = self.match_token(TokenType::Output);
                parameters.push(ExecuteParameter {
                    name: param_name,
                    value,
                    positional: false,
                    output,
                    span: SourceSpan::default(),
                });
            } else {
                let output = self.match_token(TokenType::Output);
                parameters.push(ExecuteParameter {
                    name: param_name.clone(),
                    value: Self::parsed_column(Column {
                        name: Identifier::new(&param_name),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    }),
                    positional: true,
                    output,
                    span: SourceSpan::default(),
                });
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let suffix = if self.check(TokenType::With) {
            let start = self.current;
            while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                self.skip();
            }
            Some(self.tokens_to_sql(start, self.current))
        } else {
            None
        };

        Ok(Expression::Execute(Box::new(ExecuteStatement {
            this,
            return_status,
            parameters,
            arguments: Vec::new(),
            prepared: false,
            suffix,
        })))
    }

    /// Parses GRANT privileges ON [kind] object TO principals [WITH GRANT OPTION].
    pub(crate) fn parse_grant(&mut self) -> Result<Expression> {
        let statement_start = self.current;
        self.expect_token(TokenType::Grant)?;
        if self.is_snowflake() {
            return self.parse_snowflake_grant(false);
        }

        let saved_pos = self.current;
        let mut depth = 0i32;
        let mut on_count = 0;
        let mut found_to = false;
        let mut i = self.current;
        while i < self.tokens.len() && self.tokens[i].token_type != TokenType::Semicolon {
            match self.tokens[i].token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen => depth -= 1,
                TokenType::On if depth == 0 => on_count += 1,
                TokenType::To if depth == 0 => {
                    found_to = true;
                }
                TokenType::Star if depth == 0 && on_count > 0 && !found_to => {
                    if i > 0
                        && self.tokens[i - 1].token_type != TokenType::Dot
                        && self.tokens[i - 1].token_type != TokenType::On
                    {}
                }
                TokenType::Replace if depth == 0 && found_to => {}
                _ => {}
            }
            i += 1;
        }
        if found_to && on_count == 0 {
            self.current = saved_pos;
            return self.fallback_to_command(statement_start);
        }

        self.current = saved_pos;

        let privileges = self.parse_privileges()?;

        self.expect_token(TokenType::On)?;

        let kind = self.parse_object_kind()?;

        let securable = self.parse_securable_name()?;

        let function_params = if self.check(TokenType::LParen) {
            self.parse_function_param_types()?
        } else {
            Vec::new()
        };

        self.expect_token(TokenType::To)?;

        let principals = self.parse_principals()?;

        let grant_option = self.match_token(TokenType::With)
            && self.check(TokenType::Grant)
            && {
                self.skip();
                self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("OPTION")
            }
            && {
                self.skip();
                true
            };

        let as_principal = if self.match_token(TokenType::As) {
            let name = self.expect_identifier_or_keyword()?;
            Some(Identifier::new(name))
        } else {
            None
        };

        Ok(Expression::Grant(Box::new(Grant {
            caller: None,
            bulk: None,
            container: None,
            privileges,
            kind,
            securable: Some(securable),
            function_params,
            principals,
            grant_option,
            as_principal,
            current_grants: None,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
        })))
    }

    /// Parses REVOKE [GRANT OPTION FOR] privileges ON [kind] object FROM principals [CASCADE].
    pub(crate) fn parse_revoke(&mut self) -> Result<Expression> {
        let statement_start = self.current;
        self.expect_token(TokenType::Revoke)?;
        if self.is_snowflake() {
            return self.parse_snowflake_grant(true);
        }

        let saved_pos = self.current;
        let mut depth = 0i32;
        let mut on_count = 0;
        let mut found_from = false;
        let mut i = self.current;
        while i < self.tokens.len() && self.tokens[i].token_type != TokenType::Semicolon {
            match self.tokens[i].token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen => depth -= 1,
                TokenType::On if depth == 0 => on_count += 1,
                TokenType::From if depth == 0 => {
                    found_from = true;
                }
                TokenType::Star
                    if depth == 0
                        && on_count > 0
                        && !found_from
                        && i > 0
                        && self.tokens[i - 1].token_type != TokenType::Dot
                        && self.tokens[i - 1].token_type != TokenType::On => {}
                _ => {}
            }
            i += 1;
        }
        if found_from && on_count == 0 {
            self.current = saved_pos;
            return self.fallback_to_command(statement_start);
        }

        self.current = saved_pos;

        let grant_option = if self.check(TokenType::Grant) {
            self.skip();
            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("OPTION") {
                self.skip();
                self.expect_token(TokenType::For)?;
                true
            } else {
                return Err(self.parse_error("Expected OPTION after GRANT in REVOKE"));
            }
        } else {
            false
        };

        let privileges = self.parse_privileges()?;

        self.expect_token(TokenType::On)?;

        let kind = self.parse_object_kind()?;

        let securable = self.parse_securable_name()?;

        let function_params = if self.check(TokenType::LParen) {
            self.parse_function_param_types()?
        } else {
            Vec::new()
        };

        self.expect_token(TokenType::From)?;

        let principals = self.parse_principals()?;

        let cascade = self.match_token(TokenType::Cascade);
        let restrict = if !cascade {
            self.match_token(TokenType::Restrict)
        } else {
            false
        };

        Ok(Expression::Revoke(Box::new(Revoke {
            caller: None,
            bulk: None,
            container: None,
            privileges,
            kind,
            securable: Some(securable),
            function_params,
            principals,
            grant_option,
            cascade,
            restrict,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
        })))
    }

    /// Parses GRANT/REVOKE privileges, multi-word (ALL PRIVILEGES) and per column (SELECT(c)).
    pub(crate) fn parse_privileges(&mut self) -> Result<Vec<Privilege>> {
        let mut privileges: Vec<Privilege> = Vec::new();
        loop {
            let mut priv_parts: Vec<String> = Vec::new();
            while !self.is_at_end() {
                if self.check(TokenType::On)
                    || self.check(TokenType::Comma)
                    || self.check(TokenType::LParen)
                {
                    break;
                }
                if self.is_identifier_or_keyword_token() {
                    priv_parts.push(self.advance_text()?.to_ascii_uppercase());
                } else {
                    break;
                }
            }
            if priv_parts.is_empty() {
                break;
            }
            let priv_name = priv_parts.join(" ");

            let columns = if self.match_token(TokenType::LParen) {
                let mut cols: Vec<String> = Vec::new();
                loop {
                    if self.is_identifier_or_keyword_token() {
                        cols.push(self.advance_text()?.to_string());
                    } else {
                        break;
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                cols
            } else {
                Vec::new()
            };

            privileges.push(Privilege {
                name: priv_name,
                columns,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(privileges)
    }

    /// Parse object kind (TABLE, SCHEMA, FUNCTION, PROCEDURE, SEQUENCE, etc.)
    pub(crate) fn parse_object_kind(&mut self) -> Result<Option<String>> {
        if self.check(TokenType::Table) {
            self.skip();
            Ok(Some("TABLE".to_string()))
        } else if self.check(TokenType::Schema) {
            self.skip();
            Ok(Some("SCHEMA".to_string()))
        } else if self.check(TokenType::Database) {
            self.skip();
            Ok(Some("DATABASE".to_string()))
        } else if self.check(TokenType::Function) {
            self.skip();
            Ok(Some("FUNCTION".to_string()))
        } else if self.check(TokenType::View) {
            self.skip();
            Ok(Some("VIEW".to_string()))
        } else if self.check(TokenType::Procedure) {
            self.skip();
            Ok(Some("PROCEDURE".to_string()))
        } else if self.check(TokenType::Sequence) {
            self.skip();
            Ok(Some("SEQUENCE".to_string()))
        } else if self.check(TokenType::Warehouse) {
            self.skip();
            Ok(Some("WAREHOUSE".to_string()))
        } else if self.check_identifier("STAGE")
            || self.check_identifier("INTEGRATION")
            || self.check_identifier("TASK")
            || self.check_identifier("STREAM")
            || self.check_identifier("PIPE")
            || self.check_identifier("TAG")
            || self.check_identifier("SHARE")
        {
            let kind = self.advance_text()?.to_ascii_uppercase();
            Ok(Some(kind))
        } else if self.check_identifier("FILE")
            && self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1]
                .text
                .eq_ignore_ascii_case("FORMAT")
        {
            self.skip();
            self.skip();
            Ok(Some("FILE FORMAT".to_string()))
        } else if self.check_identifier("NETWORK")
            && self.current + 1 < self.tokens.len()
            && self.tokens[self.current + 1]
                .text
                .eq_ignore_ascii_case("POLICY")
        {
            self.skip();
            self.skip();
            Ok(Some("NETWORK POLICY".to_string()))
        } else {
            Ok(None)
        }
    }

    /// Parse principal list for GRANT/REVOKE
    pub(crate) fn parse_principals(&mut self) -> Result<Vec<GrantPrincipal>> {
        let mut principals: Vec<GrantPrincipal> = Vec::new();
        loop {
            let is_role =
                if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ROLE") {
                    self.skip();
                    true
                } else {
                    false
                };
            let is_group = if !is_role && self.check(TokenType::Group) {
                self.skip();
                true
            } else {
                false
            };
            let is_share = if !is_role && !is_group && self.check_identifier("SHARE") {
                self.skip();
                true
            } else {
                false
            };
            let name = self.expect_identifier_or_keyword_with_quoted()?;
            principals.push(GrantPrincipal {
                name,
                is_role,
                is_group,
                is_share,
                kind: None,
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(principals)
    }

    /// Parses a possibly dot-qualified securable name into one identifier.
    pub(crate) fn parse_securable_name(&mut self) -> Result<Identifier> {
        let first = if self.match_token(TokenType::Star) {
            "*".to_string()
        } else {
            self.expect_identifier_or_keyword()?
        };
        let mut parts = vec![first];

        while self.match_token(TokenType::Dot) {
            let next = if self.match_token(TokenType::Star) {
                "*".to_string()
            } else {
                self.expect_identifier_or_keyword()?
            };
            parts.push(next);
        }

        Ok(Identifier::new(parts.join(".")))
    }

    /// Parses the parameter types of GRANT/REVOKE ON FUNCTION, such as `(number, varchar)`.
    pub(crate) fn parse_function_param_types(&mut self) -> Result<Vec<String>> {
        self.expect_token(TokenType::LParen)?;

        let mut params: Vec<String> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                let param_type = self.expect_identifier_or_keyword()?;
                params.push(param_type);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.expect_token(TokenType::RParen)?;
        Ok(params)
    }

    /// Parse COMMENT ON statement
    pub(crate) fn parse_comment(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Comment)?;

        let exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        self.expect_token(TokenType::On)?;

        let materialized = if self.match_token(TokenType::Materialized) {
            true
        } else if self.check(TokenType::Var)
            && self.peek_text().eq_ignore_ascii_case("MATERIALIZED")
        {
            self.skip();
            true
        } else {
            false
        };

        let kind = self.expect_identifier_or_keyword()?.to_ascii_uppercase();

        let this = if kind == PROCEDURE || kind == FUNCTION {
            let name_token = self.advance()?;
            let mut name_str = name_token.text.to_string();

            while self.match_token(TokenType::Dot) {
                let next = self.advance()?;
                name_str.push('.');
                name_str.push_str(&next.text);
            }

            if self.match_token(TokenType::LParen) {
                name_str.push('(');
                let mut first = true;
                while !self.check(TokenType::RParen) && !self.is_at_end() {
                    if !first {
                        name_str.push_str(", ");
                    }
                    first = false;
                    let param_token = self.advance()?;
                    name_str.push_str(&param_token.text);
                    self.match_token(TokenType::Comma);
                }
                self.expect_token(TokenType::RParen)?;
                name_str.push(')');
            }

            Expression::Identifier(Identifier::new(name_str))
        } else {
            self.check_snowflake_comment_name()?;
            self.parse_qualified_name()?
        };

        if self.check(TokenType::Is) {
            self.skip();
        } else {
            return Err(self.parse_error("Expected IS in COMMENT ON statement"));
        }
        if self.is_snowflake()
            && !self.check(TokenType::String)
            && !self.check(TokenType::DollarString)
            && !self.check(TokenType::NationalString)
        {
            return Err(self.parse_error("Expected a string after IS"));
        }

        let expression = self.parse_primary()?;

        Ok(Expression::Comment(Box::new(Comment {
            this,
            kind,
            expression,
            exists,
            materialized,
        })))
    }

    /// Snowflake `UNSET name` or `UNSET ( name [ , ... ] )`.
    pub(crate) fn parse_unset(&mut self) -> Result<Expression> {
        self.skip();
        let parenthesized = self.match_token(TokenType::LParen);
        let mut names = vec![self.expect_identifier_or_keyword_with_quoted()?];
        while parenthesized && self.match_token(TokenType::Comma) {
            names.push(self.expect_identifier_or_keyword_with_quoted()?);
        }
        for name in &names {
            self.reject_reserved_name_part(name)?;
        }
        if parenthesized {
            self.expect_token(TokenType::RParen)?;
        }
        Ok(Expression::UnsetStatement(Box::new(UnsetStatement {
            names,
            parenthesized,
        })))
    }

    /// Parse SET statement
    pub(crate) fn parse_set(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Set)?;

        let mut items: Vec<SetItem> = Vec::new();

        let session_authorization_kind =
            if self.match_text_seq(&["LOCAL", "SESSION", "AUTHORIZATION"]) {
                Some(Some("LOCAL".to_string()))
            } else if self.match_text_seq(&["SESSION", "SESSION", "AUTHORIZATION"]) {
                Some(Some("SESSION".to_string()))
            } else if self.match_text_seq(&["SESSION", "AUTHORIZATION"]) {
                Some(None)
            } else {
                None
            };

        if let Some(kind) = session_authorization_kind {
            return self.parse_set_session_authorization(items, kind);
        }

        if self.match_identifier("CHARACTER") {
            return self.parse_set_character_set(items);
        }

        if self.match_identifier("NAMES") {
            return self.parse_set_names(items);
        }

        let mut set_is_variable = self.parse_set_is_variable();

        loop {
            let kind = if self.match_identifier("GLOBAL") {
                Some("GLOBAL".to_string())
            } else if self.match_token(TokenType::Local) {
                Some("LOCAL".to_string())
            } else if self.match_identifier("SESSION") {
                Some("SESSION".to_string())
            } else if self.match_identifier("PERSIST") {
                Some("PERSIST".to_string())
            } else if self.match_identifier("PERSIST_ONLY") {
                Some("PERSIST_ONLY".to_string())
            } else if set_is_variable {
                set_is_variable = false;
                Some("VARIABLE".to_string())
            } else {
                None
            };

            if self.match_token(TokenType::Transaction) {
                let mut characteristics: Vec<String> = Vec::new();
                loop {
                    let mut char_tokens: Vec<String> = Vec::new();
                    while !self.is_at_end()
                        && !self.check(TokenType::Comma)
                        && !self.check(TokenType::Semicolon)
                    {
                        if self.is_identifier_token()
                            || self.is_safe_keyword_as_identifier()
                            || self.check(TokenType::Only)
                            || self.check(TokenType::Repeatable)
                        {
                            char_tokens.push(self.advance_text()?);
                        } else {
                            break;
                        }
                    }
                    if !char_tokens.is_empty() {
                        characteristics.push(char_tokens.join(" "));
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }

                let name = Expression::Identifier(Identifier::new("TRANSACTION".to_string()));
                let value = if characteristics.is_empty() {
                    Expression::Identifier(Identifier::new("".to_string()))
                } else {
                    Expression::Identifier(Identifier::new(characteristics.join(", ")))
                };

                items.push(SetItem {
                    name,
                    value,
                    kind,
                    no_equals: false,
                });
                break;
            }

            self.reject_snowflake_set_name()?;
            let name = self.parse_set_item_name()?;

            if self.match_token(TokenType::Eq)
                || self.match_token(TokenType::ColonEq)
                || self.match_token(TokenType::To)
            {
            } else if self.is_at_end()
                || self.check(TokenType::Semicolon)
                || self.check(TokenType::Comma)
            {
                items.push(SetItem {
                    name,
                    value: Expression::Identifier(Identifier::new("".to_string())),
                    kind,
                    no_equals: false,
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            } else {
                if self.check(TokenType::On) || self.check_keyword_text("OFF") {
                    let val = self.advance_text()?;
                    let name_with_val = match &name {
                        Expression::Column(col) => format!("{} {}", col.name.name, val),
                        Expression::Identifier(id) => format!("{} {}", id.name, val),
                        _ => val.clone(),
                    };
                    items.push(SetItem {
                        name: Expression::Identifier(Identifier::new(name_with_val)),
                        value: Expression::Identifier(Identifier::new("".to_string())),
                        kind,
                        no_equals: false,
                    });
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                    continue;
                }
                if !self.is_at_end() && !self.check(TokenType::Semicolon) {
                    let value = self.parse_expression()?;
                    items.push(SetItem {
                        name,
                        value,
                        kind,
                        no_equals: true,
                    });
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                    continue;
                }
                return Err(self.parse_error("Expected '=' or 'TO' in SET statement"));
            }

            let value = self.parse_set_item_value()?;

            items.push(SetItem {
                name,
                value,
                kind,
                no_equals: false,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(Expression::SetStatement(Box::new(SetStatement { items })))
    }

    fn parse_set_item_value(&mut self) -> Result<Expression> {
        Ok(
            if self.check(TokenType::On) || self.check_keyword_text("OFF") {
                Expression::Identifier(identifier_from_token(self.advance()?, false))
            } else if self.match_token(TokenType::Default) {
                Expression::Identifier(Identifier::new("DEFAULT".to_string()))
            } else {
                self.parse_expression()?
            },
        )
    }

    fn parse_set_session_authorization(
        &mut self,
        mut items: Vec<SetItem>,
        kind: Option<String>,
    ) -> Result<Expression> {
        let value = if self.match_token(TokenType::Default) {
            Expression::Identifier(Identifier::new("DEFAULT"))
        } else if self.is_at_end() || self.check(TokenType::Semicolon) {
            return Err(
                self.parse_error("Expected user name or DEFAULT after SET SESSION AUTHORIZATION")
            );
        } else {
            self.parse_primary()?
        };

        items.push(SetItem {
            name: Expression::Identifier(Identifier::new("SESSION AUTHORIZATION")),
            value,
            kind,
            no_equals: true,
        });

        Ok(Expression::SetStatement(Box::new(SetStatement { items })))
    }

    fn parse_set_character_set(&mut self, mut items: Vec<SetItem>) -> Result<Expression> {
        self.expect_token(TokenType::Set)?;
        let value = if self.match_token(TokenType::Default) {
            Expression::Identifier(Identifier::new("DEFAULT".to_string()))
        } else {
            self.parse_primary()?
        };
        items.push(SetItem {
            name: Expression::Identifier(Identifier::new("CHARACTER SET".to_string())),
            value,
            kind: None,
            no_equals: false,
        });
        Ok(Expression::SetStatement(Box::new(SetStatement { items })))
    }

    fn parse_set_names(&mut self, mut items: Vec<SetItem>) -> Result<Expression> {
        let value = if self.match_token(TokenType::Default) {
            Expression::Identifier(Identifier::new("DEFAULT".to_string()))
        } else {
            self.parse_primary()?
        };
        let collation = if self.match_identifier("COLLATE") {
            Some(self.parse_primary()?)
        } else {
            None
        };
        items.push(SetItem {
            name: Expression::Identifier(Identifier::new("NAMES".to_string())),
            value,
            kind: None,
            no_equals: false,
        });
        if let Some(coll) = collation {
            items.push(SetItem {
                name: Expression::Identifier(Identifier::new("COLLATE".to_string())),
                value: coll,
                kind: None,
                no_equals: false,
            });
        }
        Ok(Expression::SetStatement(Box::new(SetStatement { items })))
    }

    fn parse_set_is_variable(&mut self) -> bool {
        if self.check(TokenType::Var) {
            let text = self.peek_text().to_uppercase();
            if text == VARIABLE || text == VAR {
                if let Some(next) = self.tokens.get(self.current + 1) {
                    if next.token_type != TokenType::Eq
                        && next.token_type != TokenType::To
                        && next.token_type != TokenType::ColonEq
                    {
                        self.skip();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn parse_set_item_name(&mut self) -> Result<Expression> {
        Ok({
            if self.check(TokenType::AtAt) {
                self.skip();
                let mut name_str = "@@".to_string();
                let first = self.advance_text()?;
                name_str.push_str(&first);
                while self.match_token(TokenType::Dot) {
                    let next = self.advance_text()?;
                    name_str.push('.');
                    name_str.push_str(&next);
                }
                Expression::Identifier(Identifier::new(name_str))
            } else if self.check(TokenType::DAt) {
                self.skip();
                let mut name_str = "@".to_string();
                let first = self.advance_text()?;
                name_str.push_str(&first);
                Expression::Identifier(Identifier::new(name_str))
            } else if self.check(TokenType::LParen) {
                self.skip();
                let mut vars: Vec<Expression> = Vec::new();
                loop {
                    let var_name = self.expect_identifier_or_keyword_with_quoted()?;
                    self.reject_reserved_name_part(&var_name)?;
                    vars.push(Expression::Column(Box::new(Column {
                        name: var_name,
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    })));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                Expression::Tuple(Box::new(
                    sqlquack_syntax::expressions::types::models::Tuple { expressions: vars },
                ))
            } else {
                let first = self.advance_text()?;
                let mut name_str = first;
                while self.match_token(TokenType::Dot) {
                    let next = self.advance_text()?;
                    name_str.push('.');
                    name_str.push_str(&next);
                }
                while self.check(TokenType::Colon) && !self.check_next(TokenType::Eq) {
                    self.skip();
                    let next = self.advance_text()?;
                    name_str.push(':');
                    name_str.push_str(&next);
                }
                Expression::Identifier(Identifier::new(name_str))
            }
        })
    }

    /// Parse FETCH FIRST/NEXT clause
    pub(crate) fn parse_fetch(&mut self) -> Result<Fetch> {
        let direction = if self.match_token(TokenType::First) {
            "FIRST".to_string()
        } else if self.match_token(TokenType::Next) {
            "NEXT".to_string()
        } else {
            "FIRST".to_string()
        };

        let count = if !self.check(TokenType::Row)
            && !self.check(TokenType::Rows)
            && !self.check(TokenType::Percent)
            && !self.check(TokenType::Only)
        {
            if self.check(TokenType::Number)
                || self.check(TokenType::Dash)
                || self.check(TokenType::Plus)
                || self.check(TokenType::LParen)
                || self.check(TokenType::DAt)
                || self.check(TokenType::Var)
            {
                Some(self.parse_unary()?)
            } else {
                None
            }
        } else {
            None
        };

        let percent = self.match_token(TokenType::Percent);

        let rows = self.match_token(TokenType::Row) || self.match_token(TokenType::Rows);

        self.match_token(TokenType::Only);
        let with_ties = self.match_keywords(&[TokenType::With, TokenType::Ties]);

        Ok(Fetch {
            direction,
            count,
            percent,
            rows,
            with_ties,
        })
    }

    /// Parse a qualified name (schema.table.column or just table)
    pub(crate) fn parse_qualified_name(&mut self) -> Result<Expression> {
        let first = self.parse_quoting_identifier(Self::expect_identifier_or_keyword)?;
        let mut parts = vec![first];

        while self.match_token(TokenType::Dot) {
            let next = self.parse_quoting_identifier(Self::expect_identifier_or_keyword)?;
            parts.push(next);
        }

        if parts.len() == 1 {
            Ok(Expression::Identifier(parts.remove(0)))
        } else if parts.len() == QUALIFIED_NAME_PARTS {
            Ok(Self::parsed_column(Column {
                table: Some(parts[0].clone()),
                name: parts[1].clone(),
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }))
        } else {
            let column_name = parts.pop().ok_or_else(|| {
                sqlquack_syntax::tokens::errors::Error::internal(
                    "a value the parser checked is missing",
                )
            })?;
            let table_span = parts
                .first()
                .and_then(|id| id.span)
                .zip(parts.last().and_then(|id| id.span))
                .map(|(start, end)| start.through(end));
            let table_name = parts
                .iter()
                .map(|id| match id.quoted {
                    true => format!("\"{}\"", id.name.replace('"', "\"\"")),
                    false => id.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(".");
            let mut table = Identifier::new(table_name);
            table.span = table_span;
            Ok(Self::parsed_column(Column {
                table: Some(table),
                name: column_name,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }))
        }
    }

    /// Parse CREATE SCHEMA statement
    pub(crate) fn parse_create_schema(
        &mut self,
        leading_comments: Vec<String>,
        or_replace: bool,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Schema)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = self.parse_identifier_parts()?;

        let clone_from = if self.match_identifier("CLONE") {
            Some(self.parse_identifier_parts()?)
        } else {
            None
        };

        let at_clause = if self.is_snowflake() {
            self.parse_clone_time_travel()?
        } else {
            self.parse_raw_at_clause()?
        };

        let authorization = if self.match_token(TokenType::Authorization) {
            Some(self.parse_unquoted_identifier(Self::expect_identifier)?)
        } else {
            None
        };

        let mut properties = self.parse_clone_container_properties(clone_from.is_some())?;
        if self.is_snowflake() {
            properties.extend(self.parse_snowflake_container_properties(false)?);
        }

        if self.match_token(TokenType::With) {
            self.expect_token(TokenType::LParen)?;
            loop {
                let prop_name = if self.check(TokenType::String) {
                    Expression::Literal(Box::new(Literal::String(self.expect_string()?)))
                } else {
                    Expression::Identifier(
                        self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?,
                    )
                };
                self.expect_token(TokenType::Eq)?;
                let prop_value = self.parse_expression()?;
                properties.push(Expression::Property(Box::new(Property {
                    this: Box::new(prop_name),
                    value: Some(Box::new(prop_value)),
                })));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }

        if self.match_token(TokenType::Default) && self.match_token(TokenType::Collate) {
            let collation = self.parse_primary()?;
            properties.push(Expression::CollateProperty(Box::new(CollateProperty {
                this: Box::new(collation),
                default: Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                }))),
            })));
        }

        Ok(Expression::CreateSchema(Box::new(CreateSchema {
            name,
            or_replace,
            if_not_exists,
            authorization,
            clone_from,
            at_clause,
            properties,
            leading_comments,
        })))
    }

    /// Parse DROP SCHEMA statement
    pub(crate) fn parse_drop_schema(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Schema)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.expect_identifier_or_safe_keyword_with_quoted()?;

        let cascade = self.match_token(TokenType::Cascade);
        let restrict = !cascade && self.match_token(TokenType::Restrict);

        Ok(Expression::DropSchema(Box::new(DropSchema {
            name,
            if_exists,
            cascade,
            restrict,
        })))
    }

    /// Parse CREATE DATABASE statement
    pub(crate) fn parse_create_database(&mut self, or_replace: bool) -> Result<Expression> {
        self.expect_token(TokenType::Database)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = if self.is_snowflake() {
            self.parse_quoting_identifier(Self::expect_identifier)?
        } else {
            self.parse_unquoted_identifier(Self::expect_identifier)?
        };

        let clone_from = if self.match_identifier("CLONE") {
            Some(self.expect_identifier_or_safe_keyword_with_quoted()?)
        } else {
            None
        };

        let at_clause = if self.is_snowflake() {
            self.parse_clone_time_travel()?
        } else {
            self.parse_raw_at_clause()?
        };

        let _on_cluster = self.parse_on_cluster_clause()?;

        let mut properties = self.parse_clone_container_properties(clone_from.is_some())?;
        if self.is_snowflake() {
            properties.extend(self.parse_snowflake_container_properties(true)?);
        }
        let mut options: Vec<DatabaseOption> = Vec::new();

        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.match_identifier("OWNER") || self.match_token(TokenType::Eq) {
                self.match_token(TokenType::Eq);
                options.push(DatabaseOption::Owner(Identifier::new(
                    self.expect_identifier()?,
                )));
            } else if self.match_identifier("TEMPLATE") {
                self.match_token(TokenType::Eq);
                options.push(DatabaseOption::Template(Identifier::new(
                    self.expect_identifier()?,
                )));
            } else if self.match_identifier("ENCODING") {
                self.match_token(TokenType::Eq);
                let encoding = if self.check(TokenType::String) {
                    let tok = self.advance()?;
                    tok.text.trim_matches('\'').to_string()
                } else {
                    self.expect_identifier()?
                };
                options.push(DatabaseOption::Encoding(encoding));
            } else if self.match_identifier("CHARACTER") {
                self.match_token(TokenType::Set);
                self.match_token(TokenType::Eq);
                let charset = if self.check(TokenType::String) {
                    let tok = self.advance()?;
                    tok.text.trim_matches('\'').to_string()
                } else {
                    self.expect_identifier()?
                };
                options.push(DatabaseOption::CharacterSet(charset));
            } else if self.match_identifier("COLLATE") {
                self.match_token(TokenType::Eq);
                let collate = if self.check(TokenType::String) {
                    let tok = self.advance()?;
                    tok.text.trim_matches('\'').to_string()
                } else {
                    self.expect_identifier()?
                };
                options.push(DatabaseOption::Collate(collate));
            } else if self.match_identifier("LOCATION") {
                self.match_token(TokenType::Eq);
                let loc = if self.check(TokenType::String) {
                    let tok = self.advance()?;
                    tok.text.trim_matches('\'').to_string()
                } else {
                    self.expect_identifier()?
                };
                options.push(DatabaseOption::Location(loc));
            } else {
                break;
            }
        }

        Ok(Expression::CreateDatabase(Box::new(CreateDatabase {
            name,
            or_replace,
            if_not_exists,
            options,
            clone_from,
            at_clause,
            properties,
        })))
    }

    /// A schema or database clone's `AT | BEFORE ( ... )` kept as written, outside Snowflake.
    fn parse_raw_at_clause(&mut self) -> Result<Option<Expression>> {
        if !(self.match_identifier("AT") || self.match_token(TokenType::Before)) {
            return Ok(None);
        }
        let keyword = self.previous_text().to_ascii_uppercase();
        self.expect_token(TokenType::LParen)?;
        let mut result = format!("{} (", keyword);
        let mut prev_token_type: Option<TokenType> = None;
        let mut paren_depth = 1;
        while !self.is_at_end() && paren_depth > 0 {
            let token = self.advance()?;
            if token.token_type == TokenType::LParen {
                paren_depth += 1;
            } else if token.token_type == TokenType::RParen {
                paren_depth -= 1;
                if paren_depth == 0 {
                    break;
                }
            }
            let needs_space = !result.ends_with('(')
                && prev_token_type != Some(TokenType::Arrow)
                && prev_token_type != Some(TokenType::Dash)
                && prev_token_type != Some(TokenType::LParen)
                && token.token_type != TokenType::LParen;
            if needs_space
                && token.token_type != TokenType::RParen
                && token.token_type != TokenType::Comma
            {
                result.push(' ');
            }
            if token.token_type == TokenType::String {
                result.push('\'');
                result.push_str(&token.text.replace('\'', "''"));
                result.push('\'');
            } else {
                result.push_str(&token.text);
            }
            if token.token_type == TokenType::Arrow || token.token_type == TokenType::Comma {
                result.push(' ');
            }
            prev_token_type = Some(token.token_type);
        }
        result.push(')');
        Ok(Some(Expression::Raw(Raw { sql: result })))
    }

    /// Parse DROP DATABASE statement
    pub(crate) fn parse_drop_database(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Database)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);

        let name = self.expect_identifier_or_safe_keyword_with_quoted()?;

        let sync = { false };
        let cascade = self.match_token(TokenType::Cascade);
        let restrict = !cascade && self.match_token(TokenType::Restrict);

        Ok(Expression::DropDatabase(Box::new(DropDatabase {
            name,
            if_exists,
            sync,
            cascade,
            restrict,
        })))
    }
}
