//! Snowflake's rules on COMMENT ON names and values, stage commands, task properties, DEFINE and class instance method calls.

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::Call;
use crate::expressions::schema_objects::models::ExecuteObject;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::statements::models::CreateTask;
use crate::expressions::statements::models::ExecuteUsingPolicyContext;
use crate::expressions::tree::models::Expression;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::EXECUTE_DCM_PROJECT;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use crate::tokens::models::ParserToken;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;
use crate::traversal::types::ExpressionWalk;

/// The action, alias and properties of an EXECUTE DCM PROJECT deploy, plan or preview.
type DcmRun = (String, Option<Identifier>, Vec<ObjectProperty>);

impl Parser {
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
            } else if let Some((action, alias, properties)) = self.parse_dcm_project_run()? {
                node.action = Some(action);
                node.alias = alias;
                node.properties = properties;
            } else {
                self.current = start;
                return Ok(None);
            }
        }
        Ok(Some(Expression::ExecuteObject(Box::new(node))))
    }
}

impl Parser {
    /// Reads `DEPLOY [ AS <alias> ] <source>`, `PLAN [ DELTA ] <source> [ OUTPUT_PATH '<path>' ]` or `PREVIEW <table> USING CONFIGURATION <config> FROM '<path>' [ LIMIT <n> ]` of EXECUTE DCM PROJECT; none for other actions.
    fn parse_dcm_project_run(&mut self) -> Result<Option<DcmRun>> {
        if self.match_text_seq(&["DEPLOY"]) {
            let alias = match self.match_token(TokenType::As) {
                true => Some(self.parse_simple_name()?),
                false => None,
            };
            return Ok(Some((
                "DEPLOY".to_owned(),
                alias,
                self.parse_dcm_project_source()?,
            )));
        }
        if self.match_text_seq(&["PLAN"]) {
            let action = match self.match_text_seq(&["DELTA"]) {
                true => "PLAN DELTA",
                false => "PLAN",
            };
            let mut properties = self.parse_dcm_project_source()?;
            if self.match_text_seq(&["OUTPUT_PATH"]) {
                properties.push(property(
                    "OUTPUT_PATH",
                    None,
                    Some(self.parse_string_value()?),
                ));
            }
            return Ok(Some((action.to_owned(), None, properties)));
        }
        if !self.match_text_seq(&["PREVIEW"]) {
            return Ok(None);
        }
        let table = Expression::Table(Box::new(self.parse_object_name()?));
        let mut properties = vec![property("PREVIEW", None, Some(table))];
        if !self.check_text_seq(&["USING"]) {
            return Err(self.parse_error("Expected USING CONFIGURATION"));
        }
        properties.extend(self.parse_dcm_project_source()?);
        if self.match_token(TokenType::Limit) {
            if !self.check(TokenType::Number) {
                return Err(self.parse_error("Expected a row count after LIMIT"));
            }
            properties.push(property("LIMIT", None, Some(self.parse_primary()?)));
        }
        Ok(Some(("PREVIEW".to_owned(), None, properties)))
    }

    /// Reads `[ USING CONFIGURATION <config> ] FROM '<path>'` or `FROM DEPLOYMENT <deployment>`.
    fn parse_dcm_project_source(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let configured = self.match_text_seq(&["USING"]);
        if configured {
            self.expect_alter_words(&[&["CONFIGURATION"]])?;
            let config = Expression::Table(Box::new(self.parse_object_name()?));
            properties.push(property("USING CONFIGURATION", None, Some(config)));
        }
        self.expect_token(TokenType::From)?;
        if !configured && self.match_text_seq(&["DEPLOYMENT"]) {
            let deployment = Expression::Table(Box::new(self.parse_object_name()?));
            properties.push(property("FROM DEPLOYMENT", None, Some(deployment)));
        } else {
            properties.push(property("FROM", None, Some(self.parse_string_value()?)));
        }
        Ok(properties)
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
        Ok(())
    }

