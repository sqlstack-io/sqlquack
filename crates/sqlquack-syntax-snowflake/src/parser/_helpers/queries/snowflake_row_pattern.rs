//! Snowflake's MATCH_RECOGNIZE row pattern, read as a tree.

use crate::parser::constants::ROW_PATTERN_QUESTION_MARK;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::queries::models::{RowPattern, RowPatternQuantifier};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake `( <pattern> )` after PATTERN, as a tree.
    pub(crate) fn parse_snowflake_row_pattern(&mut self) -> Result<RowPattern> {
        self.expect_token(TokenType::LParen)?;
        let pattern = self.parse_row_pattern_alternation()?;
        self.expect_token(TokenType::RParen)?;
        Ok(pattern)
    }

    /// Sequences separated by `|`; one sequence stands alone.
    fn parse_row_pattern_alternation(&mut self) -> Result<RowPattern> {
        let mut alternatives = vec![self.parse_row_pattern_sequence()?];
        while self.match_token(TokenType::Pipe) {
            alternatives.push(self.parse_row_pattern_sequence()?);
        }
        Ok(match alternatives.len() {
            1 => alternatives.remove(0),
            _ => RowPattern::Alternation(alternatives),
        })
    }

    /// Quantified terms up to `|`, `,`, `)` or `-}`; one term stands alone.
    fn parse_row_pattern_sequence(&mut self) -> Result<RowPattern> {
        let mut terms: Vec<RowPattern> = Vec::new();
        while !self.is_at_end()
            && !self.check(TokenType::Pipe)
            && !self.check(TokenType::Comma)
            && !self.check(TokenType::RParen)
            && !(self.check(TokenType::Dash) && self.check_next(TokenType::RBrace))
        {
            let term = self.parse_row_pattern_primary()?;
            terms.push(self.parse_row_pattern_quantifier(term)?);
        }
        Ok(match terms.len() {
            1 => terms.remove(0),
            _ => RowPattern::Sequence(terms),
        })
    }

    /// An anchor, a variable, a group, an exclusion or PERMUTE.
    fn parse_row_pattern_primary(&mut self) -> Result<RowPattern> {
        if self.match_token(TokenType::Caret) {
            return Ok(RowPattern::Start);
        }
        if self.match_token(TokenType::Dollar) {
            return Ok(RowPattern::End);
        }
        if self.check(TokenType::LParen) {
            self.skip();
            let inner = self.with_parser_depth(Self::parse_row_pattern_alternation)?;
            self.expect_token(TokenType::RParen)?;
            return Ok(RowPattern::Group(Box::new(inner)));
        }
        if self.check(TokenType::LBrace) && self.check_next(TokenType::Dash) {
            self.current += 2;
            let inner = self.with_parser_depth(Self::parse_row_pattern_alternation)?;
            self.expect_token(TokenType::Dash)?;
            self.expect_token(TokenType::RBrace)?;
            return Ok(RowPattern::Exclusion(Box::new(inner)));
        }
        if self.check_identifier("PERMUTE") && self.check_next(TokenType::LParen) {
            self.current += 2;
            let mut members = vec![self.with_parser_depth(Self::parse_row_pattern_alternation)?];
            while self.match_token(TokenType::Comma) {
                members.push(self.with_parser_depth(Self::parse_row_pattern_alternation)?);
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(RowPattern::Permute(members));
        }
        self.parse_row_pattern_symbol()
    }

    /// A pattern variable: a quoted identifier or any word, as written.
    fn parse_row_pattern_symbol(&mut self) -> Result<RowPattern> {
        if self.check(TokenType::QuotedIdentifier) {
            return Ok(RowPattern::Symbol(
                self.parse_quoting_identifier(Self::expect_identifier)?,
            ));
        }
        let token = self.peek();
        let word = token
            .text
            .starts_with(|first: char| first.is_ascii_alphabetic() || first == '_');
        if !word {
            return Err(self.parse_error("Expected a pattern variable"));
        }
        let identifier = Identifier::new(token.text.to_string()).with_span(token.span);
        self.skip();
        Ok(RowPattern::Symbol(identifier))
    }

    /// `term` with the quantifier written after it, if any.
    fn parse_row_pattern_quantifier(&mut self, term: RowPattern) -> Result<RowPattern> {
        let (quantifier, reluctant_written) = if self.match_token(TokenType::Star) {
            (RowPatternQuantifier::ZeroOrMore, false)
        } else if self.match_token(TokenType::Plus) {
            (RowPatternQuantifier::OneOrMore, false)
        } else if self.match_token(TokenType::DQMark) {
            (RowPatternQuantifier::ZeroOrOne, true)
        } else if self.match_question_mark() {
            (RowPatternQuantifier::ZeroOrOne, false)
        } else if self.check(TokenType::LBrace) && !self.check_next(TokenType::Dash) {
            self.skip();
            (self.parse_row_pattern_bounds()?, false)
        } else {
            return Ok(term);
        };
        let reluctant = reluctant_written || self.match_question_mark();
        Ok(RowPattern::Quantified {
            this: Box::new(term),
            quantifier,
            reluctant,
        })
    }

    /// Consumes a lone `?`.
    fn match_question_mark(&mut self) -> bool {
        let question =
            self.check(TokenType::Parameter) && self.peek().text == ROW_PATTERN_QUESTION_MARK;
        if question {
            self.skip();
        }
        question
    }

    /// `n }`, `n , }`, `, m }` or `n , m }` after `{`.
    fn parse_row_pattern_bounds(&mut self) -> Result<RowPatternQuantifier> {
        let min = self.parse_row_pattern_bound()?;
        if !self.match_token(TokenType::Comma) {
            self.expect_token(TokenType::RBrace)?;
            return match min {
                Some(count) => Ok(RowPatternQuantifier::Exactly(count)),
                None => Err(self.parse_error("Expected a count in a quantifier")),
            };
        }
        let max = self.parse_row_pattern_bound()?;
        self.expect_token(TokenType::RBrace)?;
        Ok(RowPatternQuantifier::Between { min, max })
    }

    /// A whole-number bound, or None when none is written.
    fn parse_row_pattern_bound(&mut self) -> Result<Option<u64>> {
        if !self.check(TokenType::Number) {
            return Ok(None);
        }
        let bound = self
            .peek()
            .text
            .parse::<u64>()
            .map_err(|_| self.parse_error("Expected a whole number in a quantifier"))?;
        self.skip();
        Ok(Some(bound))
    }
}
