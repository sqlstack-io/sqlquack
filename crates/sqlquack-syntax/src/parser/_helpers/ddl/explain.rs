//! Parser ddl: PostgreSQL's EXPLAIN option list.

use crate::parser::constants::EXPLAIN_ANALYZE;
use crate::parser::constants::EXPLAIN_TRUE_VALUES;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Reads PostgreSQL's `EXPLAIN ( option [, ...] )` list; true when it turns ANALYZE on.
    pub(crate) fn parse_postgres_explain_options(&mut self) -> Result<bool> {
        let mut analyze_option = false;
        if self.check(TokenType::LParen)
            && self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL)
            && !matches!(
                self.peek_nth(1).map(|t| t.token_type),
                Some(TokenType::Select)
                    | Some(TokenType::With)
                    | Some(TokenType::LParen)
                    | Some(TokenType::Values)
                    | Some(TokenType::Table)
            )
        {
            self.advance()?;
            loop {
                if self.check(TokenType::RParen) || self.is_at_end() {
                    break;
                }
                let name = self.advance()?.text.to_ascii_uppercase();
                let value = if !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                    Some(self.advance()?.text.to_ascii_uppercase())
                } else {
                    None
                };
                if name == EXPLAIN_ANALYZE {
                    analyze_option = match value.as_deref() {
                        None => true,
                        Some(v) => EXPLAIN_TRUE_VALUES.contains(&v),
                    };
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        Ok(analyze_option)
    }
}
