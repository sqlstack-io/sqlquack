//! Parser ddl: the shared check of a Snowflake CREATE or ALTER kept as opaque text, which refuses a malformed head, `NAME = value` option list or tag list while the tree stays the same text.

use crate::parser::_helpers::statements::analyze::snowflake_reserved_word;
use crate::parser::constants::{
    SNOWFLAKE_DROP_KINDS, SNOWFLAKE_OPAQUE_ALTER_ACTIONS, SNOWFLAKE_OPAQUE_KINDS,
    SNOWFLAKE_OPAQUE_OPERAND_WORDS, SNOWFLAKE_OPAQUE_RENAME_WITHOUT_TO,
    SNOWFLAKE_OPAQUE_UNNAMED_KIND, SUSPENDED,
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
        let checked = self
            .check_opaque_head(create)
            .and_then(|kind| self.check_opaque_options(kind));
        self.current = start;
        checked
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

    /// The kind, `IF [ NOT ] EXISTS` and name of a kind the opaque check knows, then an ALTER's action; nothing for another kind. Returns the kind.
    fn check_opaque_head(&mut self, create: bool) -> Result<Option<&'static [&'static str]>> {
        let kind = SNOWFLAKE_OPAQUE_KINDS
            .iter()
            .chain(SNOWFLAKE_DROP_KINDS)
            .find(|words| self.check_text_seq(words));
        let Some(&kind) = kind else {
            return self.check_opaque_unknown_kind().map(|()| None);
        };
        self.current += kind.len();
        if !create && self.check(TokenType::If) && self.is_opaque_action_at(1) {
            self.skip();
            return Ok(Some(kind));
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
            return self.parse_object_name().map(|_| Some(kind));
        }
        if kind == SNOWFLAKE_OPAQUE_UNNAMED_KIND && self.is_opaque_action_at(0) {
            return Ok(Some(kind));
        }
        self.parse_object_name()?;
        match self.is_opaque_action_at(0) {
            true => Ok(Some(kind)),
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
    fn check_opaque_options(&mut self, kind: Option<&[&str]>) -> Result<()> {
        let rename_without_to =
            kind.is_some_and(|kind| SNOWFLAKE_OPAQUE_RENAME_WITHOUT_TO.contains(&kind));
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
                    if !self.match_token(TokenType::To) && !rename_without_to {
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
