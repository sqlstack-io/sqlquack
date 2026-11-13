//! Parser ddl: the shared check of a Snowflake CREATE or ALTER kept as opaque text, which refuses a malformed head, `NAME = value` option list or tag list while the tree stays the same text.

use crate::parser::_helpers::statements::analyze::snowflake_reserved_word;
use crate::parser::constants::{
    SNOWFLAKE_DROP_KINDS, SNOWFLAKE_OPAQUE_ALTER_ACTIONS, SNOWFLAKE_OPAQUE_KINDS,
    SNOWFLAKE_OPAQUE_OPERAND_WORDS, SNOWFLAKE_OPAQUE_UNNAMED_KIND, SUSPENDED,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tree::models::Expression;
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

    /// After a CREATE's name, the shape its kind needs: FROM for a code bundle or notebook project, a snapshot's service volume, an external agent's version, AS COPY for a pipe's body, and an Openflow connector's added version; the cursor stays after the name.
    fn check_opaque_shape(&mut self, create: bool, kind_at: usize) -> Result<()> {
        let after_name = self.current;
        let kind = self.tokens[kind_at].text.to_ascii_uppercase();
        let second = self.tokens[kind_at + 1].text.to_ascii_uppercase();
        if !create {
            let connector = matches!((kind.as_str(), second.as_str()), ("OPENFLOW", "CONNECTOR"));
            let checked = match connector && self.check_text_seq(&["ADD"]) {
                true => self.check_connector_version(),
                false => Ok(()),
            };
            self.current = after_name;
            return checked;
        }
        let project = matches!(second.as_str(), "PROJECT");
        let checked = match kind.as_str() {
            "EXTERNAL" if matches!(second.as_str(), "AGENT") => self.check_external_agent_tail(),
            "SNAPSHOT" if !matches!(second.as_str(), "SET" | "POLICY") => {
                self.check_snapshot_source()
            }
            "CODE" if !self.check(TokenType::From) => Err(self.parse_error("Expected FROM")),
            "NOTEBOOK" if project && !self.check(TokenType::From) => {
                Err(self.parse_error("Expected FROM"))
            }
            "PIPE" => self.check_pipe_body(),
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
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.skip();
        }
        let sql = self.tokens_to_sql(start, self.current);
        Ok(Expression::Raw(Raw {
            sql: format!("ALTER {sql}"),
        }))
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

    /// The kind, `IF [ NOT ] EXISTS` and name of a kind the opaque check knows, a notebook project's name with its database left out, then an ALTER's action; nothing for another kind.
    fn check_opaque_head(&mut self, create: bool) -> Result<()> {
        let kind = SNOWFLAKE_OPAQUE_KINDS
            .iter()
            .chain(SNOWFLAKE_DROP_KINDS)
            .find(|words| {
                self.check_text_seq(words)
                    && !(**words == ["MODEL", "MONITOR"]
                        && self.model_name_ends_at(self.current + 2))
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
