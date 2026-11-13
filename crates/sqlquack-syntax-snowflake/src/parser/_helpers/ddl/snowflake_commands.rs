//! Snowflake's rules on COMMENT ON names and values, stage commands, task properties, DEFINE and class instance method calls.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::EXECUTE_DCM_PROJECT;
use crate::parser::constants::SNOWFLAKE_DEFINE_OPAQUE_KINDS;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::Call;
use sqlquack_syntax::expressions::schema_objects::models::ExecuteObject;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::statements::models::CreateTask;
use sqlquack_syntax::expressions::statements::models::ExecuteUsingPolicyContext;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;
use sqlquack_syntax::traversal::types::ExpressionWalk;

impl Parser {
    /// In Snowflake, refuse an EXECUTE IMMEDIATE, read after EXECUTE, that is neither `IMMEDIATE <source> [ USING ( <variable> [ , ... ] ) ]` nor `IMMEDIATE FROM <location> [ USING ( <name> => <value> [ , ... ] ) ] [ DRY_RUN = { TRUE | FALSE } ]`; the cursor ends where it started.
    pub(crate) fn check_execute_immediate(&mut self) -> Result<()> {
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        let start = self.current;
        let checked = self.check_execute_immediate_parts();
        self.current = start;
        checked
    }

    fn check_execute_immediate_parts(&mut self) -> Result<()> {
        self.skip();
        let from = self.match_token(TokenType::From);
        if from {
            if !self.check(TokenType::String) && !self.is_stage_path() {
                return Err(self.parse_error("Expected a stage path or a string"));
            }
            self.parse_location_value()?;
        } else if self.check(TokenType::Dollar) {
            let end = self.peek().span.end;
            self.skip();
            if !self.is_identifier_token() || self.peek().span.start != end {
                return Err(self.parse_error("Expected a session variable's name"));
            }
            self.skip();
        } else if self.check(TokenType::String)
            || self.check(TokenType::DollarString)
            || (self.is_identifier_token()
                && !self.is_snowflake_reserved_keyword()
                && !self.peek_text().starts_with('@'))
        {
            self.skip();
        } else {
            return Err(self.parse_error("Expected the statement to execute"));
        }
        if self.match_token(TokenType::Using) {
            self.expect_token(TokenType::LParen)?;
            loop {
                if from {
                    self.expect_identifier_or_keyword()?;
                    self.expect_token(TokenType::FArrow)?;
                }
                self.parse_expression()?;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        if from && self.match_identifier("DRY_RUN") {
            self.expect_token(TokenType::Eq)?;
            if !self.match_token(TokenType::True) && !self.match_token(TokenType::False) {
                return Err(self.parse_error("Expected TRUE or FALSE"));
            }
        }
        match self.is_statement_end_at(0) {
            true => Ok(()),
            false => Err(self.parse_error("Expected the end of EXECUTE IMMEDIATE")),
        }
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

    /// A called object's dotted name; an empty middle part (`<database>..<name>`) is kept as an empty part.
    pub(crate) fn parse_call_name(&mut self) -> Result<Vec<Identifier>> {
        let mut name = vec![self.expect_identifier_or_keyword_with_quoted()?];
        while self.match_token(TokenType::Dot) {
            if name.len() == 1 && self.match_token(TokenType::Dot) {
                name.push(Identifier::new(""));
            }
            name.push(self.expect_identifier_or_keyword_with_quoted()?);
        }
        for part in &name {
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
        if opaque_kind && matches!(definition, Expression::Raw(_)) {
            return Ok(definition);
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
                self.skip_snowflake_task_assignments(|parser| {
                    if parser.match_token(TokenType::String) {
                        return Ok(());
                    }
                    parser.skip_snowflake_name()
                })?;
            } else if self.match_identifier("CONTACT") {
                self.skip_snowflake_task_assignments(Self::skip_snowflake_name)?;
            } else if self.match_identifier("TASK_DEFAULTS") {
                self.skip_snowflake_task_assignments(Self::skip_snowflake_task_value)?;
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

    /// Skips a task property value: a string, a signed number, a boolean, a name or a parenthesized name list.
    fn skip_snowflake_task_value(&mut self) -> Result<()> {
        if self.match_token(TokenType::LParen) {
            self.skip_snowflake_name()?;
            while self.match_token(TokenType::Comma) {
                self.skip_snowflake_name()?;
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

    /// Skips `( <name> = <value> , ... )`, reading each value with `value`.
    fn skip_snowflake_task_assignments(
        &mut self,
        value: fn(&mut Self) -> Result<()>,
    ) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            self.skip_snowflake_name()?;
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
    ) -> Result<Expression> {
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name_start = self.current;
        self.skip_snowflake_name()?;
        let name = self.tokens_to_sql(name_start, self.current);
        let properties_start = self.current;
        self.check_snowflake_task_properties(true)?;
        Ok(Expression::CreateTask(Box::new(CreateTask {
            or_replace,
            if_not_exists,
            name,
            properties: self.tokens_to_sql(properties_start, self.current),
            body: None,
            clone_source: None,
            graph: true,
        })))
    }
}
