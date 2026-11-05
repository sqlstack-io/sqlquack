//! Transaction statements: START, BEGIN, COMMIT, END and ROLLBACK.

use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::statements::models::Commit;
use crate::expressions::statements::models::Rollback;
use crate::expressions::statements::models::Transaction;
use crate::expressions::statements::models::TryCatch;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parses START TRANSACTION [READ ONLY | READ WRITE] [, ISOLATION LEVEL ...].
    pub(crate) fn parse_start_transaction(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Start)?;

        self.expect_token(TokenType::Transaction)?;
        self.refuse_snowflake_nameless_transaction()?;
        if self.is_snowflake() && self.match_identifier("NAME") {
            let name = self.expect_identifier_or_safe_keyword_with_quoted()?;
            return Ok(Expression::Transaction(Box::new(Transaction {
                this: Some(Box::new(Expression::Identifier(name))),
                modes: None,
                mark: Some(Box::new(Expression::Identifier(Identifier::new(
                    "START".to_string(),
                )))),
            })));
        }

        let mut mode_parts: Vec<String> = Vec::new();
        while self.is_identifier_token()
            || self.is_safe_keyword_as_identifier()
            || self.match_identifier("READ")
        {
            let read_matched = self.previous_text().eq_ignore_ascii_case("READ");
            let mut mode_tokens: Vec<String> = Vec::new();
            if read_matched {
                mode_tokens.push("READ".to_string());
            }
            while (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
                && !self.check(TokenType::Comma)
            {
                mode_tokens.push(self.advance_text()?);
            }
            if !mode_tokens.is_empty() {
                mode_parts.push(mode_tokens.join(" "));
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let modes = if !mode_parts.is_empty() {
            Some(Box::new(Expression::Identifier(Identifier::new(
                mode_parts.join(", "),
            ))))
        } else {
            None
        };

        Ok(Expression::Transaction(Box::new(Transaction {
            this: None,
            modes,
            mark: Some(Box::new(Expression::Identifier(Identifier::new(
                "START".to_string(),
            )))),
        })))
    }

    /// Parses ROLLBACK [TO [SAVEPOINT] name].
    pub(crate) fn parse_rollback(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Rollback)?;

        let has_transaction = self.match_token(TokenType::Transaction)
            || self.match_identifier("TRAN")
            || self.match_identifier("WORK");

        let (savepoint, this) = if self.match_token(TokenType::To) {
            self.match_token(TokenType::Savepoint);
            if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let name = self.advance_text()?;
                (
                    Some(Box::new(Expression::Identifier(Identifier::new(name)))),
                    None,
                )
            } else {
                (None, None)
            }
        } else if has_transaction
            && (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
        {
            let name = self.advance_text()?;
            (
                None,
                Some(Box::new(Expression::Identifier(Identifier::new(name)))),
            )
        } else if has_transaction {
            (
                None,
                Some(Box::new(Expression::Identifier(Identifier::new(
                    "TRANSACTION".to_string(),
                )))),
            )
        } else {
            (None, None)
        };

        Ok(Expression::Rollback(Box::new(Rollback { savepoint, this })))
    }

    /// Parses PostgreSQL's END [WORK|TRANSACTION] [AND [NO] CHAIN], an alias of COMMIT.
    pub(crate) fn parse_end_transaction(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::End)?;
        let _has_work = self.match_identifier("WORK") || self.match_token(TokenType::Transaction);
        let chain = if self.match_token(TokenType::And) {
            let no_chain = self.match_token(TokenType::No);
            self.match_identifier("CHAIN");
            Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: !no_chain,
            })))
        } else {
            None
        };
        Ok(Expression::Commit(Box::new(Commit {
            chain,
            this: None,
            durability: None,
        })))
    }

    /// Parses COMMIT [TRANSACTION|WORK] [name] [WITH (...)] [AND [NO] CHAIN].
    pub(crate) fn parse_commit(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Commit)?;

        let has_transaction = self.match_token(TokenType::Transaction)
            || self.match_identifier("TRAN")
            || self.match_identifier("WORK");

        let this = if has_transaction
            && (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
            && !self.check(TokenType::With)
            && !self.check(TokenType::And)
        {
            let name = self.advance_text()?;
            Some(Box::new(Expression::Identifier(Identifier::new(name))))
        } else if has_transaction {
            Some(Box::new(Expression::Identifier(Identifier::new(
                "TRANSACTION".to_string(),
            ))))
        } else {
            None
        };

        let durability = if self.match_token(TokenType::With) && self.match_token(TokenType::LParen)
        {
            if self.match_identifier("DELAYED_DURABILITY") && self.match_token(TokenType::Eq) {
                let on = self.match_token(TokenType::On) || self.match_identifier("ON");
                if !on {
                    self.match_identifier("OFF");
                }
                self.expect_token(TokenType::RParen)?;
                Some(Box::new(Expression::Boolean(BooleanLiteral { value: on })))
            } else {
                while !self.check(TokenType::RParen) && !self.is_at_end() {
                    self.skip();
                }
                self.match_token(TokenType::RParen);
                None
            }
        } else {
            None
        };

        let chain = if self.match_token(TokenType::And) {
            let no_chain = self.match_token(TokenType::No);
            self.match_identifier("CHAIN");
            if no_chain {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: false,
                })))
            } else {
                Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                })))
            }
        } else {
            None
        };

        Ok(Expression::Commit(Box::new(Commit {
            chain,
            this,
            durability,
        })))
    }

    /// Parses BEGIN/START TRANSACTION, and procedural BEGIN ... END blocks.
    pub(crate) fn parse_transaction(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Begin)?;

        let is_transaction = self.is_at_end()
            || self.check(TokenType::Semicolon)
            || self.check(TokenType::Transaction)
            || self.check_identifier("TRAN")
            || self.check_identifier("WORK")
            || self.check_identifier("DEFERRED")
            || self.check_identifier("IMMEDIATE")
            || self.check_identifier("EXCLUSIVE");

        if !is_transaction {
            if self.check_identifier("TRY") {
                return self.parse_tsql_try_catch();
            }
            if self.check_identifier("CATCH") {
                self.skip();
                let catch_body = self.parse_tsql_block_body("CATCH")?;
                return Ok(Expression::TryCatch(Box::new(TryCatch {
                    try_body: Vec::new(),
                    catch_body: Some(catch_body),
                })));
            }

            return self
                .parse_command()?
                .ok_or_else(|| self.parse_error("Failed to parse BEGIN block"));
        }

        let kind = if self.match_identifier("DEFERRED")
            || self.match_identifier("IMMEDIATE")
            || self.match_identifier("EXCLUSIVE")
        {
            Some(self.previous_text().to_string())
        } else {
            None
        };

        let has_transaction_keyword = self.match_token(TokenType::Transaction)
            || self.match_identifier("TRAN")
            || self.match_identifier("WORK");

        let trans_name = if has_transaction_keyword
            && (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
            && !self.check(TokenType::With)
        {
            let name = self.advance_text()?;
            Some(name)
        } else {
            None
        };

        let this = if let Some(name) = trans_name {
            Some(Box::new(Expression::Identifier(Identifier::new(name))))
        } else {
            kind.map(|k| Box::new(Expression::Identifier(Identifier::new(k))))
        };

        let mark = if self.match_token(TokenType::With) && self.match_identifier("MARK") {
            if self.check(TokenType::String) {
                let desc = self.advance_text()?;
                Some(Box::new(Expression::Literal(Box::new(Literal::String(
                    desc,
                )))))
            } else {
                Some(Box::new(Expression::Literal(Box::new(Literal::String(
                    "".to_string(),
                )))))
            }
        } else if has_transaction_keyword {
            Some(Box::new(Expression::Identifier(Identifier::new(
                "TRANSACTION".to_string(),
            ))))
        } else {
            None
        };

        let mut mode_parts: Vec<String> = Vec::new();
        while self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            let mut mode_tokens: Vec<String> = Vec::new();
            while (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
                && !self.check(TokenType::Comma)
            {
                mode_tokens.push(self.advance_text()?);
            }
            if !mode_tokens.is_empty() {
                mode_parts.push(mode_tokens.join(" "));
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        let modes = if !mode_parts.is_empty() {
            Some(Box::new(Expression::Identifier(Identifier::new(
                mode_parts.join(", "),
            ))))
        } else {
            None
        };

        Ok(Expression::Transaction(Box::new(Transaction {
            this,
            modes,
            mark,
        })))
    }
}
