//! Parser ddl: the shared check of a Snowflake CREATE or ALTER kept as opaque text, which refuses a malformed head, `NAME = value` option list or tag list while the tree stays the same text.

use crate::parser::_helpers::ddl::snowflake_alter::{alter_object, named_property, property};
use crate::parser::_helpers::statements::analyze::snowflake_reserved_word;
use crate::parser::constants::{
    SNOWFLAKE_DEFINE_OPAQUE_KINDS, SNOWFLAKE_DROP_KINDS, SNOWFLAKE_OPAQUE_ALTER_ACTIONS,
    SNOWFLAKE_OPAQUE_KINDS, SNOWFLAKE_OPAQUE_MODIFIERS, SNOWFLAKE_OPAQUE_OPERAND_WORDS,
    SNOWFLAKE_OPAQUE_UNNAMED_KIND, SNOWFLAKE_STRUCTURED_OPAQUE_KINDS, SUSPENDED,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Refuse an opaque Snowflake CREATE (`create`) or ALTER, read from its kind to the statement end, whose head or option lists are malformed; the cursor ends where it started.
    pub(crate) fn check_opaque_statement(&mut self, create: bool) -> Result<()> {
        if !self.applies_snowflake_context_rules() {
            return Ok(());
        }
        let start = self.current;
        let checked = match create {
            true if self.check_text_seq(&["AI", "FUNCTION"]) => self.check_ai_function(),
            true if self.check_text_seq(&["ORGANIZATION", "PROFILE"]) => {
                self.check_organization_profile()
            }
            _ => self
                .check_opaque_head(create)
                .and_then(|()| self.check_opaque_shape(create, start))
                .and_then(|()| self.check_opaque_options()),
        };
        self.current = start;
        checked
    }

    /// After a CREATE's name, the shape its kind needs: FROM and a quoted location for a code bundle or notebook project, a snapshot's service volume, an external agent's version, a variable's type, AS COPY for a pipe's body, no bare list after an interactive warehouse's name, an Openflow connector's added version and push, and a warehouse's actions; the cursor stays after the name.
    fn check_opaque_shape(&mut self, create: bool, kind_at: usize) -> Result<()> {
        let after_name = self.current;
        let kind = self.tokens[kind_at].text.to_ascii_uppercase();
        let second = self.tokens[kind_at + 1].text.to_ascii_uppercase();
        if !create {
            let connector = matches!((kind.as_str(), second.as_str()), ("OPENFLOW", "CONNECTOR"));
            let warehouse = matches!(kind.as_str(), "WAREHOUSE");
            let checked = match (connector, warehouse) {
                (true, _) if self.check_text_seq(&["ADD"]) => self.check_connector_version(),
                (true, _) if self.check_text_seq(&["PUSH"]) => self.check_connector_push(),
                (_, true) => {
                    let exists = self.tokens[kind_at + 1].token_type == TokenType::If
                        && self.tokens.get(kind_at + 2).map(|token| token.token_type)
                            == Some(TokenType::Exists);
                    let unnamed =
                        self.current == kind_at + 1 || (exists && self.current == kind_at + 3);
                    self.check_warehouse_action(unnamed)
                }
                _ => Ok(()),
            };
            self.current = after_name;
            return checked;
        }
        let project = matches!(second.as_str(), "PROJECT") && after_name > kind_at + 2;
        let checked = match kind.as_str() {
            "EXTERNAL" if matches!(second.as_str(), "AGENT") => self.check_external_agent_tail(),
            "VARIABLE" => self.check_variable_tail(),
            "SNAPSHOT" if !matches!(second.as_str(), "SET" | "POLICY") => {
                self.check_snapshot_source()
            }
            "CODE" if !self.check(TokenType::From) => Err(self.parse_error("Expected FROM")),
            "NOTEBOOK" if project && !self.check(TokenType::From) => {
                Err(self.parse_error("Expected FROM"))
            }
            "CODE" if !self.check_next(TokenType::String) => {
                Err(self.parse_error("Expected a quoted location after FROM"))
            }
            "NOTEBOOK" if project && !self.check_next(TokenType::String) => {
                Err(self.parse_error("Expected a quoted location after FROM"))
            }
            "PIPE" => self.check_pipe_body(),
            "INTERACTIVE" if self.check(TokenType::LParen) => {
                Err(self.parse_error("Expected a property after the warehouse name"))
            }
            _ => Ok(()),
        };
        self.current = after_name;
        checked
    }

    /// `FROM SERVICE <name> VOLUME <name> INSTANCE <number>` after a snapshot's name, which options follow without a bare parenthesised list.
    fn check_snapshot_source(&mut self) -> Result<()> {
        if !self.match_text_seq(&["FROM", "SERVICE"]) {
            return Err(self.parse_error("Expected FROM SERVICE"));
        }
        self.parse_object_name()?;
        if !self.match_text_seq(&["VOLUME"]) {
            return Err(self.parse_error("Expected VOLUME"));
        }
        self.parse_object_name()?;
        if !self.match_text_seq(&["INSTANCE"]) || !self.match_token(TokenType::Number) {
            return Err(self.parse_error("Expected INSTANCE and a number"));
        }
        match self.check(TokenType::LParen) {
            true => Err(self.parse_error("Expected an option")),
            false => Ok(()),
        }
    }

    /// `ADD VERSION FROM '<location>'` or `ADD LIVE VERSION FROM LAST` after an Openflow connector's name.
    fn check_connector_version(&mut self) -> Result<()> {
        self.skip();
        let live = self.match_text_seq(&["LIVE"]);
        if !self.match_text_seq(&["VERSION", "FROM"]) {
            return Err(self.parse_error("Expected VERSION FROM"));
        }
        let source = match live {
            true => self.match_text_seq(&["LAST"]),
            false => self.match_token(TokenType::String),
        };
        match source {
            true => Ok(()),
            false => {
                Err(self.parse_error("Expected LAST for a live version, or a location string"))
            }
        }
    }

    /// A variable's `<type> [ DEFAULT <value> ] [ COMMENT = '<string>' ]` after its name.
    fn check_variable_tail(&mut self) -> Result<()> {
        if self.check(TokenType::String) || self.is_statement_end_at(0) {
            return Err(self.parse_error("Expected the variable's type"));
        }
        self.parse_data_type()?;
        if self.match_token(TokenType::Default) {
            if self.is_statement_end_at(0) {
                return Err(self.parse_error("Expected a default value"));
            }
            self.parse_expression()?;
        }
        if self.match_text_seq(&["COMMENT"])
            && !(self.match_token(TokenType::Eq) && self.match_token(TokenType::String))
        {
            return Err(self.parse_error("Expected = and a string after COMMENT"));
        }
        match self.is_statement_end_at(0) || self.check(TokenType::DArrow) {
            true => Ok(()),
            false => Err(self.parse_error("Expected the end of the variable")),
        }
    }

    /// A warehouse action after its name, which `unnamed` says was left out: ABORT ALL QUERIES, a named warehouse's tables, rename or tags, and unquoted names to UNSET.
    fn check_warehouse_action(&mut self, unnamed: bool) -> Result<()> {
        let tag = self
            .peek_nth(1)
            .is_some_and(|token| token.text.eq_ignore_ascii_case("TAG"))
            && !self
                .peek_nth(2)
                .is_some_and(|token| token.token_type == TokenType::Eq);
        let tables = self.check_text_seq(&["ADD"]) || self.check_text_seq(&["DROP"]);
        let tagging = tag && (self.check_text_seq(&["SET"]) || self.check_text_seq(&["UNSET"]));
        if unnamed && (tables || tagging || self.check_text_seq(&["RENAME"])) {
            return Err(self.parse_error("Expected a warehouse name before the action"));
        }
        if self.match_text_seq(&["ABORT"]) && !self.match_text_seq(&["ALL", "QUERIES"]) {
            return Err(self.parse_error("Expected ALL QUERIES after ABORT"));
        }
        if self.match_text_seq(&["UNSET"]) {
            self.match_text_seq(&["TAG"]);
            return match self.check(TokenType::String) {
                true => Err(self.parse_error("Expected a name to unset, not a string")),
                false => Ok(()),
            };
        }
        match tables {
            true => self.check_warehouse_tables(),
            false => Ok(()),
        }
    }

    /// `{ ADD | DROP } TABLES ( <name> [ , ... ] )` after a warehouse's name.
    fn check_warehouse_tables(&mut self) -> Result<()> {
        self.skip();
        if !self.match_text_seq(&["TABLES"]) || !self.match_token(TokenType::LParen) {
            return Err(self.parse_error("Expected TABLES and a parenthesised list"));
        }
        loop {
            if self.check(TokenType::String) || self.check(TokenType::RParen) {
                return Err(self.parse_error("Expected a table name"));
            }
            self.parse_object_name()?;
            if !self.match_token(TokenType::Comma) {
                return Ok(());
            }
        }
    }

    /// `PUSH [ TO '<location>' ]` after an Openflow connector's name, which properties follow.
    fn check_connector_push(&mut self) -> Result<()> {
        self.skip();
        if self.match_token(TokenType::To) && !self.match_token(TokenType::String) {
            return Err(self.parse_error("Expected a location string after TO"));
        }
        match self.check(TokenType::Eq) {
            true => Err(self.parse_error("Expected a property name")),
            false => Ok(()),
        }
    }

    /// An external agent's `[ WITH VERSION <name> ]`, the only clause after its name.
    fn check_external_agent_tail(&mut self) -> Result<()> {
        if self.match_token(TokenType::With) {
            if !self.match_text_seq(&["VERSION"]) {
                return Err(self.parse_error("Expected VERSION after WITH"));
            }
            self.parse_object_name()?;
        }
        match self.is_statement_end_at(0) {
            true => Ok(()),
            false => Err(self.parse_error("Expected the end of the external agent")),
        }
    }

    /// A pipe's options up to `AS`, which a COPY statement follows.
    fn check_pipe_body(&mut self) -> Result<()> {
        let mut depth = 0usize;
        while !self.is_statement_end_at(0) {
            match self.peek().token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen => depth = depth.saturating_sub(1),
                TokenType::As if depth == 0 => {
                    self.skip();
                    return match self.check(TokenType::Copy) {
                        true => Ok(()),
                        false => Err(self.parse_error("Expected a COPY statement after AS")),
                    };
                }
                _ => {}
            }
            self.skip();
        }
        Err(self.parse_error("Expected AS and a COPY statement"))
    }

    /// CREATE ORGANIZATION PROFILE `[ [ IF NOT EXISTS ] <name> [ AS '<manifest>' | FROM <stage path> [ VERSION <name> ] ] [ PUBLISH = { TRUE | FALSE } ] ]`.
    fn check_organization_profile(&mut self) -> Result<()> {
        self.current += 2;
        if self.is_statement_end_at(0) {
            return Ok(());
        }
        if self.check(TokenType::If)
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        self.parse_object_name()?;
        if self.match_token(TokenType::As) {
            if !self.match_token(TokenType::String) {
                return Err(self.parse_error("Expected the manifest as a string"));
            }
        } else if self.match_token(TokenType::From) {
            if !self.is_stage_path() {
                return Err(self.parse_error("Expected a stage path"));
            }
            self.parse_stage_path()?;
            if self.match_identifier("VERSION") {
                self.skip_snowflake_name()?;
            }
        }
        if self.match_identifier("PUBLISH") {
            self.expect_token(TokenType::Eq)?;
            if !self.match_token(TokenType::True) && !self.match_token(TokenType::False) {
                return Err(self.parse_error("Expected TRUE or FALSE"));
            }
        }
        match self.is_statement_end_at(0) {
            true => Ok(()),
            false => Err(self.parse_error("Expected the end of the organization profile")),
        }
    }

    /// Refuse a CREATE AI FUNCTION whose text after AI a CREATE FUNCTION would refuse.
    fn check_ai_function(&mut self) -> Result<()> {
        self.skip();
        self.parse_create_function(false, false, false, false)?;
        match self.is_statement_end_at(0) {
            true => Ok(()),
            false => Err(self.parse_error("Expected the end of the AI function")),
        }
    }

    /// An ALTER with no structured grammar after its checks: the rest of the statement as raw text.
    pub(crate) fn parse_opaque_alter(&mut self) -> Result<Expression> {
        self.check_opaque_statement(false)?;
        let start = self.current;
        if self.check_text_seq(&["WAREHOUSE"]) {
            if let Some(alter) = self.parse_checked_warehouse_alter()? {
                return Ok(alter);
            }
            self.current = start;
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let sql = self.tokens_to_sql(start, self.current);
        Ok(Expression::Raw(Raw {
            sql: format!("ALTER {sql}"),
        }))
    }

    /// An ALTER WAREHOUSE its checks passed, as a tree: the name, which may be left out before an action, the action's words, and its names or properties (a lone IF before an action is the name, as Snowflake reads it); None keeps the raw text.
    fn parse_checked_warehouse_alter(&mut self) -> Result<Option<Expression>> {
        self.skip();
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = match self.is_opaque_action_at(0) {
            true => None,
            false => Some(self.parse_object_name()?),
        };
        let action = self.expect_alter_words(&[
            &["ABORT", "ALL", "QUERIES"],
            &["RESUME", "IF", "SUSPENDED"],
            &["RENAME", "TO"],
            &["ADD", "TABLES"],
            &["DROP", "TABLES"],
            &["ENABLE"],
            &["DISABLE"],
            &["SUSPEND"],
            &["RESUME"],
            &["SET"],
            &["UNSET"],
        ])?;
        let parts: (Vec<Expression>, Vec<ObjectProperty>) = match action.as_str() {
            "RENAME TO" => (
                vec![Expression::Table(Box::new(self.parse_object_name()?))],
                vec![],
            ),
            "ADD TABLES" | "DROP TABLES" => {
                self.expect_token(TokenType::LParen)?;
                let tables = self.parse_object_name_list()?;
                self.expect_token(TokenType::RParen)?;
                (tables, vec![])
            }
            "SET" => (vec![], self.parse_alter_object_properties(true, false)?),
            "UNSET" => (vec![], self.parse_alter_object_properties(false, false)?),
            _ => (vec![], vec![]),
        };
        if !self.is_statement_end_at(0) {
            return Ok(None);
        }
        let Expression::AlterObject(mut alter) =
            alter_object("WAREHOUSE", (TableRef::new(""), if_exists), action, parts)
        else {
            return Ok(None);
        };
        alter.name = name;
        Ok(Some(Expression::AlterObject(alter)))
    }

    /// Whether the token at `at` continues or ends the name in `CREATE MODEL <name>`: a dot, FROM, WITH VERSION, or the statement end.
    pub(crate) fn model_name_ends_at(&self, at: usize) -> bool {
        let Some(token) = self.tokens.get(at) else {
            return true;
        };
        matches!(
            token.token_type,
            TokenType::Dot | TokenType::From | TokenType::Semicolon
        ) || (token.token_type == TokenType::With
            && self
                .tokens
                .get(at + 1)
                .is_some_and(|next| next.text.eq_ignore_ascii_case("VERSION")))
    }

    /// The kind, `IF [ NOT ] EXISTS` and name of a kind the opaque check knows, a notebook project's name with its database left out, then an ALTER's action; nothing for another kind. NOTEBOOK PROJECT is a kind only when a name follows it.
    fn check_opaque_head(&mut self, create: bool) -> Result<()> {
        let kind = SNOWFLAKE_OPAQUE_KINDS
            .iter()
            .chain(SNOWFLAKE_DROP_KINDS)
            .find(|words| {
                self.check_text_seq(words)
                    && !(**words == ["MODEL", "MONITOR"]
                        && self.model_name_ends_at(self.current + 2))
                    && !(**words == ["NOTEBOOK", "PROJECT"] && self.notebook_name_ends_at(2))
            });
        let Some(&kind) = kind else {
            return self.check_opaque_unknown_kind();
        };
        self.current += kind.len();
        if !create && self.check(TokenType::If) && self.is_opaque_action_at(1) {
            self.skip();
            return Ok(());
        }
        if self.check(TokenType::If) {
            let exists = match create {
                true => self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]),
                false => self.match_keywords(&[TokenType::If, TokenType::Exists]),
            };
            if !exists {
                return Err(self.parse_error(match create {
                    true => "Expected IF NOT EXISTS",
                    false => "Expected IF EXISTS",
                }));
            }
        }
        if create {
            if kind == ["NOTEBOOK", "PROJECT"] {
                self.match_token(TokenType::Dot);
            }
            return self.parse_object_name().map(|_| ());
        }
        if kind == SNOWFLAKE_OPAQUE_UNNAMED_KIND && self.is_opaque_action_at(0) {
            return Ok(());
        }
        self.parse_object_name()?;
        match self.is_opaque_action_at(0) {
            true => Ok(()),
            false => Err(self.parse_error("Expected an action after the name")),
        }
    }

    /// Whether the token `offset` tokens ahead ends a notebook's name: the statement end, FROM or an unquoted `<property> =`, so `CREATE NOTEBOOK project FROM @s1` names the notebook PROJECT.
    fn notebook_name_ends_at(&self, offset: usize) -> bool {
        let property = self
            .peek_nth(offset)
            .is_some_and(|token| token.token_type != TokenType::QuotedIdentifier)
            && self
                .peek_nth(offset + 1)
                .is_some_and(|token| token.token_type == TokenType::Eq);
        property
            || self.peek_nth(offset).is_none_or(|token| {
                matches!(token.token_type, TokenType::From | TokenType::Semicolon)
            })
    }

    /// Refuse a kind whose first word starts a kind the check knows but whose next words complete none, as `ALTER OPENFLOW v1` does.
    fn check_opaque_unknown_kind(&self) -> Result<()> {
        let first = self.peek().text.to_ascii_uppercase();
        let partial = SNOWFLAKE_OPAQUE_KINDS
            .iter()
            .chain(SNOWFLAKE_DROP_KINDS)
            .any(|words| words[0] == first);
        match partial && self.peek().token_type != TokenType::QuotedIdentifier {
            true => Err(self.parse_error("Expected a complete object kind")),
            false => Ok(()),
        }
    }

    /// Whether the unquoted word `offset` tokens ahead starts an opaque ALTER's action.
    fn is_opaque_action_at(&self, offset: usize) -> bool {
        self.peek_nth(offset).is_some_and(|token| {
            token.token_type != TokenType::QuotedIdentifier
                && SNOWFLAKE_OPAQUE_ALTER_ACTIONS
                    .contains(&token.text.to_ascii_uppercase().as_str())
        })
    }

    /// Whether the token `offset` tokens ahead is an unquoted Snowflake reserved keyword.
    fn is_opaque_reserved_at(&self, offset: usize) -> bool {
        self.peek_nth(offset).is_some_and(|token| {
            !matches!(
                token.token_type,
                TokenType::QuotedIdentifier | TokenType::String | TokenType::Null
            ) && snowflake_reserved_word(&token.text)
        })
    }

    /// Whether the token `offset` tokens ahead has type `token_type`.
    fn is_opaque_token_at(&self, offset: usize, token_type: TokenType) -> bool {
        self.peek_nth(offset)
            .is_some_and(|token| token.token_type == token_type)
    }

    /// The tokens after the head up to a body: balanced parentheses, a name and a value around each `=`, a well-formed tag list, an operand after SET and UNSET, TO and a name after RENAME, no reserved keyword as a list item, only EXISTS, NOT EXISTS or an alert condition in `( )`, which it skips, after IF, and no word that needs an operand at the end.
    fn check_opaque_options(&mut self) -> Result<()> {
        let mut named_end = self.current;
        let mut depth = 0usize;
        while !self.is_statement_end_at(0) {
            if depth == 0 && self.is_opaque_body_start() {
                return match self.is_statement_end_at(1) {
                    true => Err(self.parse_error("Expected a body")),
                    false => Ok(()),
                };
            }
            if depth == 0 && self.check(TokenType::If) && self.check_next(TokenType::LParen) {
                self.skip_opaque_condition()?;
                continue;
            }
            match self.peek().token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen if depth == 0 => {
                    return Err(self.parse_error("Unexpected )"));
                }
                TokenType::RParen => depth -= 1,
                TokenType::Eq => self.check_opaque_assignment(depth)?,
                TokenType::If => self.check_opaque_if()?,
                _ if self.check_identifier("TAG") && self.check_next(TokenType::LParen) => {
                    self.skip();
                    self.parse_external_tags()?;
                    continue;
                }
                _ if self.check_text_seq(&["RENAME"]) => {
                    self.skip();
                    if !self.match_token(TokenType::To) {
                        return Err(self.parse_error("Expected TO after RENAME"));
                    }
                    self.parse_object_name()?;
                    named_end = self.current;
                    continue;
                }
                _ if (self.check(TokenType::Set) || self.check_identifier("UNSET"))
                    && self.is_statement_end_at(1) =>
                {
                    return Err(self.parse_error("Expected a property after SET or UNSET"));
                }
                _ if self.is_opaque_reserved_at(0) && self.is_opaque_list_item() => {
                    return Err(self.parse_error("Expected a name, not a reserved keyword"));
                }
                _ => {}
            }
            self.skip();
        }
        if depth != 0 {
            return Err(self.parse_error("Expected )"));
        }
        self.check_opaque_end(named_end)
    }

    /// Skip the `IF ( <condition> )` at the cursor, an alert's query that the option rules do not apply to, refusing it unclosed.
    fn skip_opaque_condition(&mut self) -> Result<()> {
        self.skip();
        let mut depth = 0usize;
        loop {
            if self.is_statement_end_at(0) {
                return Err(self.parse_error("Expected )"));
            }
            match self.peek().token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen => depth -= 1,
                _ => {}
            }
            self.skip();
            if depth == 0 {
                return Ok(());
            }
        }
    }

    /// Whether the token at the cursor stands alone as a name: between list punctuation, or before the statement end apart from an action word and from a stage path it is written against.
    fn is_opaque_list_item(&self) -> bool {
        let before = self.current.checked_sub(1).map(|at| &self.tokens[at]);
        let glued = before.is_some_and(|token| token.span.end == self.peek().span.start);
        let opens = matches!(
            before.map(|token| token.token_type),
            Some(TokenType::LParen | TokenType::Comma)
        );
        let closes = self.is_opaque_token_at(1, TokenType::Comma)
            || self.is_opaque_token_at(1, TokenType::RParen);
        opens && closes || self.is_statement_end_at(1) && !glued && !self.is_opaque_action_at(0)
    }

    /// The last token, which may not be a comma or a word that needs an operand, unless a name the check parsed ends at `named_end` there or the word is unreserved and stands as a value or name part after `=` or `.`.
    fn check_opaque_end(&self, named_end: usize) -> Result<()> {
        let Some(last) = self.current.checked_sub(1).map(|at| &self.tokens[at]) else {
            return Ok(());
        };
        let operand_word = !matches!(
            last.token_type,
            TokenType::QuotedIdentifier | TokenType::String
        ) && SNOWFLAKE_OPAQUE_OPERAND_WORDS
            .contains(&last.text.to_ascii_uppercase().as_str());
        let named = self.current == named_end
            || !snowflake_reserved_word(&last.text)
                && self.current.checked_sub(2).is_some_and(|at| {
                    matches!(self.tokens[at].token_type, TokenType::Dot | TokenType::Eq)
                });
        match last.token_type == TokenType::Comma || operand_word && !named {
            true => Err(self.parse_error("Expected an operand before the statement end")),
            false => Ok(()),
        }
    }

    /// Whether the current token starts the statement's body, which the option check does not read.
    fn is_opaque_body_start(&self) -> bool {
        matches!(
            self.peek().token_type,
            TokenType::As | TokenType::Arrow | TokenType::Then | TokenType::Begin
        )
    }

    /// The `=` at the cursor: a name before it outside parentheses, and a value after it that is not a reserved keyword or the start of a body.
    fn check_opaque_assignment(&self, depth: usize) -> Result<()> {
        let before = self.current.checked_sub(1).map(|at| &self.tokens[at]);
        let named = before.is_some_and(|token| {
            matches!(
                token.token_type,
                TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
            ) || token
                .text
                .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && !matches!(token.token_type, TokenType::String)
                && !snowflake_reserved_word(&token.text)
        });
        let chained = self
            .current
            .checked_sub(2)
            .is_some_and(|at| self.tokens[at].token_type == TokenType::Eq)
            && matches!(
                before.map(|token| token.token_type),
                Some(TokenType::String | TokenType::Number)
            );
        if depth == 0 && !named || chained {
            return Err(self.parse_error("Expected a property name before ="));
        }
        let missing = self.is_statement_end_at(1)
            || self.peek_nth(1).is_some_and(|token| {
                matches!(
                    token.token_type,
                    TokenType::Comma
                        | TokenType::RParen
                        | TokenType::Eq
                        | TokenType::As
                        | TokenType::Then
                )
            })
            || self.is_opaque_token_at(1, TokenType::If)
                && self.is_opaque_token_at(2, TokenType::LParen)
            || self.is_opaque_reserved_at(1)
            || self.is_opaque_token_at(2, TokenType::Eq)
                && self.peek_nth(1).is_some_and(|token| {
                    !matches!(
                        token.token_type,
                        TokenType::String | TokenType::Number | TokenType::LParen
                    )
                });
        match missing {
            true => Err(self.parse_error("Expected a value after =")),
            false => Ok(()),
        }
    }

    /// The IF at the cursor, which only EXISTS, NOT EXISTS, `(` or, in `RESUME IF SUSPENDED`, SUSPENDED may follow.
    fn check_opaque_if(&self) -> Result<()> {
        let suspended = self
            .peek_nth(1)
            .is_some_and(|token| token.text.eq_ignore_ascii_case(SUSPENDED));
        match suspended
            || self.is_opaque_token_at(1, TokenType::Exists)
            || self.is_opaque_token_at(1, TokenType::LParen)
            || self.is_opaque_token_at(1, TokenType::Not)
                && self.is_opaque_token_at(2, TokenType::Exists)
        {
            true => Ok(()),
            false => Err(self.parse_error("Expected EXISTS or NOT EXISTS after IF")),
        }
    }
}