    /// Refuses the EXECUTE IMMEDIATE shapes Snowflake rejects: no target, a stage without FROM, a broken FROM location or option, and an empty or unbalanced USING list.
    pub(crate) fn check_snowflake_execute_immediate(&self) -> Result<()> {
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        let rest = &self.tokens[(self.current + 1).min(self.tokens.len())..];
        let end = rest
            .iter()
            .position(|token| token.token_type == TokenType::Semicolon)
            .unwrap_or(rest.len());
        let tokens = &rest[..end];
        let refuse = |message: &str| Err(self.parse_error(message));
        let Some(first) = tokens.first() else {
            return refuse("Expected a statement, a variable or FROM after EXECUTE IMMEDIATE");
        };
        if first.token_type == TokenType::DAt || first.text.starts_with('@') {
            return refuse("Expected FROM before the stage");
        }
        if first.token_type != TokenType::From {
            let target = match first.token_type {
                TokenType::Dollar | TokenType::Parameter if first.text.len() <= 1 => {
                    match tokens.get(1) {
                        Some(name) if name.span.start == first.span.end => 2,
                        _ => return refuse("Expected a variable name after $"),
                    }
                }
                _ => 1,
            };
            return match &tokens[target..] {
                [] => Ok(()),
                [using, list @ ..] if using.text.eq_ignore_ascii_case("USING") => {
                    Self::check_immediate_list(list, false).map_or_else(
                        || refuse("Expected a non-empty USING ( ... ) list"),
                        |_| Ok(()),
                    )
                }
                _ => refuse("Expected USING or the end after the EXECUTE IMMEDIATE target"),
            };
        }
        let location = &tokens[1..];
        let Some(head) = location.first() else {
            return refuse("Expected a stage or a path after FROM");
        };
        let mut next = 1;
        if head.token_type != TokenType::String {
            while location
                .get(next)
                .is_some_and(|token| token.span.start == location[next - 1].span.end)
            {
                next += 1;
            }
            if location[..next]
                .iter()
                .any(|token| token.token_type == TokenType::String)
            {
                return refuse("A stage location part cannot be a string literal");
            }
        }
        let mut options = &location[next..];
        if options
            .first()
            .is_some_and(|token| token.text.eq_ignore_ascii_case("USING"))
        {
            match Self::check_immediate_list(&options[1..], true) {
                Some(used) => options = &options[1 + used..],
                None => return refuse("Expected USING ( <name> => <value> , ... )"),
            }
        }
        match options {
            [] => Ok(()),
            [dry_run, eq, value]
                if dry_run.text.eq_ignore_ascii_case("DRY_RUN")
                    && eq.token_type == TokenType::Eq
                    && matches!(value.token_type, TokenType::True | TokenType::False) =>
            {
                Ok(())
            }
            _ => refuse("Expected USING, DRY_RUN = TRUE | FALSE or the end after the location"),
        }
    }

    /// The tokens a non-empty `( <item> , ... )` list takes, each item `<name> => <value>` when `named`; none when it is malformed.
    fn check_immediate_list(tokens: &[ParserToken], named: bool) -> Option<usize> {
        if tokens.first()?.token_type != TokenType::LParen {
            return None;
        }
        let close = tokens
            .iter()
            .position(|token| token.token_type == TokenType::RParen)?;
        let items = &tokens[1..close];
        if items.is_empty() {
            return None;
        }
        let well_formed = items
            .split(|token| token.token_type == TokenType::Comma)
            .all(|item| match named {
                true => matches!(item, [name, arrow, _, ..] if name.token_type != TokenType::FArrow && arrow.token_type == TokenType::FArrow),
                false => !item.is_empty(),
            });
        well_formed.then_some(close + 1)
    }

    /// Snowflake DCM `DEFINE`, refused for kinds kept as opaque text or not yet checked closely.
    pub(crate) fn parse_snowflake_define(&mut self) -> Result<Expression> {
        let definition = self.parse_create()?;
        let unchecked_kind = matches!(definition, Expression::CreateTable(_));
        if unchecked_kind
            || definition
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
