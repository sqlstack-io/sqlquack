//! Parser ddl: CREATE FUNCTION.

use crate::parser::_helpers::ddl::snowflake_routines::nullability_property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::main::record_function_property::record_function_property;
use sqlquack_syntax::expressions::schema_objects::models::CreateFunction;
use sqlquack_syntax::expressions::schema_objects::models::FunctionBody;
use sqlquack_syntax::expressions::schema_objects::models::FunctionPropertyKind;
use sqlquack_syntax::expressions::schema_objects::models::FunctionSecurity;
use sqlquack_syntax::expressions::schema_objects::models::FunctionSetOption;
use sqlquack_syntax::expressions::schema_objects::models::FunctionSetValue;
use sqlquack_syntax::expressions::schema_objects::models::FunctionUsingResource;
use sqlquack_syntax::expressions::schema_objects::models::SqlDataAccess;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parse CREATE FUNCTION statement
    pub(crate) fn parse_create_function(
        &mut self,
        or_replace: bool,
        or_alter: bool,
        temporary: bool,
        is_table_function: bool,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Function)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = match self.is_snowflake() {
            true => self.parse_object_name()?,
            false => self.parse_table_ref()?,
        };

        let (parameters, has_parens) = if self.match_token(TokenType::LParen) {
            let params = self.parse_function_parameters()?;
            self.expect_token(TokenType::RParen)?;
            (params, true)
        } else {
            (Vec::new(), false)
        };

        let mut language_first = false;
        let mut return_type = None;
        let mut language = None;
        let sql_data_access = None;

        if self.match_token(TokenType::Language) {
            language = Some(self.expect_identifier_or_keyword()?);
            language_first = true;
        }

        let mut returns_table_body: Option<String> = None;
        (return_type, returns_table_body) =
            self.parse_create_function_returns(return_type, returns_table_body)?;
        let return_nullability = self.match_snowflake_return_nullability();

        let mut function = CreateFunction {
            name,
            parameters,
            return_type,
            body: None,
            or_replace,
            or_alter,
            if_not_exists,
            temporary,
            secure: false,
            external: false,
            language,
            deterministic: None,
            returns_null_on_null_input: None,
            security: None,
            has_parens,
            sql_data_access,
            returns_table_body,
            language_first,
            set_options: Vec::new(),
            strict: false,
            options: Vec::new(),
            is_table_function,
            property_order: Vec::new(),
            using_resources: Vec::new(),
            environment: Vec::new(),
            handler: None,
            handler_uses_eq: false,
            runtime_version: None,
            packages: None,
            parameter_style: None,
            snowflake_properties: return_nullability
                .into_iter()
                .map(nullability_property)
                .collect(),
            overloads: Vec::new(),
        };
        function = self.parse_create_function_properties(function)?;

        if function.options.is_empty() && self.match_identifier("OPTIONS") {
            let parsed_options = self.parse_options_list()?;
            function.options.extend(parsed_options);
            function = record_function_property(function, FunctionPropertyKind::Options);
        }

        Ok(Expression::CreateFunction(Box::new(function)))
    }

    fn parse_create_function_properties(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::Returns)
                && self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1].token_type == TokenType::Null
            {
                function = self.parse_create_function_returns_null_on_null(function)?;
            } else if self.match_token(TokenType::Returns) {
                function.return_type = Some(self.parse_data_type()?);
                if let Some(nullability) = self.match_snowflake_return_nullability() {
                    function
                        .snowflake_properties
                        .push(nullability_property(nullability));
                }
            } else if self.match_token(TokenType::Language) {
                function.language = Some(self.expect_identifier_or_keyword()?);
                function = record_function_property(function, FunctionPropertyKind::Language);
            } else if self.match_token(TokenType::Not) && self.match_identifier("DETERMINISTIC") {
                function.deterministic = Some(false);
                function = record_function_property(function, FunctionPropertyKind::Determinism);
            } else if self.match_identifier("DETERMINISTIC") || self.match_identifier("IMMUTABLE") {
                function.deterministic = Some(true);
                function = record_function_property(function, FunctionPropertyKind::Determinism);
            } else if self.match_identifier("STABLE") || self.match_identifier("VOLATILE") {
                function.deterministic = Some(false);
                function = record_function_property(function, FunctionPropertyKind::Determinism);
            } else if self.match_identifier("STRICT") {
                function.returns_null_on_null_input = Some(true);
                function.strict = true;
                function = record_function_property(function, FunctionPropertyKind::NullInput);
            } else if self.match_identifier("CALLED") {
                function = self.parse_create_function_called_on_null(function)?;
            } else if self.match_identifier("SECURITY") {
                function = self.parse_create_function_security(function);
            } else if self.match_identifier("CONTAINS") {
                self.match_identifier("SQL");
                function.sql_data_access = Some(SqlDataAccess::ContainsSql);
                function = record_function_property(function, FunctionPropertyKind::SqlDataAccess);
            } else if self.match_identifier("READS") {
                self.match_identifier("SQL");
                self.match_identifier("DATA");
                function.sql_data_access = Some(SqlDataAccess::ReadsSqlData);
                function = record_function_property(function, FunctionPropertyKind::SqlDataAccess);
            } else if self.match_identifier("MODIFIES") {
                self.match_identifier("SQL");
                self.match_identifier("DATA");
                function.sql_data_access = Some(SqlDataAccess::ModifiesSqlData);
                function = record_function_property(function, FunctionPropertyKind::SqlDataAccess);
            } else if self.match_token(TokenType::No) && self.match_identifier("SQL") {
                function.sql_data_access = Some(SqlDataAccess::NoSql);
                function = record_function_property(function, FunctionPropertyKind::SqlDataAccess);
            } else if self.match_token(TokenType::Set) {
                function = self.parse_create_function_set_options(function)?;
            } else if self.match_token(TokenType::As) {
                function = self.parse_create_function_as_body(function)?;
            } else if self.match_identifier("RETURN") {
                let expr = if self.check(TokenType::Select) || self.check(TokenType::With) {
                    self.parse_statement()?
                } else {
                    self.parse_expression()?
                };
                function.body = Some(FunctionBody::Return(expr));
            } else if self.match_token(TokenType::Using) {
                function = self.parse_create_function_using(function)?;
            } else if self.match_identifier("EXTERNAL") {
                function = self.parse_create_function_external(function)?;
            } else if self.match_identifier("OPTIONS") {
                let parsed_options = self.parse_options_list()?;
                function.options.extend(parsed_options);
                function = record_function_property(function, FunctionPropertyKind::Options);
            } else if self.match_identifier("ENVIRONMENT") {
                let parsed_env = self.parse_environment_list()?;
                function.environment.extend(parsed_env);
                function = record_function_property(function, FunctionPropertyKind::Environment);
            } else if self.check_identifier("HANDLER") && !self.names_snowflake_word_value() {
                self.skip();
                function = self.parse_create_function_handler(function)?;
            } else if self.match_identifier("RUNTIME_VERSION") {
                function = self.parse_create_function_runtime_version(function)?;
            } else if self.match_identifier("PACKAGES") {
                function = self.parse_create_function_packages(function)?;
            } else if self.match_text_seq(&["PARAMETER", "STYLE"]) {
                let style = self.expect_identifier_or_keyword()?;
                function.parameter_style = Some(style.to_ascii_uppercase());
                function = record_function_property(function, FunctionPropertyKind::ParameterStyle);
            } else if self.check_identifier("SQL")
                && self.current + 1 < self.tokens.len()
                && self.tokens[self.current + 1]
                    .text
                    .eq_ignore_ascii_case("SECURITY")
            {
                self.skip();
                self.skip();
                function = self.parse_create_function_security(function);
            } else if self.check(TokenType::Select) || self.check(TokenType::With) {
                let stmt = self.parse_statement()?;
                function.body = Some(FunctionBody::Expression(stmt));
                function = record_function_property(function, FunctionPropertyKind::As);
            } else if self.is_snowflake()
                && self.check(TokenType::From)
                && self.check_next_identifier("EXPERIMENT")
            {
                let source = self.parse_experiment_run_source()?;
                function.snowflake_properties.push(source);
            } else if let Some(property) = self.parse_snowflake_routine_property()? {
                function.snowflake_properties.push(property);
            } else {
                break;
            }
        }
        Ok(function)
    }

    fn parse_create_function_returns_null_on_null(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        self.skip();
        self.skip();
        self.expect_on_null_input()?;
        function.returns_null_on_null_input = Some(true);
        Ok(record_function_property(
            function,
            FunctionPropertyKind::NullInput,
        ))
    }

    fn parse_create_function_called_on_null(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        self.expect_on_null_input()?;
        function.returns_null_on_null_input = Some(false);
        Ok(record_function_property(
            function,
            FunctionPropertyKind::NullInput,
        ))
    }

    /// `ON NULL INPUT`, all three words in a Snowflake parse and any of them elsewhere.
    fn expect_on_null_input(&mut self) -> Result<()> {
        let words =
            [TokenType::On, TokenType::Null, TokenType::Input].map(|word| self.match_token(word));
        if self.is_snowflake() && words.contains(&false) {
            return Err(self.parse_error("Expected ON NULL INPUT"));
        }
        Ok(())
    }

    fn parse_create_function_security(&mut self, mut function: CreateFunction) -> CreateFunction {
        if self.match_identifier("DEFINER") {
            function.security = Some(FunctionSecurity::Definer);
        } else if self.match_identifier("INVOKER") {
            function.security = Some(FunctionSecurity::Invoker);
        }
        function = record_function_property(function, FunctionPropertyKind::Security);
        function
    }

    fn parse_create_function_set_options(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        let opt_name = self.expect_identifier_or_keyword()?;
        let value = if self.match_token(TokenType::From) {
            if !self.match_token(TokenType::Current) {
                return Err(self.parse_error("Expected CURRENT after FROM in SET option"));
            }
            FunctionSetValue::FromCurrent
        } else {
            let use_to = self.match_token(TokenType::To);
            if !use_to && !self.match_token(TokenType::Eq) {
                return Err(self.parse_error("Expected = or TO after SET key"));
            }
            let val = if self.check(TokenType::String) {
                let tok = self.advance()?;
                format!("'{}'", tok.text)
            } else {
                self.expect_identifier_or_keyword()?
            };
            FunctionSetValue::Value { value: val, use_to }
        };
        function.set_options.push(FunctionSetOption {
            name: opt_name,
            value,
        });
        function = record_function_property(function, FunctionPropertyKind::Set);
        Ok(function)
    }

    fn parse_create_function_as_body(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        function = record_function_property(function, FunctionPropertyKind::As);
        if self.is_snowflake()
            && !self.check(TokenType::String)
            && !self.check(TokenType::DollarString)
        {
            return Err(self.parse_error("Expected the function body as a string"));
        }
        if self.match_identifier("RETURN") {
            let expr = if self.check(TokenType::Select) || self.check(TokenType::With) {
                self.parse_statement()?
            } else {
                self.parse_expression()?
            };
            function.body = Some(FunctionBody::Return(expr));
        } else if self.check(TokenType::Select) || self.check(TokenType::With) {
            let stmt = self.parse_statement()?;
            function.body = Some(FunctionBody::Expression(stmt));
        } else if self.check(TokenType::DollarString) {
            let tok = self.advance()?;
            let (tag, content) =
                sqlquack_syntax::tokens::main::parse_dollar_string_token::parse_dollar_string_token(
                    &tok.text,
                );
            function.body = Some(FunctionBody::DollarQuoted { content, tag });
        } else if self.check(TokenType::String) {
            let tok = self.advance()?;
            function.body = Some(FunctionBody::StringLiteral(tok.text.to_string()));
        } else if self.match_token(TokenType::Begin) {
            let mut block_content = String::new();
            let mut depth = 1;
            while depth > 0 && !self.is_at_end() {
                let tok = self.advance()?;
                if tok.token_type == TokenType::Begin {
                    depth += 1;
                } else if tok.token_type == TokenType::End {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                block_content.push_str(&tok.text);
                block_content.push(' ');
            }
            function.body = Some(FunctionBody::Block(block_content.trim().to_string()));
        } else if self.check(TokenType::Table) {
            self.advance()?;
            if function.return_type.is_none() {
                function.return_type = Some(DataType::Custom {
                    name: "TABLE".to_string(),
                });
            }
            let stmt = self.parse_statement()?;
            function.body = Some(FunctionBody::Return(stmt));
        } else {
            let expr = self.parse_expression()?;
            function.body = Some(FunctionBody::Expression(expr));
        }
        Ok(function)
    }

    fn parse_create_function_using(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        while self.match_identifier("JAR")
            || self.match_identifier("FILE")
            || self.match_identifier("ARCHIVE")
        {
            let kind = self.previous_text().to_ascii_uppercase();
            let uri = self.expect_string()?;
            function
                .using_resources
                .push(FunctionUsingResource { kind, uri });
            let _ = self.match_token(TokenType::Comma);
        }
        function = record_function_property(function, FunctionPropertyKind::Using);
        Ok(function)
    }

    fn parse_create_function_external(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        self.match_identifier("NAME");
        let ext_name = if self.check(TokenType::String) {
            let tok = self.advance()?;
            tok.text.trim_matches('\'').to_string()
        } else {
            self.expect_identifier()?
        };
        function.body = Some(FunctionBody::External(ext_name));
        Ok(function)
    }

    fn parse_create_function_handler(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        function.handler_uses_eq = self.match_token(TokenType::Eq);
        if self.check(TokenType::String) {
            let tok = self.advance()?;
            function.handler = Some(tok.text.to_string());
        } else if self.is_snowflake() {
            return Err(self.parse_error("Expected the handler as a string"));
        }
        function = record_function_property(function, FunctionPropertyKind::Handler);
        Ok(function)
    }

    fn parse_create_function_runtime_version(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        let _ = self.match_token(TokenType::Eq);
        if self.check(TokenType::String)
            || self.check(TokenType::Number)
            || self.is_identifier_or_keyword_token()
        {
            function.runtime_version = Some(self.advance_text()?);
        }
        function = record_function_property(function, FunctionPropertyKind::RuntimeVersion);
        Ok(function)
    }

    fn parse_create_function_packages(
        &mut self,
        mut function: CreateFunction,
    ) -> Result<CreateFunction> {
        let _ = self.match_token(TokenType::Eq);
        let mut parsed_packages: Vec<String> = Vec::new();
        if self.match_token(TokenType::LParen) {
            while !self.is_at_end() && !self.check(TokenType::RParen) {
                if self.check(TokenType::String)
                    || self.check(TokenType::Identifier)
                    || self.check(TokenType::Var)
                    || self.check(TokenType::Number)
                    || self.check_keyword()
                {
                    parsed_packages.push(self.advance_text()?);
                } else {
                    break;
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        function.packages = Some(parsed_packages);
        function = record_function_property(function, FunctionPropertyKind::Packages);
        Ok(function)
    }

    fn parse_create_function_returns(
        &mut self,
        mut return_type: Option<DataType>,
        mut returns_table_body: Option<String>,
    ) -> Result<(Option<DataType>, Option<String>)> {
        if self.match_token(TokenType::Returns) {
            if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
                let var_name = self.advance_text()?;
                if self.check(TokenType::Table) {
                    self.skip();
                    return_type = Some(DataType::Custom {
                        name: "TABLE".to_string(),
                    });
                    if self.match_token(TokenType::LParen) {
                        let start = self.current;
                        let mut depth = 1;
                        while depth > 0 && !self.is_at_end() {
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
                        let mut col_defs_str = String::new();
                        for (i, tok) in self.tokens[start..self.current].iter().enumerate() {
                            let prev_tok = if i > 0 {
                                Some(&self.tokens[start + i - 1])
                            } else {
                                None
                            };
                            let needs_space = i > 0
                                && tok.token_type != TokenType::Comma
                                && tok.token_type != TokenType::RParen
                                && tok.token_type != TokenType::LParen
                                && prev_tok
                                    .map(|p| p.token_type != TokenType::LParen)
                                    .unwrap_or(true);
                            if needs_space {
                                col_defs_str.push(' ');
                            }
                            col_defs_str.push_str(&tok.text);
                        }
                        returns_table_body = Some(format!("{} TABLE ({})", var_name, col_defs_str));
                        self.expect_token(TokenType::RParen)?;
                    } else {
                        returns_table_body = Some(format!("{} TABLE", var_name));
                    }
                } else {
                    return_type = Some(self.parse_data_type()?);
                }
            } else if self.check(TokenType::Table) {
                self.skip();
                if self.check(TokenType::Lt) {
                    self.skip();
                    let mut cols: Vec<String> = Vec::new();
                    loop {
                        let col_name = self.expect_identifier()?;
                        let col_type = self.parse_data_type()?;
                        cols.push(format!(
                            "{} {}",
                            col_name,
                            self.data_type_to_string(&col_type)
                        ));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    if !self.match_token(TokenType::Gt) {
                        return Err(self.parse_error("Expected > after TABLE column definitions"));
                    }
                    returns_table_body = Some(format!("TABLE <{}>", cols.join(", ")));
                } else if self.check(TokenType::LParen) {
                    self.skip();
                    let mut cols: Vec<String> = Vec::new();
                    loop {
                        let untyped = self.is_snowflake()
                            && (self.check_next(TokenType::Comma)
                                || self.check_next(TokenType::RParen));
                        if untyped {
                            self.check_snowflake_object_name()?;
                            cols.push(self.advance_text()?);
                            if !self.match_token(TokenType::Comma) {
                                break;
                            }
                            continue;
                        }
                        let col_name = self.expect_identifier()?;
                        let col_type = self.parse_data_type()?;
                        cols.push(format!(
                            "{} {}",
                            col_name,
                            self.data_type_to_string(&col_type)
                        ));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    returns_table_body = Some(format!("TABLE ({})", cols.join(", ")));
                } else {
                    return_type = Some(DataType::Custom {
                        name: "TABLE".to_string(),
                    });
                }
            } else {
                return_type = Some(self.parse_function_return_type()?);
            }
        }
        Ok((return_type, returns_table_body))
    }
}