/// The words after a property kind that take one value without `=`: an organization profile's or external agent's VERSION, a snapshot's VOLUME and INSTANCE, an AI function's RUN and a listing's SHARE.
const OPAQUE_VALUE_WORDS: [&str; 5] = ["VERSION", "VOLUME", "INSTANCE", "RUN", "SHARE"];

impl Parser {
    /// A Snowflake CREATE the opaque check passed, as a create-object tree: a scoped temporary table, a read-only temporary clone, or a kind the opaque check reads; None keeps the raw text, as for a DEFINE of a kind DEFINE does not take.
    pub(crate) fn parse_opaque_create(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        if let Some(table) = self.parse_scoped_temp_table(or_replace, or_alter)? {
            return Ok(Some(table));
        }
        if !or_alter {
            if let Some(clone) = self.parse_snowflake_read_only_clone(or_replace)? {
                return Ok(Some(clone));
            }
        }
        match self.parse_opaque_create_tree(or_replace, or_alter) {
            Ok(Some(tree)) if self.is_statement_end_at(0) => Ok(Some(tree)),
            _ => {
                self.current = start;
                Ok(None)
            }
        }
    }

    /// The create-object tree of an opaque CREATE from its kind: the modifier written before the kind, IF NOT EXISTS, the name, an AI function's signature and return type, then its clauses and properties.
    fn parse_opaque_create_tree(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let Some(keyword_at) = self.statement_keyword_at(self.current) else {
            return Ok(None);
        };
        let define = self.tokens[keyword_at].token_type == TokenType::Define;
        let mut head = keyword_at + 1;
        if self.tokens[head].token_type == TokenType::Or {
            head += 2;
        }
        let quoted = self.tokens[head..self.current].iter().any(|token| {
            matches!(
                token.token_type,
                TokenType::QuotedIdentifier | TokenType::String
            )
        });
        let words: Vec<String> = self.tokens[head..self.current]
            .iter()
            .map(|token| token.text.to_ascii_uppercase())
            .collect();
        let modifier = match words.is_empty() {
            true => None,
            false => match SNOWFLAKE_OPAQUE_MODIFIERS
                .iter()
                .any(|modifier| *modifier == words.as_slice())
            {
                true if !quoted => Some(words.join(" ")),
                _ => return Ok(None),
            },
        };
        let Some(&kind) = SNOWFLAKE_STRUCTURED_OPAQUE_KINDS.iter().find(|words| {
            self.check_text_seq(words)
                && !(**words == ["NOTEBOOK", "PROJECT"] && self.notebook_name_ends_at(2))
                && !(**words == ["SNAPSHOT"]
                    && self.peek_nth(1).is_some_and(|token| {
                        token.text.eq_ignore_ascii_case("SET")
                            || token.text.eq_ignore_ascii_case("POLICY")
                    }))
        }) else {
            return Ok(None);
        };
        if define && !SNOWFLAKE_DEFINE_OPAQUE_KINDS.contains(&kind) {
            return Ok(None);
        }
        self.current += kind.len();
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Ok(None);
        }
        if kind == ["NOTEBOOK", "PROJECT"] && self.check(TokenType::Dot) {
            return Ok(None);
        }
        let mut create = CreateObject {
            kind: kind.join(" "),
            modifier,
            or_replace,
            or_alter,
            if_not_exists,
            name: self.parse_object_name()?,
            ..Default::default()
        };
        let function = kind == ["AI", "FUNCTION"];
        if function {
            self.expect_token(TokenType::LParen)?;
            create.signature = Some(self.parse_function_parameters()?);
            self.expect_token(TokenType::RParen)?;
            if !self.match_text_seq(&["RETURNS"]) {
                return Ok(None);
            }
            create.returns = Some(self.parse_data_type()?);
        }
        let mut with = false;
        while !self.is_statement_end_at(0) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                if with {
                    return Ok(None);
                }
                self.skip();
                with = true;
                continue;
            }
            let first = create.properties.len();
            create = match self.parse_opaque_create_clause(create, function)? {
                Some(create) => create,
                None => return Ok(None),
            };
            for found in &mut create.properties[first..] {
                found.with = with;
            }
            if with && create.properties.len() == first {
                return Ok(None);
            }
            with = false;
        }
        match with {
            true => Ok(None),
            false => Ok(Some(Expression::CreateObject(Box::new(create)))),
        }
    }

    /// `create` with one more clause or property of an opaque CREATE: FROM and its source, AS and a body or manifest, a tag or contact list, MANAGED ACCESS, COPY GRANTS, an interactive warehouse's TABLES, a word that takes one value, or `<property> = <value>`; None for anything else.
    fn parse_opaque_create_clause(
        &mut self,
        mut create: CreateObject,
        function: bool,
    ) -> Result<Option<CreateObject>> {
        if self.check(TokenType::From) && create.source.is_empty() && create.body.is_none() {
            self.skip();
            let (source, operand) = if self.match_text_seq(&["SPECIFICATION"]) {
                ("FROM SPECIFICATION", self.parse_opaque_text()?)
            } else if self.match_text_seq(&["SERVICE"]) {
                (
                    "FROM SERVICE",
                    Expression::Table(Box::new(self.parse_object_name()?)),
                )
            } else if self.match_text_seq(&["EXPERIMENT"]) {
                (
                    "FROM EXPERIMENT",
                    Expression::Table(Box::new(self.parse_object_name()?)),
                )
            } else if self.is_stage_path() || self.check(TokenType::String) {
                ("FROM", self.parse_stage_path()?)
            } else {
                return Ok(None);
            };
            create.source = source.to_owned();
            create.operands.push(operand);
            return Ok(Some(create));
        }
        if self.check(TokenType::As) && create.source.is_empty() && create.body.is_none() {
            self.skip();
            if self.check(TokenType::Copy) || function {
                let body = match function {
                    true => self.parse_opaque_text()?,
                    false => self.parse_statement()?,
                };
                create.body = Some(Box::new(body));
            } else {
                create.source = "AS".to_owned();
                create.operands.push(self.parse_opaque_text()?);
            }
            return Ok(Some(create));
        }
        if self.check_identifier("TAG") && self.check_next(TokenType::LParen) {
            create.properties.extend(self.parse_create_property_item()?);
            return Ok(Some(create));
        }
        if self.check_identifier("CONTACT") && self.check_next(TokenType::LParen) {
            self.skip();
            self.skip();
            loop {
                let purpose = self.parse_contact_purpose(Self::parse_simple_name)?;
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_object_value()?;
                create
                    .properties
                    .push(named_property("CONTACT", purpose, Some(value)));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(Some(create));
        }
        if let Some(words) = [&["MANAGED", "ACCESS"], &["COPY", "GRANTS"]]
            .into_iter()
            .find(|words| self.match_text_seq(*words))
        {
            create
                .properties
                .push(property(&words.join(" "), None, None));
            return Ok(Some(create));
        }
        if self.check_identifier("TABLES") && self.check_next(TokenType::LParen) {
            self.skip();
            self.skip();
            let mut tables: Vec<Expression> = Vec::new();
            while !self.match_token(TokenType::RParen) {
                tables.push(Expression::Table(Box::new(self.parse_object_name()?)));
                if !self.match_token(TokenType::Comma) {
                    self.expect_token(TokenType::RParen)?;
                    break;
                }
            }
            let tables = Expression::Tuple(Box::new(Tuple {
                expressions: tables,
            }));
            create
                .properties
                .push(property("TABLES", None, Some(tables)));
            return Ok(Some(create));
        }
        let word = self.peek_text().to_ascii_uppercase();
        if self.peek().token_type != TokenType::QuotedIdentifier
            && OPAQUE_VALUE_WORDS.contains(&word.as_str())
            && !self.check_next(TokenType::Eq)
        {
            self.skip();
            let value = self.parse_object_value()?;
            create.properties.push(property(&word, None, Some(value)));
            return Ok(Some(create));
        }
        if self.check_next(TokenType::Eq) {
            create.properties.extend(self.parse_create_property_item()?);
            return Ok(Some(create));
        }
        Ok(None)
    }

    /// A specification, manifest or body written as a string or dollar-quoted text.
    fn parse_opaque_text(&mut self) -> Result<Expression> {
        if !(self.check(TokenType::String) || self.check(TokenType::DollarString)) {
            return Err(self.parse_error("Expected a string or dollar-quoted text"));
        }
        self.parse_primary()
    }
}
