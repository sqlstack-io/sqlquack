//! Snowflake's rules on COMMENT ON names and values, role grants, stage commands, task properties, DEFINE and class instance method calls.

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::Call;
use crate::expressions::statements::models::CreateTask;
use crate::expressions::statements::models::ExecuteObject;
use crate::expressions::statements::models::ExecuteUsingPolicyContext;
use crate::expressions::statements::models::{Grant, GrantPrincipal, Revoke};
use crate::expressions::tree::models::Expression;
use crate::parser::constants::EXECUTE_DCM_PROJECT;
use crate::parser::constants::ROLE;
use crate::parser::constants::SHARE;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;
use crate::traversal::types::ExpressionWalk;

/// The roles a Snowflake GRANT or REVOKE without ON hands out, each with the grantee kinds it takes.
const ROLE_GRANTS: &[(&[&str], &[&[&str]])] = &[
    (&["ROLE"], &[&["ROLE"], &["USER"]]),
    (
        &["DATABASE", "ROLE"],
        &[
            &["DATABASE", "ROLE"],
            &["ROLE"],
            &["USER"],
            &["APPLICATION"],
            &["SHARE"],
        ],
    ),
    (
        &["APPLICATION", "ROLE"],
        &[
            &["APPLICATION", "ROLE"],
            &["APPLICATION"],
            &["USER"],
            &["ROLE"],
        ],
    ),
];

impl Parser {
    /// In Snowflake, parse a GRANT (or, when `revoke`, a REVOKE) of a role after its first word; None outside Snowflake or for a SERVICE ROLE.
    pub(super) fn parse_snowflake_role_grant(
        &mut self,
        revoke: bool,
    ) -> Result<Option<Expression>> {
        if !self.is_snowflake() || self.check_text_seq(&["SERVICE", "ROLE"]) {
            return Ok(None);
        }
        let grant_option = revoke && self.match_text_seq(&["GRANT"]) && {
            self.expect_alter_words(&[&["OPTION", "FOR"]])?;
            true
        };
        let Some((kind, grantees)) = ROLE_GRANTS
            .iter()
            .find(|(kind, _)| self.match_text_seq(kind))
        else {
            return Err(self.parse_error("Expected ROLE, DATABASE ROLE or APPLICATION ROLE"));
        };
        let role = securable_identifier(self.parse_object_name()?);
        self.expect_token(if revoke {
            TokenType::From
        } else {
            TokenType::To
        })?;
        let grantee = self.expect_alter_words(grantees)?;
        let principal = GrantPrincipal {
            name: securable_identifier(self.parse_object_name()?),
            is_role: grantee == ROLE,
            is_group: false,
            is_share: grantee == SHARE,
            kind: (!matches!(grantee.as_str(), "ROLE" | "SHARE")).then_some(grantee),
        };
        let kind = Some(kind.join(" "));
        let securable = role;
        if revoke {
            let cascade = self.match_token(TokenType::Cascade);
            let restrict = !cascade && self.match_token(TokenType::Restrict);
            return Ok(Some(Expression::Revoke(Box::new(Revoke {
                privileges: Vec::new(),
                kind,
                securable,
                function_params: Vec::new(),
                principals: vec![principal],
                grant_option,
                cascade,
                restrict,
            }))));
        }
        let grant_option = self.match_token(TokenType::With) && {
            self.expect_token(TokenType::Grant)?;
            self.expect_alter_words(&[&["OPTION"]])?;
            true
        };
        Ok(Some(Expression::Grant(Box::new(Grant {
            privileges: Vec::new(),
            kind,
            securable,
            function_params: Vec::new(),
            principals: vec![principal],
            grant_option,
            as_principal: None,
        }))))
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
        } else if self.match_text_seq(&["TASK", "GRAPH"]) {
            "TASK GRAPH"
        } else if self.match_text_seq(&["TASK"]) {
            "TASK"
        } else if self.match_text_seq(&["EXPERIMENT"]) {
            "EXPERIMENT"
        } else {
            return Ok(None);
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
                self.current = start;
                return Ok(None);
            }
        }
        Ok(Some(Expression::ExecuteObject(Box::new(node))))
    }
}

/// A role or grantee name as one identifier, its parts joined by dots like a GRANT securable.
fn securable_identifier(name: crate::expressions::names::models::TableRef) -> Identifier {
    match (name.catalog, name.schema) {
        (None, None) => name.name,
        (catalog, schema) => Identifier::new(
            [catalog, schema, Some(name.name)]
                .into_iter()
                .flatten()
                .map(|part| part.name)
                .collect::<Vec<_>>()
                .join("."),
        ),
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

    /// Snowflake DCM `DEFINE`, refused for kinds kept as opaque text or not yet checked closely.
    pub(crate) fn parse_snowflake_define(&mut self) -> Result<Expression> {
        let definition = self.parse_create()?;
        let unchecked_kind = matches!(
            definition,
            Expression::CreateTable(_)
                | Expression::CreateView(_)
                | Expression::CreateFunction(_)
                | Expression::CreateProcedure(_)
        );
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
        while !(graph && (self.is_at_end() || self.check(TokenType::Semicolon)))
            && !(!graph && self.check(TokenType::As))
        {
            self.match_token(TokenType::With);
            if self.match_identifier("TAG") {
                self.skip_snowflake_task_assignments(|parser| {
                    parser.expect_token(TokenType::String).map(|_| ())
                })?;
            } else if self.match_identifier("CONTACT") {
                self.skip_snowflake_task_assignments(Self::skip_snowflake_name)?;
            } else if self.match_identifier("TASK_DEFAULTS") {
                self.skip_snowflake_task_assignments(Self::skip_snowflake_task_value)?;
            } else if self.match_token(TokenType::After) {
                self.skip_snowflake_name()?;
                while self.match_token(TokenType::Comma) {
                    self.skip_snowflake_name()?;
                }
            } else if self.match_text_seq(&["EXECUTE", "AS", "USER"]) {
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
