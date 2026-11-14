//! Parser ddl: Snowflake stage locations, as PUT, GET, LIST, REMOVE and COPY write them, and the stage commands' options.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::stages::main::stage_name_fields::stage_name_fields;
use sqlquack_syntax::expressions::stages::models::StageReference;
use sqlquack_syntax::expressions::statements::models::CopyParameter;
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Reads a stage location for PUT, GET, LIST or REMOVE: `@...` written out, or a string literal starting with `@`.
    pub(crate) fn parse_stage_location(&mut self) -> Result<Expression> {
        if self.check(TokenType::String) && self.peek_text().starts_with('@') {
            let text = self.advance_text()?;
            return Ok(stage_reference(&text, true));
        }
        if self.check(TokenType::DAt) {
            self.skip();
            let mut stage_path = String::from("@");
            let adjacent = self.copy_adjacent(self.current);
            if adjacent && (self.check(TokenType::Percent) || self.check(TokenType::Mod)) {
                stage_path.push('%');
                self.skip();
            } else if adjacent && self.check(TokenType::Tilde) {
                stage_path.push('~');
                self.skip();
                stage_path.push_str(&self.stage_path_segments()?);
                return Ok(stage_reference(&stage_path, false));
            }
            stage_path.push_str(&self.stage_name_part()?);
            stage_path.push_str(&self.stage_dotted_parts()?);
            stage_path.push_str(&self.stage_path_segments()?);
            return Ok(stage_reference(&stage_path, false));
        }

        if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
            let mut stage_path = self.advance_text()?;
            stage_path.push_str(&self.stage_dotted_parts()?);
            stage_path.push_str(&self.stage_path_segments()?);
            return Ok(stage_reference(&stage_path, false));
        }

        Err(self.parse_error("Expected stage reference starting with @"))
    }

    /// The `NAME = value` options after a stage command's locations; a bare word value, such as `SOURCE_COMPRESSION = GZIP`, is a keyword.
    pub(crate) fn parse_stage_command_params(&mut self) -> Result<Vec<CopyParameter>> {
        let mut params: Vec<CopyParameter> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let is_param_name = self.check(TokenType::Var)
                || self.check_keyword()
                || self.check(TokenType::Overwrite);
            if !is_param_name {
                break;
            }
            let name = self.advance_text()?;
            let eq = self.match_token(TokenType::Eq);
            let keyword = (self.check(TokenType::Var) || self.check_keyword())
                && !self.check(TokenType::True)
                && !self.check(TokenType::False)
                && !self.check_next(TokenType::Dot)
                && !self.check_next(TokenType::LParen);
            let value = match (eq, keyword) {
                (false, _) => None,
                (true, true) => Some(Expression::Var(Box::new(Var {
                    this: self.advance_text()?,
                }))),
                (true, false) => Some(self.parse_primary()?),
            };
            params.push(CopyParameter {
                name,
                value,
                values: Vec::new(),
                eq,
            });
        }
        Ok(params)
    }

    /// One stage name part, quoted or not, or nothing when the current token is not one.
    fn stage_name_part(&mut self) -> Result<String> {
        if !self.copy_adjacent(self.current) {
            return Ok(String::new());
        }
        if self.check(TokenType::QuotedIdentifier) {
            let text = format!("\"{}\"", self.peek_text());
            self.skip();
            return Ok(text);
        }
        match self.check(TokenType::Var)
            || self.check_keyword()
            || self.check(TokenType::Identifier)
        {
            true => self.advance_text(),
            false => Ok(String::new()),
        }
    }

    /// Each `.part` of a stage name, where a part may be a table stage `%name`.
    fn stage_dotted_parts(&mut self) -> Result<String> {
        let mut parts = String::new();
        while self.copy_adjacent(self.current) && self.match_token(TokenType::Dot) {
            parts.push('.');
            if self.copy_adjacent(self.current)
                && (self.check(TokenType::Percent) || self.check(TokenType::Mod))
            {
                parts.push('%');
                self.skip();
            }
            parts.push_str(&self.stage_name_part()?);
        }
        Ok(parts)
    }

    /// Each `/segment` of a stage path.
    fn stage_path_segments(&mut self) -> Result<String> {
        let mut segments = String::new();
        while self.copy_adjacent(self.current) && self.match_token(TokenType::Slash) {
            segments.push('/');
            while self.copy_adjacent(self.current)
                && (self.check(TokenType::Var)
                    || self.check_keyword()
                    || self.is_identifier_token()
                    || self.check(TokenType::Number)
                    || self.check(TokenType::Dash))
                && !self.check_next(TokenType::Eq)
            {
                segments.push_str(&self.advance_text()?);
            }
        }
        Ok(segments)
    }
}

/// A stage reference node for a stage location's text, such as `@v1.%v1/p1`, split by `stage_name_fields`.
pub(crate) fn stage_reference(text: &str, quoted: bool) -> Expression {
    let (stage_name, rest) = stage_name_fields(text);
    Expression::StageReference(Box::new(StageReference {
        name: text[..text.len() - rest.len()].to_string(),
        path: (!rest.is_empty()).then(|| rest.to_string()),
        file_format: None,
        pattern: None,
        quoted,
        stage_name: Some(stage_name),
    }))
}
