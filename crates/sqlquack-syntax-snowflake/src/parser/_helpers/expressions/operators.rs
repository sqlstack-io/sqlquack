//! Parser expressions: operators.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::functions::models::Xor;
use sqlquack_syntax::expressions::names::main::identifier_from_token::identifier_from_token;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::main::maybe_rebalance_boolean_chain::maybe_rebalance_boolean_chain;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::operators::models::IsNull;
use sqlquack_syntax::expressions::operators::models::UnaryOp;
use sqlquack_syntax::expressions::schema_objects::models::AlterIndex;
use sqlquack_syntax::expressions::schema_objects::models::AlterIndexAction;
use sqlquack_syntax::expressions::schema_objects::models::AlterSequence;
use sqlquack_syntax::expressions::schema_objects::models::AlterView;
use sqlquack_syntax::expressions::schema_objects::models::AlterViewAction;
use sqlquack_syntax::expressions::schema_objects::models::CreateProcedure;
use sqlquack_syntax::expressions::schema_objects::models::CreateSequence;
use sqlquack_syntax::expressions::schema_objects::models::CreateTrigger;
use sqlquack_syntax::expressions::schema_objects::models::CreateType;
use sqlquack_syntax::expressions::schema_objects::models::DomainConstraint;
use sqlquack_syntax::expressions::schema_objects::models::DropFunction;
use sqlquack_syntax::expressions::schema_objects::models::DropProcedure;
use sqlquack_syntax::expressions::schema_objects::models::DropSequence;
use sqlquack_syntax::expressions::schema_objects::models::DropTrigger;
use sqlquack_syntax::expressions::schema_objects::models::DropType;
use sqlquack_syntax::expressions::schema_objects::models::FunctionBody;
use sqlquack_syntax::expressions::schema_objects::models::FunctionParameter;
use sqlquack_syntax::expressions::schema_objects::models::FunctionSecurity;
use sqlquack_syntax::expressions::schema_objects::models::ParameterMode;
use sqlquack_syntax::expressions::schema_objects::models::SeqPropKind;
use sqlquack_syntax::expressions::schema_objects::models::SequenceBound;
use sqlquack_syntax::expressions::schema_objects::models::TriggerBody;
use sqlquack_syntax::expressions::schema_objects::models::TriggerEvent;
use sqlquack_syntax::expressions::schema_objects::models::TriggerForEach;
use sqlquack_syntax::expressions::schema_objects::models::TriggerReferencing;
use sqlquack_syntax::expressions::schema_objects::models::TriggerTiming;
use sqlquack_syntax::expressions::schema_objects::models::TypeAttribute;
use sqlquack_syntax::expressions::schema_objects::models::TypeDefinition;
use sqlquack_syntax::expressions::schema_objects::models::ViewColumn;
use sqlquack_syntax::expressions::statements::models::Annotated;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::typed_functions::models::JsonExtractFunc;
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parse function parameters
    pub(crate) fn parse_function_parameters(&mut self) -> Result<Vec<FunctionParameter>> {
        let mut params: Vec<FunctionParameter> = Vec::new();

        if self.check(TokenType::RParen) {
            return Ok(params);
        }

        loop {
            let mut mode = None;
            let mut mode_text: Option<String> = None;

            if self.match_token(TokenType::In) {
                if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("OUT") {
                    let out_text = self.advance_text()?;
                    mode_text = Some(format!("IN {}", out_text));
                    mode = Some(ParameterMode::InOut);
                } else {
                    mode_text = Some("IN".to_string());
                    mode = Some(ParameterMode::In);
                }
            } else if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("OUT") {
                let text = self.advance_text()?;
                mode_text = Some(text);
                mode = Some(ParameterMode::Out);
            } else if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("INOUT") {
                let text = self.advance_text()?;
                mode_text = Some(text);
                mode = Some(ParameterMode::InOut);
            } else if self.check(TokenType::Var)
                && self.peek_text().eq_ignore_ascii_case("VARIADIC")
            {
                let text = self.advance_text()?;
                mode_text = Some(text);
                mode = Some(ParameterMode::Variadic);
            }

            let (name, data_type) = if mode.is_some() {
                let saved = self.current;
                let type_result = self.parse_data_type();
                if let Ok(dt) = type_result {
                    if self.check(TokenType::Comma)
                        || self.check(TokenType::RParen)
                        || self.check(TokenType::Default)
                        || self.check(TokenType::Eq)
                    {
                        (None, dt)
                    } else {
                        self.current = saved;
                        let first_ident =
                            if self.check(TokenType::Input) || self.check(TokenType::Output) {
                                let token = self.advance()?;
                                identifier_from_token(token, false)
                            } else {
                                self.expect_identifier_with_quoted()?
                            };
                        self.match_token(TokenType::As);
                        let dt = self.parse_data_type()?;
                        (Some(first_ident), dt)
                    }
                } else {
                    self.current = saved;
                    let first_ident =
                        if self.check(TokenType::Input) || self.check(TokenType::Output) {
                            let token = self.advance()?;
                            identifier_from_token(token, false)
                        } else {
                            self.expect_identifier_with_quoted()?
                        };
                    if self.check(TokenType::Comma)
                        || self.check(TokenType::RParen)
                        || self.check(TokenType::Default)
                    {
                        (None, self.identifier_to_datatype(&first_ident.name)?)
                    } else {
                        self.match_token(TokenType::As);
                        let dt = self.parse_data_type()?;
                        (Some(first_ident), dt)
                    }
                }
            } else {
                let first_ident = if self.check(TokenType::Input) || self.check(TokenType::Output) {
                    let token = self.advance()?;
                    identifier_from_token(token, false)
                } else {
                    self.expect_identifier_with_quoted()?
                };

                if self.check(TokenType::Comma)
                    || self.check(TokenType::RParen)
                    || self.check(TokenType::Default)
                {
                    (None, self.identifier_to_datatype(&first_ident.name)?)
                } else {
                    self.match_token(TokenType::As);
                    if let Some((after_name, text)) = self.match_argument_mode()? {
                        mode = Some(after_name);
                        mode_text = Some(text);
                        let dt = self.parse_data_type()?;
                        if self.check(TokenType::Default) {
                            return Err(
                                self.parse_error("Expected no DEFAULT after an argument mode")
                            );
                        }
                        (Some(first_ident), dt)
                    } else {
                        let dt = self.parse_data_type()?;
                        (Some(first_ident), dt)
                    }
                }
            };

            let default = if self.match_token(TokenType::Default) || self.match_token(TokenType::Eq)
            {
                Some(self.parse_expression()?)
            } else {
                None
            };

            params.push(FunctionParameter {
                name,
                data_type,
                mode,
                default,
                mode_text: mode_text.clone(),
                span: SourceSpan::default(),
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(params)
    }

    /// A Snowflake procedure or function argument's mode after its name, IN, INPUT, OUT or OUTPUT, when its type follows.
    fn match_argument_mode(&mut self) -> Result<Option<(ParameterMode, String)>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        let mode = if self.check(TokenType::In) || self.check(TokenType::Input) {
            ParameterMode::In
        } else if self.check(TokenType::Output) || self.check_identifier("OUT") {
            ParameterMode::Out
        } else {
            return Ok(None);
        };
        if self.check_next(TokenType::Comma)
            || self.check_next(TokenType::RParen)
            || self.check_next(TokenType::Default)
        {
            return Ok(None);
        }
        let text = self.advance_text()?.to_ascii_uppercase();
        Ok(Some((mode, text)))
    }

    /// Parses TSQL unparenthesized procedure parameters `@p1 TYPE, @p2 TYPE ... AS`.
    pub(crate) fn parse_tsql_procedure_params(&mut self) -> Result<Vec<FunctionParameter>> {
        let mut params: Vec<FunctionParameter> = Vec::new();
        loop {
            if !self.check(TokenType::Var) {
                break;
            }
            let name = self.advance_text()?;
            self.match_token(TokenType::As);
            let data_type = self.parse_data_type()?;
            let default = if self.match_token(TokenType::Default) || self.match_token(TokenType::Eq)
            {
                Some(self.parse_expression()?)
            } else {
                None
            };
            params.push(FunctionParameter {
                name: Some(Identifier::new(name)),
                data_type,
                mode: None,
                default,
                mode_text: None,
                span: SourceSpan::default(),
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        Ok(params)
    }

    /// A function parameter type from an identifier, keeping its name as written.
    pub(crate) fn identifier_to_datatype(&self, ident: &str) -> Result<DataType> {
        Ok(DataType::Custom {
            name: ident.to_string(),
        })
    }

    /// Parses a RETURNS data type, keeping simple type names as written.
    pub(crate) fn parse_function_return_type(&mut self) -> Result<DataType> {
        if (self.check(TokenType::Identifier) || self.check(TokenType::Var))
            && !self.check_next(TokenType::LParen)
            && !self.check_next(TokenType::LBracket)
        {
            let type_name = self.advance_text()?;
            return Ok(DataType::Custom { name: type_name });
        }

        self.parse_data_type()
    }

    /// Parse DROP FUNCTION statement
    pub(crate) fn parse_drop_function(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Function)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name_ref()?;

        let parameters = if self.match_token(TokenType::LParen) {
            let mut types: Vec<DataType> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    if self.check(TokenType::Table)
                        && self.check_next(TokenType::LParen)
                        && self.tokens.get(self.current + 2).is_some_and(|token| {
                            matches!(token.token_type, TokenType::RParen | TokenType::String)
                        })
                    {
                        return Err(
                            self.parse_error("A TABLE argument names at least one data type")
                        );
                    }
                    types.push(self.parse_data_type()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            Some(types)
        } else {
            None
        };

        let cascade = self.match_token(TokenType::Cascade);
        if !cascade {
            self.match_token(TokenType::Restrict);
        }

        Ok(Expression::DropFunction(Box::new(DropFunction {
            name,
            parameters,
            if_exists,
            cascade,
        })))
    }

    /// Refuses null handling or volatility in a Snowflake Java, Scala or Python procedure, as every receipt does (errno 1003).
    fn refuse_handler_null_or_volatility(&self, language: Option<&str>, found: bool) -> Result<()> {
        let handler = language.is_some_and(|language| {
            ["JAVA", "SCALA", "PYTHON"].contains(&language.to_ascii_uppercase().as_str())
        });
        match self.is_snowflake() && found && handler {
            true => Err(self.parse_error(
                "A Java, Scala or Python procedure takes no null handling or volatility",
            )),
            false => Ok(()),
        }
    }

    /// Parse CREATE PROCEDURE statement
    pub(crate) fn parse_create_procedure(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Expression> {
        let use_proc_keyword = self.peek_text().eq_ignore_ascii_case("PROC");
        self.expect_token(TokenType::Procedure)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        self.check_snowflake_object_name()?;
        let name = self.parse_table_ref()?;

        let (parameters, has_parens) = if self.match_token(TokenType::LParen) {
            let params = self.parse_function_parameters()?;
            self.expect_token(TokenType::RParen)?;
            (params, true)
        } else if self.check(TokenType::Var) && !self.check(TokenType::As) {
            let params = self.parse_tsql_procedure_params()?;
            (params, false)
        } else {
            (Vec::new(), false)
        };

        let mut language = None;
        let mut security = None;
        let mut body = None;
        let mut return_type = None;
        let mut execute_as = None;
        let mut with_options: Vec<String> = Vec::new();
        let mut null_or_volatility = false;

        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if let Some(property) = self.parse_snowflake_routine_property()? {
                null_or_volatility |= matches!(
                    property.to_ascii_uppercase().as_str(),
                    "CALLED ON NULL INPUT"
                        | "RETURNS NULL ON NULL INPUT"
                        | "STRICT"
                        | "VOLATILE"
                        | "IMMUTABLE"
                );
                with_options.push(property);
            } else if self.match_token(TokenType::Returns) {
                self.check_snowflake_table_columns()?;
                return_type = Some(self.parse_data_type()?);
                match self.match_snowflake_return_nullability() {
                    Some(nullability) => with_options.push(nullability),
                    None => {
                        self.match_token(TokenType::Null);
                    }
                }
            } else if self.match_identifier("EXECUTE") || self.match_token(TokenType::Execute) {
                if self.match_token(TokenType::As) {
                    if self.match_identifier("CALLER") {
                        execute_as = Some("CALLER".to_string());
                    } else if self.match_identifier("OWNER") {
                        execute_as = Some("OWNER".to_string());
                    } else if self.match_identifier("SELF") {
                        execute_as = Some("SELF".to_string());
                    } else if self.match_text_seq(&["RESTRICTED", "CALLER"]) {
                        execute_as = Some("RESTRICTED CALLER".to_string());
                    } else if self.is_snowflake() {
                        return Err(self.parse_error("Expected OWNER, CALLER or RESTRICTED CALLER"));
                    }
                }
            } else if self.match_token(TokenType::Language) {
                language = Some(self.expect_identifier_or_keyword()?);
            } else if self.match_identifier("SECURITY") {
                if self.match_identifier("DEFINER") {
                    security = Some(FunctionSecurity::Definer);
                } else if self.match_identifier("INVOKER") {
                    security = Some(FunctionSecurity::Invoker);
                }
            } else if self.match_token(TokenType::With) {
                loop {
                    if self.match_identifier("EXECUTE") || self.match_token(TokenType::Execute) {
                        self.expect_token(TokenType::As)?;
                        if self.check(TokenType::String) {
                            let tok = self.advance()?;
                            with_options.push(format!("EXECUTE AS '{}'", tok.text));
                        } else {
                            let ident = self.expect_identifier_or_keyword()?;
                            with_options.push(format!("EXECUTE AS {}", ident.to_ascii_uppercase()));
                        }
                    } else {
                        let opt = self.expect_identifier_or_keyword()?;
                        with_options.push(opt.to_ascii_uppercase());
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else if self.match_token(TokenType::As) {
                if self.check(TokenType::String) {
                    let tok = self.advance()?;
                    body = Some(FunctionBody::StringLiteral(tok.text.to_string()));
                } else if self.check(TokenType::HeredocString) {
                    let tok = self.advance()?;
                    body = Some(FunctionBody::Block(tok.text.to_string()));
                } else if self.match_token(TokenType::Begin) {
                    let mut statements: Vec<Expression> = Vec::new();
                    while !self.check(TokenType::End) && !self.is_at_end() {
                        while self.match_token(TokenType::Semicolon) {}
                        if self.check(TokenType::End) {
                            break;
                        }
                        statements.push(self.parse_statement()?);
                        self.match_token(TokenType::Semicolon);
                    }
                    self.expect_token(TokenType::End)?;
                    body = Some(FunctionBody::Statements(statements));
                } else if self.check(TokenType::DollarString) {
                    body = Some(FunctionBody::Expression(self.parse_expression()?));
                } else {
                    let stmt = self.parse_statement()?;
                    body = Some(FunctionBody::Expression(stmt));
                }
            } else {
                break;
            }
        }
        self.refuse_handler_null_or_volatility(language.as_deref(), null_or_volatility)?;

        Ok(Expression::CreateProcedure(Box::new(CreateProcedure {
            name,
            parameters,
            body,
            or_replace,
            or_alter,
            if_not_exists,
            temporary: false,
            secure: false,
            language,
            security,
            return_type,
            execute_as,
            with_options,
            has_parens,
            use_proc_keyword,
            returns_table: None,
        })))
    }

    /// Parse DROP PROCEDURE statement
    pub(crate) fn parse_drop_procedure(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Procedure)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name_ref()?;

        let parameters = if self.match_token(TokenType::LParen) {
            let mut types: Vec<DataType> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    types.push(self.parse_data_type()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            Some(types)
        } else {
            None
        };

        let cascade = self.match_token(TokenType::Cascade);
        if !cascade {
            self.match_token(TokenType::Restrict);
        }

        Ok(Expression::DropProcedure(Box::new(DropProcedure {
            name,
            parameters,
            if_exists,
            cascade,
        })))
    }

    /// Parse CREATE SEQUENCE statement
    pub(crate) fn parse_create_sequence(
        &mut self,
        temporary: bool,
        or_replace: bool,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Sequence)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = if self.applies_snowflake_context_rules() {
            self.parse_object_name()?
        } else {
            self.parse_table_ref()?
        };

        let mut seq = CreateSequence {
            name,
            if_not_exists,
            temporary,
            or_replace,
            as_type: None,
            increment: None,
            minvalue: None,
            maxvalue: None,
            start: None,
            cache: None,
            cycle: false,
            owned_by: None,
            owned_by_none: false,
            order: None,
            comment: None,
            sharing: None,
            scale_modifier: None,
            shard_modifier: None,
            property_order: Vec::new(),
        };

        if self.match_token(TokenType::As) {
            seq.as_type = Some(self.parse_data_type()?);
        }

        self.match_token(TokenType::With);

        loop {
            self.match_token(TokenType::Comma);

            if self.is_at_end() || self.check(TokenType::Semicolon) {
                break;
            }

            if self.match_token(TokenType::Increment) || self.match_identifier("INCREMENT") {
                seq = self.parse_sequence_increment(seq)?;
            } else if self.match_token(TokenType::Minvalue) {
                seq.minvalue = Some(SequenceBound::Value(self.parse_signed_integer()?));
                seq.property_order.push(SeqPropKind::Minvalue);
            } else if self.match_keywords(&[TokenType::No, TokenType::Minvalue]) {
                seq.minvalue = Some(SequenceBound::None);
                seq.property_order.push(SeqPropKind::Minvalue);
            } else if self.match_identifier("NOMINVALUE") {
                seq.minvalue = Some(SequenceBound::None);
                seq.property_order.push(SeqPropKind::NoMinvalueWord);
            } else if self.match_token(TokenType::Maxvalue) {
                seq.maxvalue = Some(SequenceBound::Value(self.parse_signed_integer()?));
                seq.property_order.push(SeqPropKind::Maxvalue);
            } else if self.match_keywords(&[TokenType::No, TokenType::Maxvalue]) {
                seq.maxvalue = Some(SequenceBound::None);
                seq.property_order.push(SeqPropKind::Maxvalue);
            } else if self.match_identifier("NOMAXVALUE") {
                seq.maxvalue = Some(SequenceBound::None);
                seq.property_order.push(SeqPropKind::NoMaxvalueWord);
            } else if self.match_token(TokenType::Start) {
                self.match_token(TokenType::With);
                self.match_token(TokenType::Eq);
                seq.start = Some(self.parse_signed_integer()?);
                seq.property_order.push(SeqPropKind::Start);
            } else if self.match_token(TokenType::Cache) {
                seq.cache = Some(self.parse_signed_integer()?);
                seq.property_order.push(SeqPropKind::Cache);
            } else if self.match_identifier("NOCACHE") {
                seq.property_order.push(SeqPropKind::NoCacheWord);
            } else if self.match_token(TokenType::Cycle) {
                seq.cycle = true;
                seq.property_order.push(SeqPropKind::Cycle);
            } else if self.match_token(TokenType::NoCycle) {
                seq.cycle = false;
                seq.property_order.push(SeqPropKind::NoCycleWord);
            } else if self.match_token(TokenType::No) {
                if self.match_token(TokenType::Cycle) {
                    seq.cycle = false;
                    seq.property_order.push(SeqPropKind::NoCycle);
                } else if self.match_token(TokenType::Cache) || self.match_identifier("CACHE") {
                    seq.property_order.push(SeqPropKind::NoCache);
                } else if self.match_token(TokenType::Minvalue) {
                    seq.minvalue = Some(SequenceBound::None);
                    seq.property_order.push(SeqPropKind::Minvalue);
                } else if self.match_token(TokenType::Maxvalue) {
                    seq.maxvalue = Some(SequenceBound::None);
                    seq.property_order.push(SeqPropKind::Maxvalue);
                } else {
                    break;
                }
            } else if self.match_token(TokenType::Owned) {
                seq = self.parse_sequence_owned_by(seq)?;
            } else if self.match_token(TokenType::Order) {
                seq.order = Some(true);
                seq.property_order.push(SeqPropKind::Order);
            } else if self.match_identifier("NOORDER") {
                seq.order = Some(false);
                seq.property_order.push(SeqPropKind::NoOrder);
            } else if self.match_token(TokenType::Comment) || self.match_identifier("COMMENT") {
                self.expect_token(TokenType::Eq)?;
                let comment_val = self.expect_token(TokenType::String)?;
                seq.comment = Some(comment_val.text.to_string());
                seq.property_order.push(SeqPropKind::Comment);
            } else if self.match_identifier("SHARING") {
                self.expect_token(TokenType::Eq)?;
                let val = self.expect_identifier_or_keyword()?;
                seq.sharing = Some(val);
                seq.property_order.push(SeqPropKind::Sharing);
            } else if self.match_identifier("NOKEEP") {
                seq.property_order.push(SeqPropKind::NoKeep);
            } else if self.match_token(TokenType::Keep) || self.match_identifier("KEEP") {
                seq.property_order.push(SeqPropKind::Keep);
            } else if self.match_identifier("SCALE") {
                seq = self.parse_sequence_scale(seq);
            } else if self.match_identifier("NOSCALE") {
                seq.property_order.push(SeqPropKind::NoScale);
            } else if self.match_identifier("SHARD") {
                seq = self.parse_sequence_shard(seq);
            } else if self.match_identifier("NOSHARD") {
                seq.property_order.push(SeqPropKind::NoShard);
            } else if self.match_identifier("SESSION") {
                seq.property_order.push(SeqPropKind::Session);
            } else if self.match_identifier("GLOBAL") {
                seq.property_order.push(SeqPropKind::Global);
            } else {
                break;
            }
        }

        Ok(Expression::CreateSequence(Box::new(seq)))
    }

    fn parse_sequence_increment(&mut self, mut seq: CreateSequence) -> Result<CreateSequence> {
        self.match_token(TokenType::By);
        self.match_token(TokenType::Eq);
        seq.increment = Some(self.parse_signed_integer()?);
        seq.property_order.push(SeqPropKind::Increment);
        Ok(seq)
    }

    fn parse_sequence_owned_by(&mut self, mut seq: CreateSequence) -> Result<CreateSequence> {
        self.expect_token(TokenType::By)?;
        if self.match_identifier("NONE") {
            seq.owned_by = None;
            seq.owned_by_none = true;
        } else {
            seq.owned_by = Some(self.parse_table_ref()?);
        }
        seq.property_order.push(SeqPropKind::OwnedBy);
        Ok(seq)
    }

    fn parse_sequence_scale(&mut self, mut seq: CreateSequence) -> CreateSequence {
        let modifier = if self.match_identifier("EXTEND") {
            "EXTEND".to_string()
        } else if self.match_identifier("NOEXTEND") {
            "NOEXTEND".to_string()
        } else {
            String::new()
        };
        seq.scale_modifier = Some(modifier);
        seq.property_order.push(SeqPropKind::Scale);
        seq
    }

    fn parse_sequence_shard(&mut self, mut seq: CreateSequence) -> CreateSequence {
        let modifier = if self.match_identifier("EXTEND") {
            "EXTEND".to_string()
        } else if self.match_identifier("NOEXTEND") {
            "NOEXTEND".to_string()
        } else {
            String::new()
        };
        seq.shard_modifier = Some(modifier);
        seq.property_order.push(SeqPropKind::Shard);
        seq
    }

    /// Parse a signed integer (positive or negative)
    pub(crate) fn parse_signed_integer(&mut self) -> Result<i64> {
        let negative = self.match_token(TokenType::Dash);
        let tok = self.expect_token(TokenType::Number)?;
        let text = if negative {
            format!("-{}", tok.text)
        } else {
            tok.text.to_string()
        };
        text.parse()
            .map_err(|_| self.parse_error(format!("Invalid integer: {text}")))
    }

    /// Parse DROP SEQUENCE statement
    pub(crate) fn parse_drop_sequence(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Sequence)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name_ref()?;

        let cascade = self.match_token(TokenType::Cascade);
        let restrict = !cascade && self.match_token(TokenType::Restrict);

        Ok(Expression::DropSequence(Box::new(DropSequence {
            name,
            if_exists,
            cascade,
            restrict,
        })))
    }

    /// Parse ALTER SEQUENCE statement
    pub(crate) fn parse_alter_sequence(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Sequence)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_table_ref()?;

        let mut seq = AlterSequence {
            name,
            if_exists,
            increment: None,
            minvalue: None,
            maxvalue: None,
            start: None,
            restart: None,
            cache: None,
            cycle: None,
            owned_by: None,
        };

        if self.applies_snowflake_context_rules()
            && (self.is_at_end() || self.check(TokenType::Semicolon))
        {
            return Err(self.parse_error("Expected an ALTER SEQUENCE action"));
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.match_token(TokenType::Increment) || self.match_identifier("INCREMENT") {
                self.match_token(TokenType::By);
                if self.applies_snowflake_context_rules() {
                    self.match_token(TokenType::Eq);
                }
                seq.increment = Some(self.parse_signed_integer()?);
            } else if self.match_token(TokenType::Minvalue) {
                seq.minvalue = Some(SequenceBound::Value(self.parse_signed_integer()?));
            } else if self.match_keywords(&[TokenType::No, TokenType::Minvalue]) {
                seq.minvalue = Some(SequenceBound::None);
            } else if self.match_token(TokenType::Maxvalue) {
                seq.maxvalue = Some(SequenceBound::Value(self.parse_signed_integer()?));
            } else if self.match_keywords(&[TokenType::No, TokenType::Maxvalue]) {
                seq.maxvalue = Some(SequenceBound::None);
            } else if self.match_token(TokenType::Start) {
                self.match_token(TokenType::With);
                seq.start = Some(self.parse_signed_integer()?);
            } else if self.match_token(TokenType::Restart) {
                if self.match_token(TokenType::With)
                    || self.check(TokenType::Number)
                    || self.check(TokenType::Dash)
                {
                    seq.restart = Some(Some(self.parse_signed_integer()?));
                } else {
                    seq.restart = Some(None);
                }
            } else if self.match_token(TokenType::Cache) {
                seq.cache = Some(self.parse_signed_integer()?);
            } else if self.match_token(TokenType::Cycle) {
                seq.cycle = Some(true);
            } else if self.match_token(TokenType::NoCycle) {
                seq.cycle = Some(false);
            } else if self.match_token(TokenType::Owned) {
                self.expect_token(TokenType::By)?;
                if self.match_identifier("NONE") {
                    seq.owned_by = Some(None);
                } else {
                    seq.owned_by = Some(Some(self.parse_table_ref()?));
                }
            } else {
                break;
            }
        }

        Ok(Expression::AlterSequence(Box::new(seq)))
    }

    /// Parse CREATE TRIGGER statement
    pub(crate) fn parse_create_trigger(
        &mut self,
        or_replace: bool,
        or_alter: bool,
        constraint: bool,
        create_pos: usize,
    ) -> Result<Expression> {
        self.expect_token(TokenType::Trigger)?;

        let name = self.expect_identifier_with_quoted()?;

        if self.check(TokenType::On) && !constraint {
            self.current = create_pos;
            return self.fallback_to_command(create_pos);
        }

        let timing = if self.match_token(TokenType::Before) {
            TriggerTiming::Before
        } else if self.match_token(TokenType::After) {
            TriggerTiming::After
        } else if self.match_token(TokenType::Instead) {
            self.expect_token(TokenType::Of)?;
            TriggerTiming::InsteadOf
        } else {
            self.current = create_pos;
            return self.fallback_to_command(create_pos);
        };

        let events = self.parse_create_trigger_events()?;

        self.expect_token(TokenType::On)?;
        let table = self.parse_table_ref()?;

        let referencing = self.parse_create_trigger_referencing(constraint)?;

        let mut deferrable = None;
        let mut initially_deferred = None;
        if constraint {
            if self.match_identifier("DEFERRABLE") {
                deferrable = Some(true);
            } else if self.match_keywords(&[TokenType::Not, TokenType::Identifier]) {
                deferrable = Some(false);
            }
            if self.match_identifier("INITIALLY") {
                if self.match_identifier("DEFERRED") {
                    initially_deferred = Some(true);
                } else if self.match_identifier("IMMEDIATE") {
                    initially_deferred = Some(false);
                }
            }
        }

        let for_each = if self.match_token(TokenType::For) {
            self.match_token(TokenType::Each);
            if self.match_token(TokenType::Row) {
                Some(TriggerForEach::Row)
            } else if self.match_token(TokenType::Statement) {
                Some(TriggerForEach::Statement)
            } else {
                Some(TriggerForEach::Row)
            }
        } else {
            None
        };

        let (when, when_paren) = if self.match_token(TokenType::When) {
            let has_paren = self.match_token(TokenType::LParen);
            let expr = self.parse_expression()?;
            if has_paren {
                self.expect_token(TokenType::RParen)?;
            }
            (Some(expr), has_paren)
        } else {
            (None, false)
        };

        let body = self.parse_create_trigger_body()?;

        Ok(Expression::CreateTrigger(Box::new(CreateTrigger {
            name,
            table,
            timing,
            events,
            for_each,
            when,
            when_paren,
            body,
            or_replace,
            or_alter,
            constraint,
            deferrable,
            initially_deferred,
            referencing,
        })))
    }

    fn parse_create_trigger_events(&mut self) -> Result<Vec<TriggerEvent>> {
        let mut events: Vec<TriggerEvent> = Vec::new();
        loop {
            if self.match_token(TokenType::Insert) {
                events.push(TriggerEvent::Insert);
            } else if self.match_token(TokenType::Update) {
                if self.match_token(TokenType::Of) {
                    let mut cols: Vec<Identifier> = Vec::new();
                    loop {
                        cols.push(self.parse_unquoted_identifier(Self::expect_identifier)?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    events.push(TriggerEvent::Update(Some(cols)));
                } else {
                    events.push(TriggerEvent::Update(None));
                }
            } else if self.match_token(TokenType::Delete) {
                events.push(TriggerEvent::Delete);
            } else if self.match_token(TokenType::Truncate) {
                events.push(TriggerEvent::Truncate);
            } else {
                break;
            }

            if !self.match_token(TokenType::Or) {
                break;
            }
        }
        Ok(events)
    }

    fn parse_create_trigger_referencing(
        &mut self,
        constraint: bool,
    ) -> Result<Option<TriggerReferencing>> {
        Ok(if !constraint && self.match_token(TokenType::Referencing) {
            let mut ref_clause = TriggerReferencing {
                old_table: None,
                new_table: None,
                old_row: None,
                new_row: None,
            };
            while self.match_token(TokenType::Old) || self.match_token(TokenType::New) {
                let is_old = self.previous().token_type == TokenType::Old;
                let is_table = self.match_token(TokenType::Table);
                let _is_row = !is_table && self.match_token(TokenType::Row);
                self.match_token(TokenType::As);
                let alias = self.parse_unquoted_identifier(Self::expect_identifier)?;

                if is_old {
                    if is_table {
                        ref_clause.old_table = Some(alias);
                    } else {
                        ref_clause.old_row = Some(alias);
                    }
                } else {
                    if is_table {
                        ref_clause.new_table = Some(alias);
                    } else {
                        ref_clause.new_row = Some(alias);
                    }
                }
            }
            Some(ref_clause)
        } else {
            None
        })
    }

    fn parse_create_trigger_body(&mut self) -> Result<TriggerBody> {
        Ok(if self.match_token(TokenType::Execute) {
            self.match_token(TokenType::Function);
            self.match_token(TokenType::Procedure);
            let func_name = self.parse_table_ref()?;
            self.expect_token(TokenType::LParen)?;
            let mut args: Vec<Expression> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    args.push(self.parse_expression()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            TriggerBody::Execute {
                function: Box::new(func_name),
                args,
            }
        } else if self.match_token(TokenType::Begin) {
            let body_start = if !self.is_at_end() {
                self.tokens[self.current].span.start
            } else {
                0
            };
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
            }
            let block_content = if self.source.is_some() {
                let body_end = if self.current > 0 {
                    self.tokens[self.current - 1].span.start
                } else {
                    body_start
                };
                self.source_text_range(body_start, body_end)
                    .trim()
                    .to_string()
            } else {
                String::new()
            };
            TriggerBody::Block(block_content)
        } else {
            return Err(self.parse_error("Expected EXECUTE or BEGIN in trigger body"));
        })
    }

    /// Parse DROP TRIGGER statement
    pub(crate) fn parse_drop_trigger(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Trigger)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_unquoted_identifier(Self::expect_identifier)?;

        let table = if self.match_token(TokenType::On) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        let cascade = self.match_token(TokenType::Cascade);
        if !cascade {
            self.match_token(TokenType::Restrict);
        }

        Ok(Expression::DropTrigger(Box::new(DropTrigger {
            name,
            table,
            if_exists,
            cascade,
        })))
    }

    /// Parse CREATE TYPE statement
    pub(crate) fn parse_create_type(&mut self, or_replace: bool) -> Result<Expression> {
        self.expect_token(TokenType::Type)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        if self.is_snowflake() {
            return self.parse_snowflake_create_type(or_replace, if_not_exists);
        }
        let name = self.parse_table_ref()?;

        if !self.match_token(TokenType::As) {
            return Ok(Expression::CreateType(Box::new(CreateType {
                name,
                definition: TypeDefinition::Base {
                    input: String::new(),
                    output: String::new(),
                    internallength: None,
                },
                if_not_exists,
                or_replace,
                temporary: false,
            })));
        }

        let definition = if self.match_token(TokenType::Enum) {
            self.expect_token(TokenType::LParen)?;
            let mut values: Vec<String> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    let tok = self.expect_token(TokenType::String)?;
                    values.push(tok.text.trim_matches('\'').to_string());
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            TypeDefinition::Enum(values)
        } else if self.match_token(TokenType::LParen) {
            let mut attrs: Vec<TypeAttribute> = Vec::new();
            loop {
                let attr_name = self.parse_unquoted_identifier(Self::expect_identifier)?;
                let data_type = self.parse_data_type()?;
                let collate = if self.match_identifier("COLLATE") {
                    Some(self.parse_unquoted_identifier(Self::expect_identifier)?)
                } else {
                    None
                };
                attrs.push(TypeAttribute {
                    name: attr_name,
                    data_type,
                    collate,
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            TypeDefinition::Composite(attrs)
        } else if self.match_token(TokenType::Range) {
            self.expect_token(TokenType::LParen)?;
            self.match_identifier("SUBTYPE");
            self.match_token(TokenType::Eq);
            let subtype_start = self.current;
            let subtype = self.parse_data_type()?;
            let subtype = if matches!(subtype, DataType::Custom { .. }) {
                DataType::Custom {
                    name: self.tokens_to_sql(subtype_start, self.current),
                }
            } else {
                subtype
            };

            let mut subtype_diff = None;
            let mut canonical = None;

            while self.match_token(TokenType::Comma) {
                if self.match_identifier("SUBTYPE_DIFF") {
                    self.match_token(TokenType::Eq);
                    subtype_diff = Some(self.expect_identifier()?);
                } else if self.match_identifier("CANONICAL") {
                    self.match_token(TokenType::Eq);
                    canonical = Some(self.expect_identifier()?);
                }
            }
            self.expect_token(TokenType::RParen)?;

            TypeDefinition::Range {
                subtype,
                subtype_diff,
                canonical,
            }
        } else {
            return Err(
                self.parse_error("Expected ENUM, composite type definition, or RANGE after AS")
            );
        };

        Ok(Expression::CreateType(Box::new(CreateType {
            name,
            definition,
            if_not_exists,
            or_replace,
            temporary: false,
        })))
    }

    /// Snowflake CREATE TYPE after IF NOT EXISTS: a name, AS, a data type and an optional COMMENT string.
    fn parse_snowflake_create_type(
        &mut self,
        or_replace: bool,
        if_not_exists: bool,
    ) -> Result<Expression> {
        let name = self.parse_object_name()?;
        self.expect_token(TokenType::As)?;
        if self.check(TokenType::String) || self.check(TokenType::Semicolon) || self.is_at_end() {
            return Err(self.parse_error("Expected a data type after AS"));
        }
        let data_type = self.parse_data_type()?;
        let comment = if self.match_token(TokenType::Comment) {
            self.expect_token(TokenType::Eq)?;
            Some(self.expect_string()?)
        } else {
            None
        };
        Ok(Expression::CreateType(Box::new(CreateType {
            name,
            definition: TypeDefinition::Alias { data_type, comment },
            if_not_exists,
            or_replace,
            temporary: false,
        })))
    }

    /// Parse CREATE DOMAIN statement
    pub(crate) fn parse_create_domain(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Domain)?;

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = self.parse_table_ref()?;

        self.expect_token(TokenType::As)?;
        let base_type = self.parse_data_type()?;

        let mut default = None;
        let mut constraints: Vec<DomainConstraint> = Vec::new();

        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.match_token(TokenType::Default) {
                default = Some(self.parse_expression()?);
            } else if self.match_token(TokenType::Constraint) {
                let constr_name = Some(self.parse_unquoted_identifier(Self::expect_identifier)?);
                self.expect_token(TokenType::Check)?;
                self.expect_token(TokenType::LParen)?;
                let check_expr = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                constraints.push(DomainConstraint {
                    name: constr_name,
                    check: check_expr,
                });
            } else if self.match_token(TokenType::Check) {
                self.expect_token(TokenType::LParen)?;
                let check_expr = self.parse_expression()?;
                self.expect_token(TokenType::RParen)?;
                constraints.push(DomainConstraint {
                    name: None,
                    check: check_expr,
                });
            } else if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                constraints.push(DomainConstraint {
                    name: None,
                    check: Expression::IsNull(Box::new(IsNull {
                        this: Expression::Identifier(Identifier::new("VALUE")),
                        not: true,
                        postfix_form: false,
                        span: SourceSpan::default(),
                    })),
                });
            } else {
                break;
            }
        }

        Ok(Expression::CreateType(Box::new(CreateType {
            name,
            definition: TypeDefinition::Domain {
                base_type,
                default: default.map(Box::new),
                constraints,
            },
            if_not_exists,
            or_replace: false,
            temporary: false,
        })))
    }

    /// Parse CREATE STAGE statement (Snowflake)
    pub(crate) fn parse_create_stage(
        &mut self,
        or_replace: bool,
        temporary: bool,
    ) -> Result<Expression> {
        self.check_opaque_statement(true)?;
        self.skip();
        let start = self.current;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let sql = self.tokens_to_sql_stage_format(start, self.current);

        let mut prefix = String::from("CREATE");
        if or_replace {
            prefix.push_str(" OR REPLACE");
        }
        if temporary {
            prefix.push_str(" TEMPORARY");
        }
        prefix.push_str(" STAGE");

        Ok(Expression::Raw(Raw {
            sql: format!("{} {}", prefix, sql),
        }))
    }

    /// Parse CREATE TAG statement (Snowflake)
    pub(crate) fn parse_create_tag(&mut self, or_replace: bool) -> Result<Expression> {
        self.check_opaque_statement(true)?;
        self.skip();
        let start = self.current;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let sql = self.tokens_to_sql(start, self.current);
        let prefix = if or_replace {
            "CREATE OR REPLACE TAG"
        } else {
            "CREATE TAG"
        };
        Ok(Expression::Raw(Raw {
            sql: format!("{} {}", prefix, sql),
        }))
    }

    /// Parse CREATE STREAM statement (Snowflake)
    pub(crate) fn parse_create_stream(&mut self, _or_replace: bool) -> Result<Expression> {
        self.check_opaque_statement(true)?;
        self.skip();
        let start = self.current;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let sql = self.tokens_to_sql(start, self.current);
        Ok(Expression::Raw(Raw {
            sql: format!("CREATE STREAM {}", sql),
        }))
    }

    /// Parses Snowflake CREATE [OR REPLACE] TASK [IF NOT EXISTS] name [...] AS statement.
    pub(crate) fn parse_create_task(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Expression> {
        self.skip();
        let graph = self.is_snowflake()
            && self.check_identifier("GRAPH")
            && self.peek_nth(1).is_some_and(|next| {
                next.token_type == TokenType::If
                    || !matches!(
                        next.token_type,
                        TokenType::As | TokenType::Dot | TokenType::Eq
                    ) && self
                        .peek_nth(2)
                        .is_some_and(|after| after.token_type != TokenType::Eq)
            });
        if graph {
            self.skip();
            return self.parse_snowflake_create_task_graph(or_replace, or_alter);
        }

        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);

        if self.is_snowflake() {
            let start = self.current;
            let target = self.parse_object_name();
            if let (Ok(target), true) = (target, self.check_identifier("CLONE")) {
                let name = [target.catalog, target.schema, Some(target.name)]
                    .into_iter()
                    .flatten()
                    .map(|part| match part.quoted {
                        true => format!("\"{}\"", part.name.replace('"', "\"\"")),
                        false => part.name,
                    })
                    .collect::<Vec<_>>()
                    .join(".");
                self.skip();
                let clone_source = self.parse_object_name()?;
                return Ok(Expression::CreateTask(Box::new(
                    sqlquack_syntax::expressions::statements::models::CreateTask {
                        or_replace,
                        or_alter,
                        if_not_exists,
                        name,
                        properties: String::new(),
                        task_properties: Vec::new(),
                        body: None,
                        clone_source: Some(clone_source),
                        graph: false,
                    },
                )));
            }
            self.current = start;
        }

        if self.is_snowflake() {
            let start = self.current;
            self.skip_snowflake_name()?;
            self.current = start;
        }
        let mut name = String::new();
        if self.check(TokenType::Var) || self.check_keyword() || self.is_identifier_token() {
            name.push_str(&self.advance_text()?);
        }
        while self.check(TokenType::Dot) {
            self.skip();
            name.push('.');
            if self.check(TokenType::Var) || self.check_keyword() || self.is_identifier_token() {
                name.push_str(&self.advance_text()?);
            }
        }

        let props_start = self.current;
        if self.is_snowflake()
            && (self.is_at_end() || self.check(TokenType::Semicolon))
            && !name.eq_ignore_ascii_case("GRAPH")
        {
            return Ok(Expression::CreateTask(Box::new(
                sqlquack_syntax::expressions::statements::models::CreateTask {
                    or_replace,
                    or_alter,
                    if_not_exists,
                    name,
                    properties: String::new(),
                    task_properties: Vec::new(),
                    body: None,
                    clone_source: None,
                    graph: false,
                },
            )));
        }
        let task_properties = match self.is_snowflake() {
            true => self.parse_snowflake_task_properties(false)?,
            false => Vec::new(),
        };
        while !self.is_at_end() && !self.check(TokenType::Semicolon) && !self.check(TokenType::As) {
            self.skip();
        }
        let properties = match self.is_snowflake() {
            true => String::new(),
            false => self.tokens_to_sql(props_start, self.current),
        };

        if !self.match_token(TokenType::As) {
            return Err(self.parse_error("Expected AS keyword in CREATE TASK"));
        }

        let body = self.parse_statement()?;

        Ok(Expression::CreateTask(Box::new(
            sqlquack_syntax::expressions::statements::models::CreateTask {
                or_replace,
                or_alter,
                if_not_exists,
                name,
                properties,
                task_properties,
                body: Some(body),
                clone_source: None,
                graph: false,
            },
        )))
    }

    /// Parse CREATE FILE FORMAT statement (Snowflake)
    pub(crate) fn parse_create_file_format(
        &mut self,
        or_replace: bool,
        temporary: bool,
    ) -> Result<Expression> {
        self.check_opaque_statement(true)?;
        self.skip();
        self.skip();
        let start = self.current;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let sql = self.tokens_to_sql(start, self.current);
        let mut prefix = String::from("CREATE");
        if or_replace {
            prefix.push_str(" OR REPLACE");
        }
        if temporary {
            prefix.push_str(" TEMPORARY");
        }
        prefix.push_str(" FILE FORMAT ");
        prefix.push_str(&sql);
        Ok(Expression::Raw(Raw { sql: prefix }))
    }

    /// Parse DROP TYPE statement
    pub(crate) fn parse_drop_type(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Type)?;

        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_object_name_ref()?;

        let cascade = self.match_token(TokenType::Cascade);
        if !cascade {
            self.match_token(TokenType::Restrict);
        }

        Ok(Expression::DropType(Box::new(DropType {
            name,
            if_exists,
            cascade,
        })))
    }

    pub(crate) fn parse_alter_view_with_modifiers(
        &mut self,
        algorithm: Option<String>,
        definer: Option<String>,
        sql_security: Option<String>,
    ) -> Result<Expression> {
        self.expect_token(TokenType::View)?;

        let name = self.parse_table_ref()?;
        let mut actions: Vec<AlterViewAction> = Vec::new();

        let columns = if self.check(TokenType::LParen) {
            let saved = self.current;
            self.skip();

            let is_column_aliases = self.check(TokenType::Identifier)
                || self.check(TokenType::Var)
                || self.check(TokenType::QuotedIdentifier);

            if is_column_aliases {
                let mut cols: Vec<ViewColumn> = Vec::new();
                loop {
                    let col_name = self.expect_identifier()?;
                    let comment = if self.match_token(TokenType::Comment) {
                        Some(self.expect_string()?)
                    } else {
                        None
                    };
                    cols.push(ViewColumn {
                        name: Identifier::new(col_name),
                        comment,
                        options: Vec::new(),
                    });
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                cols
            } else {
                self.current = saved;
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let with_option = if self.match_token(TokenType::With) {
            let opt = self.expect_identifier_or_keyword()?;
            Some(opt.to_ascii_uppercase())
        } else {
            None
        };

        if let Some(action) = self.parse_data_metric_function_action()? {
            actions.push(AlterViewAction::DataMetricFunction(action));
        } else if self.match_token(TokenType::Rename) {
            self.expect_token(TokenType::To)?;
            actions.push(AlterViewAction::Rename(Box::new(self.parse_table_ref()?)));
        } else if self.match_identifier("OWNER") {
            self.expect_token(TokenType::To)?;
            actions.push(AlterViewAction::OwnerTo(Identifier::new(
                self.expect_identifier()?,
            )));
        } else if self.match_token(TokenType::Set) {
            if self.match_identifier("TBLPROPERTIES") {
                let props = self.parse_tblproperties_key_value_list()?;
                actions.push(AlterViewAction::SetTblproperties(props));
            } else if self.match_token(TokenType::Authorization) {
                let mut auth_text = String::new();
                if self.match_texts(&["ROLE"]) {
                    auth_text.push_str("ROLE ");
                }
                let user = self.expect_identifier()?;
                auth_text.push_str(&user);
                actions.push(AlterViewAction::SetAuthorization(auth_text));
            } else {
                self.expect_token(TokenType::Schema)?;
                actions.push(AlterViewAction::SetSchema(Identifier::new(
                    self.expect_identifier()?,
                )));
            }
        } else if self.match_identifier("UNSET") {
            if !self.match_identifier("TBLPROPERTIES") {
                return Err(self.parse_error("Expected TBLPROPERTIES after UNSET"));
            }
            let keys = self.parse_tblproperties_key_list()?;
            actions.push(AlterViewAction::UnsetTblproperties(keys));
        } else if self.match_token(TokenType::Alter) {
            self.match_token(TokenType::Column);
            let col_name = self.parse_unquoted_identifier(Self::expect_identifier)?;
            let action = self.parse_alter_column_action()?;
            actions.push(AlterViewAction::AlterColumn {
                name: col_name,
                action: Box::new(action),
            });
        } else if self.match_token(TokenType::As) {
            let query = self.parse_statement()?;
            actions.push(AlterViewAction::AsSelect(Box::new(query)));
        }

        Ok(Expression::AlterView(Box::new(AlterView {
            name,
            actions,
            algorithm,
            definer,
            sql_security,
            with_option,
            columns,
        })))
    }

    /// Parse TBLPROPERTIES key-value list: ('key1'='value1', 'key2'='value2', ...)
    pub(crate) fn parse_tblproperties_key_value_list(&mut self) -> Result<Vec<(String, String)>> {
        self.expect_token(TokenType::LParen)?;
        let mut props: Vec<(String, String)> = Vec::new();
        loop {
            let key = self.expect_string()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.expect_string()?;
            props.push((key, value));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(props)
    }

    /// Parse TBLPROPERTIES key list (for UNSET): ('key1', 'key2', ...)
    pub(crate) fn parse_tblproperties_key_list(&mut self) -> Result<Vec<String>> {
        self.expect_token(TokenType::LParen)?;
        let mut keys: Vec<String> = Vec::new();
        loop {
            let key = self.expect_string()?;
            keys.push(key);
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(keys)
    }

    /// Parse ALTER INDEX statement
    pub(crate) fn parse_alter_index(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Index)?;

        let name = self.expect_identifier_or_keyword_with_quoted()?;

        let table = if self.match_token(TokenType::On) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        let mut actions: Vec<AlterIndexAction> = Vec::new();

        if self.match_token(TokenType::Rename) {
            self.expect_token(TokenType::To)?;
            actions.push(AlterIndexAction::Rename(
                self.expect_identifier_or_keyword_with_quoted()?,
            ));
        } else if self.match_token(TokenType::Set) {
            self.match_identifier("TABLESPACE");
            actions.push(AlterIndexAction::SetTablespace(
                self.expect_identifier_or_keyword_with_quoted()?,
            ));
        } else if self.match_identifier("VISIBLE") {
            actions.push(AlterIndexAction::Visible(true));
        } else if self.match_identifier("INVISIBLE") {
            actions.push(AlterIndexAction::Visible(false));
        }

        Ok(Expression::AlterIndex(Box::new(AlterIndex {
            name,
            table,
            actions,
        })))
    }

    /// Parses an expression; assignment (:=) binds looser than OR.
    pub(crate) fn parse_expression(&mut self) -> Result<Expression> {
        let start = self.current;
        let outer_path = std::mem::take(&mut self.parsing_duckdb_json_path);
        let result = self.with_recursive_stack(|parser| parser.parse_expression_inner());
        self.parsing_duckdb_json_path = outer_path;
        Ok(self.spanned(start, result?))
    }

    pub(crate) fn parse_expression_inner(&mut self) -> Result<Expression> {
        let mut left = self.parse_duckdb_json_arrow()?;

        while self.match_token(TokenType::ColonEq) {
            let right = self.parse_duckdb_json_arrow()?;
            left = Expression::PropertyEQ(Box::new(BinaryOp::new(left, right)));
        }

        Ok(left)
    }

    pub(crate) fn parse_duckdb_json_arrow(&mut self) -> Result<Expression> {
        let expression_start = self.current;
        let mut expression = self.parse_or()?;
        if self.config.dialect == Some(sqlquack_syntax::tokens::models::DialectType::DuckDB) {
            while self.match_token(TokenType::Arrow) {
                let outer_path = std::mem::replace(&mut self.parsing_duckdb_json_path, true);
                let path = self.parse_or();
                self.parsing_duckdb_json_path = outer_path;
                expression = Expression::JsonExtract(Box::new(JsonExtractFunc {
                    options: None,
                    source_dialect: None,
                    this: expression,
                    path: path?,
                    returning: None,
                    arrow_syntax: true,
                    hash_arrow_syntax: false,
                    wrapper_option: None,
                    quotes_option: None,
                    on_scalar_string: false,
                    on_error: None,
                }));
                if self.check(TokenType::DArrow) {
                    expression = self.parse_duckdb_concat_tail(expression)?;
                    expression = self.parse_shift_tail(expression, expression_start)?;
                    expression = self.parse_bitwise_and_tail(expression)?;
                    expression = self.parse_bitwise_xor_tail(expression)?;
                    expression = self.parse_bitwise_or_tail(expression)?;
                    expression = self.parse_comparison_tail(expression, Vec::new())?;
                    expression = self.parse_and_tail(expression)?;
                    expression = self.parse_xor_tail(expression)?;
                    expression = self.parse_or_tail(expression)?;
                }
            }
        }
        Ok(expression)
    }

    /// Parse OR expressions
    #[inline(always)]
    pub(crate) fn parse_or(&mut self) -> Result<Expression> {
        let start = self.current;
        let left = self.parse_xor()?;
        let expr = self.parse_or_tail(left)?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn parse_or_tail(&mut self, mut left: Expression) -> Result<Expression> {
        while self.check(TokenType::Or) {
            let mut all_comments = self.previous_trailing_comments().to_vec();
            all_comments.extend_from_slice(self.current_leading_comments());
            self.skip();
            all_comments.extend_from_slice(self.previous_trailing_comments());
            if !all_comments.is_empty() {
                Self::clear_rightmost_trailing_comments(&mut left);
            }
            all_comments.retain(|c| !c.trim().is_empty());
            let mut left_comments: Vec<String> = Vec::new();
            let mut operator_comments: Vec<String> = Vec::new();
            for comment in all_comments {
                if comment.starts_with("/*") {
                    left_comments.push(comment);
                } else {
                    operator_comments.push(comment);
                }
            }
            let mut right = self.parse_xor()?;
            if !self.pending_leading_comments.is_empty() {
                let pending = std::mem::take(&mut self.pending_leading_comments);
                right = Expression::Annotated(Box::new(Annotated {
                    this: right,
                    trailing_comments: pending,
                    type_assertion: None,
                    type_like: None,
                }));
            }
            left = Expression::Or(Box::new(BinaryOp {
                left,
                right,
                left_comments,
                operator_comments,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }));
        }

        Ok(maybe_rebalance_boolean_chain(left, false))
    }

    /// Parse XOR expressions (MySQL logical XOR)
    #[inline(always)]
    pub(crate) fn parse_xor(&mut self) -> Result<Expression> {
        let left = self.parse_and()?;
        self.parse_xor_tail(left)
    }

    #[inline(never)]
    pub(crate) fn parse_xor_tail(&mut self, mut left: Expression) -> Result<Expression> {
        while self.match_token(TokenType::Xor) {
            let right = self.parse_and()?;
            left = Expression::Xor(Box::new(Xor {
                this: Some(Box::new(left)),
                expression: Some(Box::new(right)),
                expressions: Vec::new(),
            }));
        }

        Ok(left)
    }

    /// Parse AND expressions
    #[inline(always)]
    pub(crate) fn parse_and(&mut self) -> Result<Expression> {
        let start = self.current;
        let left = self.parse_not()?;
        let expr = self.parse_and_tail(left)?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn parse_and_tail(&mut self, mut left: Expression) -> Result<Expression> {
        while self.check(TokenType::And) {
            let mut all_comments = self.previous_trailing_comments().to_vec();
            all_comments.extend_from_slice(self.current_leading_comments());
            self.skip();
            all_comments.extend_from_slice(self.previous_trailing_comments());
            if !all_comments.is_empty() {
                Self::clear_rightmost_trailing_comments(&mut left);
            }
            all_comments.retain(|c| !c.trim().is_empty());
            let mut left_comments: Vec<String> = Vec::new();
            let mut operator_comments: Vec<String> = Vec::new();
            for comment in all_comments {
                if comment.starts_with("/*") {
                    left_comments.push(comment);
                } else {
                    operator_comments.push(comment);
                }
            }
            let mut right = self.parse_not()?;
            if !self.pending_leading_comments.is_empty() {
                let pending = std::mem::take(&mut self.pending_leading_comments);
                right = Expression::Annotated(Box::new(Annotated {
                    this: right,
                    trailing_comments: pending,
                    type_assertion: None,
                    type_like: None,
                }));
            }
            left = Expression::And(Box::new(BinaryOp {
                left,
                right,
                left_comments,
                operator_comments,
                trailing_comments: Vec::new(),
                inferred_type: None,
                span: SourceSpan::default(),
            }));
        }

        Ok(maybe_rebalance_boolean_chain(left, true))
    }

    /// Parse NOT expressions
    pub(crate) fn parse_not(&mut self) -> Result<Expression> {
        if !self.check(TokenType::Not) {
            return self.parse_comparison();
        }

        let start = self.current;
        let expr = self.parse_not_prefixes()?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn parse_not_prefixes(&mut self) -> Result<Expression> {
        let mut starts: Vec<usize> = Vec::new();
        while self.check(TokenType::Not) {
            let _scope = self.enter_parser_depth(starts.len() + 1)?;
            starts.push(self.current);
            self.skip();
        }
        let _scope = self.enter_parser_depth(starts.len())?;
        let mut expr = self.parse_comparison()?;
        for _ in starts {
            expr = Expression::Not(Box::new(UnaryOp::new(expr)));
        }
        Ok(expr)
    }

    /// Parse comparison expressions
    #[inline(always)]
    pub(crate) fn parse_comparison(&mut self) -> Result<Expression> {
        let start = self.current;
        let pre_left_comments = self.current_leading_comments().to_vec();
        let left = self.parse_bitwise_or()?;
        let expr = self.parse_comparison_tail(left, pre_left_comments)?;
        Ok(self.spanned(start, expr))
    }
}
