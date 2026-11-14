//! Snowflake's rules on COMMENT ON names and values, stage commands, task properties, DEFINE and class instance method calls.

use crate::parser::_helpers::ddl::snowflake_alter::named_property;
use crate::parser::_helpers::ddl::snowflake_alter::parameter_property;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::EXECUTE_DCM_PROJECT;
use crate::parser::constants::SNOWFLAKE_DEFINE_OPAQUE_KINDS;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::operators::models::Call;
use sqlquack_syntax::expressions::operators::models::NamedArgSeparator;
use sqlquack_syntax::expressions::operators::models::NamedArgument;
use sqlquack_syntax::expressions::operators::models::Parameter;
use sqlquack_syntax::expressions::operators::models::ParameterStyle;
use sqlquack_syntax::expressions::schema_objects::models::ExecuteObject;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::statements::models::CreateTask;
use sqlquack_syntax::expressions::statements::models::ExecuteImmediate;
use sqlquack_syntax::expressions::statements::models::ExecuteUsingPolicyContext;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;
use sqlquack_syntax::traversal::types::ExpressionWalk;

impl Parser {
    /// Snowflake's `IMMEDIATE <source> [ USING ( <variable> [ , ... ] ) ]` or `IMMEDIATE FROM <location> [ USING ( <name> => <value> [ , ... ] ) ] [ DRY_RUN = { TRUE | FALSE } ]`, read after EXECUTE up to the statement end or a `)` closing an enclosing assignment.
    pub(crate) fn parse_execute_immediate(&mut self) -> Result<Expression> {
        self.skip();
        let from = self.match_token(TokenType::From);
        let source = match from {
            true if !self.check(TokenType::String) && !self.is_stage_path() => {
                return Err(self.parse_error("Expected a stage path or a string"));
            }
            true => self.parse_location_value()?,
            false => self.parse_execute_source()?,
        };
        let mut using: Vec<Expression> = Vec::new();
        if self.match_token(TokenType::Using) {
            self.expect_token(TokenType::LParen)?;
            loop {
                using.push(match from {
                    true => self.parse_execute_argument()?,
                    false => self.parse_expression()?,
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        let mut dry_run = None;
        if from && self.match_identifier("DRY_RUN") {
            self.expect_token(TokenType::Eq)?;
            dry_run = Some(match () {
                () if self.match_token(TokenType::True) => true,
                () if self.match_token(TokenType::False) => false,
                () => return Err(self.parse_error("Expected TRUE or FALSE")),
            });
        }
        if !self.is_statement_end_at(0) && !self.check(TokenType::RParen) {
            return Err(self.parse_error("Expected the end of EXECUTE IMMEDIATE"));
        }
        Ok(Expression::ExecuteImmediate(Box::new(ExecuteImmediate {
            source,
            from,
            using,
            dry_run,
        })))
    }

    /// The text EXECUTE IMMEDIATE runs: operands joined by `||`, left to right.
    fn parse_execute_source(&mut self) -> Result<Expression> {
        let mut source = self.parse_execute_operand()?;
        while self.match_token(TokenType::DPipe) {
            let right = self.parse_execute_operand()?;
            source = Expression::Concat(Box::new(BinaryOp::new(source, right)));
        }
        Ok(source)
    }

    /// One part of the text EXECUTE IMMEDIATE runs: a string, a `$session` or `:bound` variable, or a variable's name.
    fn parse_execute_operand(&mut self) -> Result<Expression> {
        let style = match self.peek().token_type {
            TokenType::Dollar => Some(ParameterStyle::Dollar),
            TokenType::Colon => Some(ParameterStyle::Colon),
            _ => None,
        };
        if let Some(style) = style {
            let end = self.peek().span.end;
            self.skip();
            if !self.is_identifier_token() || self.peek().span.start != end {
                return Err(self.parse_error("Expected a variable's name"));
            }
            return Ok(Expression::Parameter(Box::new(Parameter {
                name: Some(self.advance_text()?),
                index: None,
                style,
                quoted: false,
                string_quoted: false,
                expression: None,
                span: SourceSpan::default(),
            })));
        }
        if self.check(TokenType::String)
            || self.check(TokenType::DollarString)
            || (self.is_identifier_token()
                && !self.is_snowflake_reserved_keyword()
                && !self.peek_text().starts_with('@'))
        {
            let start = self.current;
            let operand = self.parse_primary()?;
            if self.current == start + 1 {
                return Ok(operand);
            }
        }
        Err(self.parse_error("Expected the statement to execute"))
    }

    /// `<name> => <value>`, one argument of a file EXECUTE IMMEDIATE FROM runs.
    fn parse_execute_argument(&mut self) -> Result<Expression> {
        let token = self.peek().span;
        let name = self.expect_identifier_or_keyword()?;
        self.expect_token(TokenType::FArrow)?;
        Ok(Expression::NamedArgument(Box::new(NamedArgument {
            name: Identifier::new(name).with_span(token),
            value: self.parse_expression()?,
            separator: NamedArgSeparator::DArrow,
        })))
    }

    /// In Snowflake, refuse an unquoted reserved keyword in any dotted part of a COMMENT ON name.
    pub(super) fn check_snowflake_comment_name(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let mut offset = 0;
        while let Some(token) = self.tokens.get(self.current + offset) {
            if token.token_type != TokenType::QuotedIdentifier
                && SNOWFLAKE_RESERVED_KEYWORDS
                    .iter()
                    .any(|keyword| token.text.eq_ignore_ascii_case(keyword))
            {
                return Err(self.parse_error("Expected an object name"));
            }
            match self.tokens.get(self.current + offset + 1) {
                Some(next) if next.token_type == TokenType::Dot => offset += 2,
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    /// A called object's dotted name; the middle part `<database>..<name>` leaves out is a null entry.
    pub(crate) fn parse_call_name(&mut self) -> Result<Vec<Option<Identifier>>> {
        let mut name = vec![Some(self.expect_identifier_or_keyword_with_quoted()?)];
        while self.match_token(TokenType::Dot) {
            if name.len() == 1 && self.match_token(TokenType::Dot) {
                name.push(None);
            }
            name.push(Some(self.expect_identifier_or_keyword_with_quoted()?));
        }
        for part in name.iter().flatten() {
            self.reject_reserved_name_part(part)?;
        }
        Ok(name)
    }

    /// Parse a Snowflake class instance method call, `<instance>!<method>( [ <arg> , ... ] )`, after CALL; None for any other CALL.
    pub(super) fn parse_snowflake_method_call(&mut self) -> Result<Option<Expression>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        if self.check(TokenType::Exclamation) {
            return Err(self.parse_error("Expected an instance name before !"));
        }
        let token_type = |offset: usize| {
            self.tokens
                .get(self.current + offset)
                .map(|token| token.token_type)
        };
        let mut offset = 1;
        while token_type(offset) == Some(TokenType::Dot) {
            offset += if token_type(offset + 1) == Some(TokenType::Dot) {
                3
            } else {
                2
            };
        }
        if token_type(offset) != Some(TokenType::Exclamation) {
            return Ok(None);
        }
        let name = self.parse_call_name()?;
        self.expect_token(TokenType::Exclamation)?;
        if !self.is_identifier_token() {
            return Err(self.parse_error("Expected a method name after !"));
        }
        let member = self.expect_identifier_or_keyword_with_quoted()?;
        self.expect_token(TokenType::LParen)?;
        let arguments = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::Call(Box::new(Call {
            source_dialect: None,
            name,
            member: Some(member),
            arguments,
            asynchronous: false,
            hints: Vec::new(),
            with: None,
            into: None,
        }))))
    }

    /// The rest of Snowflake `EXECUTE USING POLICY_CONTEXT ( <name> => <value> , ... ) AS <query>`.
    pub(super) fn parse_execute_using_policy_context(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let arguments = self.parse_function_arguments()?;
        if arguments
            .iter()
            .any(|argument| !matches!(argument, Expression::NamedArgument(_)))
        {
            return Err(self.parse_error("POLICY_CONTEXT takes only named arguments"));
        }
        self.expect_token(TokenType::RParen)?;
        self.expect_token(TokenType::As)?;
        if !(self.check(TokenType::Select)
            || self.check(TokenType::With)
            || self.check(TokenType::LParen))
        {
            return Err(self.parse_error("Expected a query after POLICY_CONTEXT ( ... ) AS"));
        }
        let query = self.parse_statement()?;
        Ok(Expression::ExecuteUsingPolicyContext(Box::new(
            ExecuteUsingPolicyContext { arguments, query },
        )))
    }

    /// Snowflake `EXECUTE ALERT | EXPERIMENT | TASK [ GRAPH ] | DCM PROJECT <name>` with its action, or none.
    pub(super) fn parse_snowflake_execute_object(&mut self) -> Result<Option<Expression>> {
        let start = self.current;
        let kind = if self.match_text_seq(&["ALERT"]) {
            "ALERT"
        } else if self.match_text_seq(&["DCM", "PROJECT"]) {
            EXECUTE_DCM_PROJECT
        } else if self.check_text_seq(&["TASK", "GRAPH"])
            && !matches!(
                self.text_at(2).as_deref(),
                None | Some(";" | "RETRY" | "USING")
            )
        {
            self.skip();
            self.skip();
            "TASK GRAPH"
        } else if self.match_text_seq(&["TASK"]) {
            "TASK"
        } else if self.match_text_seq(&["EXPERIMENT"]) {
            "EXPERIMENT"
        } else {
            return self.parse_snowflake_execute_job();
        };
        let name = match kind {
            "EXPERIMENT" => self.parse_object_name()?,
            _ => self.parse_object_name_ref()?,
        };
        let mut node = ExecuteObject {
            kind: kind.to_owned(),
            name,
            action: None,
            group: None,
            alias: None,
            config: None,
            properties: Vec::new(),
        };
        if kind.starts_with("TASK") {
            if self.match_text_seq(&["RETRY", "LAST"]) {
                node.action = Some("RETRY LAST".to_owned());
            } else if self.match_text_seq(&["RETRY", "GRAPH", "RUN", "GROUP"]) {
                node.action = Some("RETRY GRAPH RUN".to_owned());
                if !self.check(TokenType::String) {
                    return Err(self.parse_error("Expected the graph run group as a string"));
                }
                node.group = Some(self.parse_primary()?);
            } else if self.match_text_seq(&["USING", "CONFIG"]) {
                self.expect_token(TokenType::Eq)?;
                node.config = Some(self.parse_primary()?);
            }
        } else if kind == EXECUTE_DCM_PROJECT {
            if self.match_text_seq(&["PURGE"]) {
                node.action = Some("PURGE".to_owned());
                if self.match_token(TokenType::As) {
                    node.alias = Some(self.parse_simple_name()?);
                }
            } else if self.match_text_seq(&["TEST", "ALL"]) {
                node.action = Some("TEST ALL".to_owned());
            } else {
                return match self.parse_dcm_project_run(node)? {
                    Some(run) => Ok(Some(Expression::ExecuteObject(Box::new(run)))),
                    None => {
                        self.current = start;
                        Ok(None)
                    }
                };
            }
        }
        Ok(Some(Expression::ExecuteObject(Box::new(node))))
    }

    /// The PLAN [ DELTA ], DEPLOY or PREVIEW of `EXECUTE DCM PROJECT <name>` with its source and options; false for any other action. Snowflake refuses the page's parenthesised USING values, so USING takes a configuration only.
    fn parse_dcm_project_run(&mut self, mut node: ExecuteObject) -> Result<Option<ExecuteObject>> {
        let action = if self.match_text_seq(&["PLAN", "DELTA"]) {
            "PLAN DELTA"
        } else if self.match_text_seq(&["PLAN"]) {
            "PLAN"
        } else if self.match_text_seq(&["DEPLOY"]) {
            "DEPLOY"
        } else if self.match_text_seq(&["PREVIEW"]) {
            "PREVIEW"
        } else {
            return Ok(None);
        };
        node.action = Some(action.to_owned());
        match action {
            "PREVIEW" => {
                let table = Expression::Table(Box::new(self.parse_object_name()?));
                node.properties.push(property("PREVIEW", None, Some(table)));
                self.expect_alter_words(&[&["USING", "CONFIGURATION"]])?;
                let config = Expression::Identifier(self.parse_simple_name()?);
                node.properties
                    .push(property("USING CONFIGURATION", None, Some(config)));
                node.properties.extend(self.parse_dcm_environment()?);
                self.expect_token(TokenType::From)?;
                node.properties
                    .push(property("FROM", None, Some(self.parse_string_value()?)));
                if self.match_token(TokenType::Limit) {
                    if !self.check(TokenType::Number) {
                        return Err(self.parse_error("Expected a number of rows after LIMIT"));
                    }
                    node.properties
                        .push(property("LIMIT", None, Some(self.parse_primary()?)));
                }
                return Ok(Some(node));
            }
            "DEPLOY" if self.match_token(TokenType::As) => {
                node.alias = Some(self.parse_simple_name()?);
            }
            _ => {}
        }
        if self.match_text_seq(&["FROM", "DEPLOYMENT"]) {
            let deployment = Expression::Identifier(self.parse_simple_name()?);
            node.properties
                .push(property("FROM DEPLOYMENT", None, Some(deployment)));
        } else {
            if self.match_token(TokenType::Using) {
                self.expect_alter_words(&[&["CONFIGURATION"]])?;
                let config = Expression::Identifier(self.parse_simple_name()?);
                node.properties
                    .push(property("USING CONFIGURATION", None, Some(config)));
            }
            node.properties.extend(self.parse_dcm_environment()?);
            self.expect_token(TokenType::From)?;
            node.properties
                .push(property("FROM", None, Some(self.parse_string_value()?)));
        }
        if action.starts_with("PLAN") && self.match_text_seq(&["OUTPUT_PATH"]) {
            node.properties.push(property(
                "OUTPUT_PATH",
                None,
                Some(self.parse_string_value()?),
            ));
        }
        Ok(Some(node))
    }

    /// `[ ENVIRONMENT ( <bind_variable> ) ]` of a DCM project run.
    fn parse_dcm_environment(&mut self) -> Result<Option<ObjectProperty>> {
        if !self.match_text_seq(&["ENVIRONMENT"]) {
            return Ok(None);
        }
        let values = self.parse_dcm_values()?;
        Ok(Some(property("ENVIRONMENT", None, Some(values))))
    }

    /// `( <expr> [ , ... ] )`, with at least one expression.
    fn parse_dcm_values(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions = vec![self.parse_expression()?];
        while self.match_token(TokenType::Comma) {
            expressions.push(self.parse_expression()?);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }
}

impl Parser {
    /// Refuses a Snowflake GET/LIST/REMOVE lacking a stage, with a literal stage part, an empty option or GET's `file://` target.
    pub(crate) fn check_snowflake_stage_command(&self, needs_target: bool) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let rest = &self.tokens[(self.current + 1).min(self.tokens.len())..];
        let end: usize = rest
            .iter()
            .position(|token| token.token_type == TokenType::Semicolon)
            .unwrap_or(rest.len());
        let tokens = &rest[..end];
        let stage: bool = tokens
            .first()
            .is_some_and(|token| token.token_type == TokenType::DAt || token.text.starts_with('@'));
        if !stage {
            return Err(self.parse_error("Expected a stage after the command"));
        }
        let quoted_part: bool = tokens.windows(2).any(|pair| {
            pair[1].token_type == TokenType::String
                && matches!(
                    pair[0].token_type,
                    TokenType::DAt | TokenType::Percent | TokenType::Mod | TokenType::Dot
                )
        });
        if quoted_part {
            return Err(self.parse_error("A stage name part cannot be a string literal"));
        }
        if tokens.last().map(|token| token.token_type) == Some(TokenType::Eq) {
            return Err(self.parse_error("Expected a value after ="));
        }
        let target: bool = tokens[1..].iter().any(|token| {
            token.token_type == TokenType::String
                || token.text.to_ascii_lowercase().starts_with("file")
        });
        if needs_target && !target {
            return Err(self.parse_error("Expected a file:// target after the stage"));
        }
        self.check_snowflake_stage_words(tokens, if needs_target { 2 } else { 1 })
    }

    /// Refuses a quoted name with a space in an unquoted location, and a word after the locations that is no option.
    pub(crate) fn check_snowflake_stage_words(
        &self,
        tokens: &[sqlquack_syntax::tokens::models::ParserToken],
        locations: usize,
    ) -> Result<()> {
        let quoted_space = tokens.iter().any(|token| {
            token.token_type == TokenType::QuotedIdentifier
                && token.text.chars().any(char::is_whitespace)
        });
        if quoted_space {
            return Err(
                self.parse_error("A quoted name with a space needs the whole location quoted")
            );
        }
        let mut words: Vec<&[sqlquack_syntax::tokens::models::ParserToken]> = Vec::new();
        let mut start = 0;
        for index in 1..=tokens.len() {
            if index == tokens.len() || tokens[index - 1].span.end != tokens[index].span.start {
                words.push(&tokens[start..index]);
                start = index;
            }
        }
        let mut after_eq = words.len() < locations;
        for word in words.iter().skip(locations) {
            let first = word[0].token_type;
            let option_name = matches!(
                first,
                TokenType::Var | TokenType::Identifier | TokenType::Eq
            ) || first.is_keyword();
            if !after_eq && !option_name {
                return Err(self.parse_error("Expected an option after the location"));
            }
            after_eq = word[word.len() - 1].token_type == TokenType::Eq;
        }
        Ok(())
    }

    /// Refuses a Snowflake PUT whose file:// source is missing, split by a space, or not followed by the stage.
    pub(crate) fn check_snowflake_put(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let rest = &self.tokens[self.current.min(self.tokens.len())..];
        let end: usize = rest
            .iter()
            .position(|token| token.token_type == TokenType::Semicolon)
            .unwrap_or(rest.len());
        let stage_first = rest
            .first()
            .is_none_or(|token| token.token_type == TokenType::DAt || token.text.starts_with('@'));
        let source_end = rest[..end]
            .windows(2)
            .position(|pair| pair[0].span.end != pair[1].span.start)
            .map_or(end, |index| index + 1);
        let stage_second = rest.get(source_end).is_some_and(|token| {
            matches!(
                token.token_type,
                TokenType::DAt | TokenType::String | TokenType::Parameter
            ) || token.text.starts_with('@')
        });
        if stage_first || !stage_second {
            return Err(self.parse_error("Expected a file:// source and then the stage"));
        }
        self.check_snowflake_stage_words(&rest[..end], 2)
    }

    /// Refuses a Snowflake DELETE without FROM, or whose table is a reserved word.
    pub(crate) fn check_snowflake_delete_head(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let reserved = self.tokens.get(self.current + 1).is_some_and(|token| {
            token.token_type != TokenType::QuotedIdentifier
                && crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS
                    .iter()
                    .any(|keyword| token.text.eq_ignore_ascii_case(keyword))
        });
        if !self.check(TokenType::From) || reserved {
            return Err(self.parse_error("Expected FROM and a table name after DELETE"));
        }
        Ok(())
    }

    /// Snowflake DCM `DEFINE`, refused for kinds kept as opaque text or not yet checked closely; a database's or schema's properties are checked word by word before they are kept as text.
    pub(crate) fn parse_snowflake_define(&mut self) -> Result<Expression> {
        let start = self.current;
        self.skip();
        let opaque_kind = SNOWFLAKE_DEFINE_OPAQUE_KINDS
            .iter()
            .any(|words| self.check_text_seq(words));
        self.current = start;
        let definition = self.parse_create()?;
        if let (true, Expression::Raw(raw)) = (opaque_kind, &definition) {
            let text = raw.sql.strip_prefix("CREATE").unwrap_or(&raw.sql);
            return Ok(Expression::Raw(Raw {
                sql: format!("DEFINE{text}"),
            }));
        }
        if matches!(&definition, Expression::CreateObject(create) if matches!(create.kind.as_str(), "FILE FORMAT") && create.modifier.is_some())
        {
            return Err(self.parse_error("DEFINE FILE FORMAT takes no TEMPORARY or VOLATILE"));
        }
        let unchecked_kind = matches!(definition, Expression::CreateTable(_));
        let mut unchecked = definition.clone();
        match &mut unchecked {
            Expression::CreateDatabase(database) => database.properties.clear(),
            Expression::CreateSchema(schema) => schema.properties.clear(),
            _ => {}
        }
        if unchecked_kind
            || unchecked
                .contains(|node| matches!(node, Expression::Command(_) | Expression::Raw(_)))
        {
            return Err(self.parse_error("Unsupported DEFINE object definition"));
        }
        Ok(definition)
    }

    /// Reads Snowflake CREATE TASK properties up to AS, refusing items that are not tags, contacts, assignments or clauses.
    pub(crate) fn check_snowflake_task_properties(&mut self, graph: bool) -> Result<()> {
        let mut executes_as = false;
        while !(graph && (self.is_at_end() || self.check(TokenType::Semicolon)))
            && !(!graph && self.check(TokenType::As))
        {
            self.match_token(TokenType::With);
            if self.match_identifier("TAG") {
                self.skip_snowflake_task_assignments(
                    |parser| {
                        if parser.match_token(TokenType::String) {
                            return Ok(());
                        }
                        parser.skip_snowflake_name()
                    },
                    false,
                )?;
            } else if self.match_identifier("CONTACT") {
                self.skip_snowflake_task_assignments(Self::skip_snowflake_name, true)?;
            } else if self.match_identifier("TASK_DEFAULTS") {
                self.skip_snowflake_task_assignments(Self::skip_snowflake_task_value, false)?;
            } else if self.match_token(TokenType::After) {
                if executes_as {
                    return Err(self.parse_error("AFTER comes before EXECUTE AS USER"));
                }
                self.skip_snowflake_name()?;
                while self.match_token(TokenType::Comma) {
                    self.skip_snowflake_name()?;
                }
            } else if self.match_text_seq(&["EXECUTE", "AS", "USER"]) {
                executes_as = true;
                self.skip_snowflake_name()?;
            } else if self.match_token(TokenType::When) {
                if self.check(TokenType::As) {
                    return Err(self.parse_error("Expected a condition after WHEN"));
                }
                self.parse_expression()?;
            } else if self.match_identifier("STARTED") || self.match_identifier("SUSPENDED") {
                continue;
            } else if self.check_next(TokenType::Eq) {
                self.skip_snowflake_name()?;
                self.skip();
                self.skip_snowflake_task_value()?;
                self.match_token(TokenType::Comma);
            } else {
                return Err(self.parse_error("Expected a task property or AS"));
            }
        }
        Ok(())
    }

    /// The CREATE TASK properties that `check_snowflake_task_properties` accepts, one by one.
    pub(crate) fn parse_snowflake_task_properties(
        &mut self,
        graph: bool,
    ) -> Result<Vec<ObjectProperty>> {
        let start = self.current;
        self.check_snowflake_task_properties(graph)?;
        let end = self.current;
        self.current = start;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while self.current < end {
            self.match_token(TokenType::With);
            if let Some(kind) = ["TAG", "CONTACT", "TASK_DEFAULTS"]
                .into_iter()
                .find(|kind| self.match_identifier(kind))
            {
                self.expect_token(TokenType::LParen)?;
                loop {
                    let name = self.parse_contact_purpose(Self::parse_simple_qualified_name)?;
                    self.expect_token(TokenType::Eq)?;
                    let value = self.parse_snowflake_task_value()?;
                    properties.push(named_property(kind, name, Some(value)));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
            } else if self.match_token(TokenType::After) {
                let mut tasks = vec![Expression::Table(Box::new(self.parse_object_name()?))];
                while self.match_token(TokenType::Comma) {
                    tasks.push(Expression::Table(Box::new(self.parse_object_name()?)));
                }
                let tasks = Expression::Tuple(Box::new(Tuple { expressions: tasks }));
                properties.push(property("AFTER", None, Some(tasks)));
            } else if self.match_text_seq(&["EXECUTE", "AS", "USER"]) {
                let user = Expression::Table(Box::new(self.parse_object_name()?));
                properties.push(property("EXECUTE AS USER", None, Some(user)));
            } else if self.match_token(TokenType::When) {
                properties.push(property("WHEN", None, Some(self.parse_expression()?)));
            } else if let Some(state) = ["STARTED", "SUSPENDED"]
                .into_iter()
                .find(|state| self.match_identifier(state))
            {
                properties.push(property(state, None, None));
            } else {
                let name = self.parse_simple_name()?;
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_snowflake_task_value()?;
                properties.push(parameter_property(&name, Some(value)));
                self.match_token(TokenType::Comma);
            }
        }
        Ok(properties)
    }

    /// A task property value as `skip_snowflake_task_value` reads it: a parenthesized list, a literal or a name.
    fn parse_snowflake_task_value(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::LParen) {
            let mut items: Vec<Expression> = Vec::new();
            while !self.match_token(TokenType::RParen) {
                items.push(match self.check(TokenType::String) {
                    true => self.parse_primary()?,
                    false => Expression::Table(Box::new(self.parse_object_name()?)),
                });
                self.match_token(TokenType::Comma);
            }
            return Ok(Expression::Tuple(Box::new(Tuple { expressions: items })));
        }
        if self.check(TokenType::Dash) {
            return self.parse_unary();
        }
        if self.check(TokenType::String)
            || self.check(TokenType::DollarString)
            || self.check(TokenType::HeredocString)
            || self.check(TokenType::Number)
            || self.check(TokenType::True)
            || self.check(TokenType::False)
        {
            return self.parse_primary();
        }
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Skips one name part or a dotted name, refusing strings and Snowflake reserved words.
    pub(crate) fn skip_snowflake_name(&mut self) -> Result<()> {
        loop {
            if self.is_snowflake_reserved_keyword()
                || !(self.is_identifier_token() || self.check_keyword())
            {
                return Err(self.parse_error("Expected a name"));
            }
            self.skip();
            if !self.match_token(TokenType::Dot) {
                return Ok(());
            }
        }
    }

    /// Skips a task property value: a string, a signed number, a boolean, a name or a parenthesized list of names and strings, which may be empty.
    fn skip_snowflake_task_value(&mut self) -> Result<()> {
        if self.match_token(TokenType::LParen) {
            if self.match_token(TokenType::RParen) {
                return Ok(());
            }
            loop {
                if !self.match_token(TokenType::String) {
                    self.skip_snowflake_name()?;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(());
        }
        self.match_token(TokenType::Dash);
        if self.check(TokenType::String)
            || self.check(TokenType::DollarString)
            || self.check(TokenType::HeredocString)
            || self.check(TokenType::Number)
            || self.check(TokenType::True)
            || self.check(TokenType::False)
        {
            self.skip();
            return Ok(());
        }
        self.skip_snowflake_name()
    }

    /// Skips `( <name> = <value> , ... )`, reading each value with `value`; `quoted_name` also admits a string name, as Snowflake does for a contact purpose.
    fn skip_snowflake_task_assignments(
        &mut self,
        value: fn(&mut Self) -> Result<()>,
        quoted_name: bool,
    ) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            if !(quoted_name && self.match_token(TokenType::String)) {
                self.skip_snowflake_name()?;
            }
            self.expect_token(TokenType::Eq)?;
            value(self)?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Snowflake `CREATE TASK GRAPH [ IF NOT EXISTS ] <name>` and its properties, after GRAPH.
    pub(crate) fn parse_snowflake_create_task_graph(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Expression> {
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name_start = self.current;
        self.skip_snowflake_name()?;
        let name = self.tokens_to_sql(name_start, self.current);
        let task_properties = self.parse_snowflake_task_properties(true)?;
        Ok(Expression::CreateTask(Box::new(CreateTask {
            or_replace,
            or_alter,
            if_not_exists,
            name,
            properties: String::new(),
            task_properties,
            body: None,
            clone_source: None,
            graph: true,
        })))
    }
}
