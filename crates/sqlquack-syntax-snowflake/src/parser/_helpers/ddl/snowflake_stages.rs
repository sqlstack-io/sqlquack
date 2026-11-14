//! Parser ddl: Snowflake stage locations, as PUT, GET, LIST, REMOVE and COPY write them, and the stage commands' options.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::stages::models::StageName;
use sqlquack_syntax::expressions::statements::models::CopyParameter;
use sqlquack_syntax::expressions::statements::models::StageReference;
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

/// A stage location's written name split into its parts, and the text after the name, such as a `/` path.
pub(crate) fn stage_name_fields(text: &str) -> (StageName, &str) {
    let body = text.strip_prefix('@').unwrap_or(text);
    if let Some(rest) = body.strip_prefix('~') {
        let name = StageName {
            parts: Vec::new(),
            table_stage: false,
            user_stage: true,
        };
        return (name, rest);
    }
    let mut parts: Vec<Option<Identifier>> = Vec::new();
    let mut table_stage = false;
    let mut at = 0;
    loop {
        if body[at..].starts_with('%') {
            table_stage = true;
            at += 1;
        }
        if body[at..].starts_with('"') {
            let mut name = String::new();
            let mut chars = body[at + 1..].char_indices().peekable();
            let mut end = body.len();
            while let Some((offset, character)) = chars.next() {
                if character != '"' {
                    name.push(character);
                    continue;
                }
                if chars.peek().is_some_and(|(_, next)| *next == '"') {
                    chars.next();
                    name.push('"');
                    continue;
                }
                end = at + 1 + offset + 1;
                break;
            }
            at = end;
            parts.push(Some(Identifier::quoted(name)));
        } else {
            let end = body[at..]
                .find(['.', '/'])
                .map_or(body.len(), |offset| at + offset);
            let name = &body[at..end];
            parts.push((!name.is_empty()).then(|| Identifier::new(name)));
            at = end;
        }
        match body[at..].strip_prefix('.') {
            Some(_) => at += 1,
            None => break,
        }
    }
    let name = StageName {
        parts,
        table_stage,
        user_stage: false,
    };
    (name, &body[at..])
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
