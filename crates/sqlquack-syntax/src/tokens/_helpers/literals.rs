//! Tokens: literals.

use std::collections::HashMap;

use crate::tokens::constants::INSERT_FORMAT_MIN_TOKENS;
use crate::tokens::constants::LOWERCASE_ESCAPE_STRING_PREFIX;
use crate::tokens::constants::MAX_KEYWORD_LEN;
use crate::tokens::constants::TRIPLE_QUOTE_LEN;
use crate::tokens::errors::Error;
use crate::tokens::models::Span;
use crate::tokens::models::TokenSource;
use crate::tokens::models::TokenType;
use crate::tokens::models::TokenizerState;
use crate::tokens::types::Result;
use crate::tokens::types::TokenOutput;
use crate::tokens::types::TokenizerCursor;

impl<'a, C: TokenizerCursor, T: TokenOutput> TokenizerState<'a, C, T> {
    pub(crate) fn scan_string(&mut self) -> Result<()> {
        self.advance();
        if let Some((text_start, text_end)) =
            self.try_scan_simple_quoted_content('\'', self.config.string_escapes.contains(&'\\'))
        {
            self.add_token_from_source(TokenType::String, text_start, text_end);
            return Ok(());
        }
        let mut value = String::new();

        while !self.is_at_end() {
            let c = self.peek();
            if c == '\'' {
                if self.peek_next() == '\'' {
                    value.push('\'');
                    self.advance();
                    self.advance();
                } else {
                    break;
                }
            } else if c == '\\' && self.config.string_escapes.contains(&'\\') {
                if self.config.recover_terminal_backslash_quote
                    && self.peek_next() == '\''
                    && !self.range_contains(self.current + 2, '\'')
                {
                    value.push(self.advance());
                    break;
                }

                value = self.scan_backslash_escape(value);
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            if self.config.recover_unterminated_string {
                self.add_token_with_text(TokenType::String, value);
                return Ok(());
            }

            return Err(Error::tokenize(
                "Unterminated string",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        self.advance();
        self.add_token_with_text(TokenType::String, value);
        Ok(())
    }

    /// Scan a double-quoted string (for dialects like BigQuery where " is a string delimiter)
    pub(crate) fn scan_double_quoted_string(&mut self) -> Result<()> {
        self.advance();
        let mut value = String::new();

        while !self.is_at_end() {
            let c = self.peek();
            if c == '"' {
                if self.peek_next() == '"' {
                    value.push('"');
                    self.advance();
                    self.advance();
                } else {
                    break;
                }
            } else if c == '\\' && self.config.string_escapes.contains(&'\\') {
                value = self.scan_backslash_escape(value);
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated double-quoted string",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        self.advance();
        self.add_token_with_text(TokenType::String, value);
        Ok(())
    }

    pub(crate) fn scan_backslash_escape(&mut self, mut value: String) -> String {
        self.advance();
        if self.is_at_end() {
            value.push('\\');
            return value;
        }

        let escaped = self.advance();
        let restricted = !self.config.escape_follow_chars.is_empty();
        let always_allowed = matches!(escaped, '\\' | '\'' | '"');
        if restricted && !always_allowed && !self.config.escape_follow_chars.contains(&escaped) {
            value.push(escaped);
            return value;
        }

        let supports_octal =
            restricted && ('1'..='7').any(|digit| self.config.escape_follow_chars.contains(&digit));
        if supports_octal && escaped.is_digit(8) {
            if let Some(codepoint) = self.peek_radix_digits(2, 8).and_then(|suffix| {
                let first = escaped.to_digit(8)?;
                first.checked_mul(64)?.checked_add(suffix)
            }) {
                if let Ok(byte) = u8::try_from(codepoint) {
                    self.advance_count(2);
                    value.push(byte as char);
                    return value;
                }
            }

            if escaped == '0' {
                value.push('\0');
            } else {
                value.push(escaped);
            }
            return value;
        }

        match escaped {
            'n' => value.push('\n'),
            'r' => value.push('\r'),
            't' => value.push('\t'),
            '0' => value.push('\0'),
            'Z' => value.push('\x1A'),
            'a' => value.push('\x07'),
            'b' => value.push('\x08'),
            'f' => value.push('\x0C'),
            'v' => value.push('\x0B'),
            'x' => {
                if let Some(codepoint) = self.peek_radix_digits(2, 16) {
                    self.advance_count(2);
                    value.push(codepoint as u8 as char);
                } else if restricted {
                    value.push('x');
                } else {
                    value.push('\\');
                    value.push('x');
                    for _ in 0..2 {
                        if !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                            value.push(self.advance());
                        }
                    }
                }
            }
            'u' if restricted && self.config.escape_follow_chars.contains(&'u') => {
                if let Some(codepoint) = self.peek_radix_digits(4, 16).and_then(char::from_u32) {
                    self.advance_count(4);
                    value.push(codepoint);
                } else {
                    value.push('u');
                }
            }
            '\\' => value.push('\\'),
            '\'' => value.push('\''),
            '"' => value.push('"'),
            '%' => value.push('%'),
            '_' => value.push('_'),
            _ if restricted => value.push(escaped),
            _ => {
                value.push('\\');
                value.push(escaped);
            }
        }
        value
    }

    pub(crate) fn peek_radix_digits(&self, count: usize, radix: u32) -> Option<u32> {
        if self.current + count > self.size {
            return None;
        }

        let mut value = 0_u32;
        for offset in 0..count {
            value = value
                .checked_mul(radix)?
                .checked_add(self.char_at(self.current + offset).to_digit(radix)?)?;
        }
        Some(value)
    }

    pub(crate) fn advance_count(&mut self, count: usize) {
        for _ in 0..count {
            self.advance();
        }
    }

    pub(crate) fn scan_triple_quoted_string(&mut self, quote_char: char) -> Result<()> {
        self.advance();
        self.advance();
        self.advance();
        let mut value = String::new();

        while !self.is_at_end() {
            if self.peek() == quote_char
                && self.current + 1 < self.size
                && self.char_at(self.current + 1) == quote_char
                && self.current + 2 < self.size
                && self.char_at(self.current + 2) == quote_char
            {
                break;
            }
            if self.peek() == '\\' && self.config.string_escapes.contains(&'\\') {
                value = self.scan_backslash_escape(value);
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated triple-quoted string",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        self.advance();
        self.advance();
        self.advance();
        let token_type = if quote_char == '"' {
            TokenType::TripleDoubleQuotedString
        } else {
            TokenType::TripleSingleQuotedString
        };
        self.add_token_with_text(token_type, value);
        Ok(())
    }

    pub(crate) fn scan_quoted_identifier(&mut self, end_quote: char) -> Result<()> {
        self.advance();
        let mut value = String::new();

        loop {
            if self.is_at_end() {
                return Err(Error::tokenize(
                    "Unterminated identifier",
                    Span {
                        start: self.start,
                        end: self.current,
                        line: self.line,
                        column: self.column,
                    },
                ));
            }
            if self.config.identifier_backslash_escapes && self.peek() == '\\' {
                if self.peek_next() == end_quote {
                    value.push(end_quote);
                    self.advance_count(2);
                } else {
                    value = self.scan_backslash_escape(value);
                }
                continue;
            }
            if self.peek() == end_quote {
                if self.peek_next() == end_quote {
                    value.push(end_quote);
                    self.advance();
                    self.advance();
                } else {
                    break;
                }
            } else {
                value.push(self.peek());
                self.advance();
            }
        }

        self.advance();
        self.add_token_with_text(TokenType::QuotedIdentifier, value);
        Ok(())
    }

    /// Scans a string in curly single quotes, literal and closed by its matching quote.
    pub(crate) fn scan_unicode_quoted_string(&mut self) -> Result<()> {
        self.advance();
        let start = self.current;
        let close_quote = '\u{2019}';
        while !self.is_at_end() && self.peek() != close_quote {
            self.advance();
        }
        let value = self.text_from_range(start, self.current);
        if !self.is_at_end() {
            self.advance();
        }
        self.add_token_with_text(TokenType::String, value);
        Ok(())
    }

    /// Scans an identifier in curly double quotes, closed by its matching quote.
    pub(crate) fn scan_unicode_quoted_identifier(&mut self) -> Result<()> {
        self.advance();
        let start = self.current;
        let close_quote = '\u{201D}';
        while !self.is_at_end() && self.peek() != close_quote && self.peek() != '"' {
            self.advance();
        }
        let value = self.text_from_range(start, self.current);
        if !self.is_at_end() {
            self.advance();
        }
        self.add_token_with_text(TokenType::QuotedIdentifier, value);
        Ok(())
    }

    pub(crate) fn starts_numeric_exponent(&self) -> bool {
        if !matches!(self.peek(), 'e' | 'E') {
            return false;
        }
        let digits = self.current + 1 + usize::from(matches!(self.peek_next(), '+' | '-'));
        digits < self.size && self.char_at(digits).is_ascii_digit()
    }

    pub(crate) fn scan_number(&mut self) -> Result<()> {
        if self.config.hex_number_strings
            && self.peek() == '0'
            && !self.is_at_end()
            && self.try_scan_hex_number_string()?
        {
            return Ok(());
        }

        if !self.advance_ascii_digits() {
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                if self.peek() == '_' && (self.is_at_end() || !self.peek_next().is_ascii_digit()) {
                    break;
                }
                self.advance();
            }
        }

        if self.peek() == '.' {
            let next = self.peek_next();
            if next != '.' {
                self.advance();
                if !self.advance_ascii_digits() {
                    while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_')
                    {
                        if self.peek() == '_' && !self.peek_next().is_ascii_digit() {
                            break;
                        }
                        self.advance();
                    }
                }
            }
        }

        if self.starts_numeric_exponent() {
            self.advance();
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }
            if !self.advance_ascii_digits() {
                while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                    if self.peek() == '_' && !self.peek_next().is_ascii_digit() {
                        break;
                    }
                    self.advance();
                }
            }
        }

        let source_text = self
            .cursor
            .source_range(self.source, self.start, self.current);
        let raw_owned = source_text
            .is_none()
            .then(|| self.text_from_range(self.start, self.current));
        let raw_text = source_text.or(raw_owned.as_deref()).ok_or_else(|| {
            crate::tokens::errors::Error::internal("non-ASCII numbers own their text")
        })?;
        let normalized = (self.config.numbers_can_be_underscore_separated
            && raw_text.contains('_'))
        .then(|| raw_text.replace('_', ""));
        let text = normalized.as_deref().unwrap_or(raw_text);

        if !self.config.numeric_literals.is_empty() && !self.is_at_end() {
            let next_char: String = self.peek().to_ascii_uppercase().to_string();
            let suffix_match = self.numeric_literal_suffix(next_char);

            if let Some((suffix, len)) = suffix_match {
                for _ in 0..len {
                    self.advance();
                }
                let type_name = self
                    .config
                    .numeric_literals
                    .get(&suffix)
                    .ok_or_else(|| {
                        crate::tokens::errors::Error::internal(
                            "suffix verified by contains_key above",
                        )
                    })?
                    .clone();
                let combined = format!("{}::{}", text, type_name);
                self.add_token_with_text(TokenType::Number, combined);
                return Ok(());
            }
        }

        if self.config.identifiers_can_start_with_digit && !self.is_at_end() {
            let next = self.peek();
            if next.is_alphabetic() || next == '_' {
                if !self.advance_ascii_identifier() {
                    while !self.is_at_end() {
                        let ch = self.peek();
                        if ch.is_alphanumeric() || ch == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.add_token(TokenType::Identifier);
                return Ok(());
            }
        }

        if let Some(text) = normalized.or(raw_owned) {
            self.add_token_with_text(TokenType::Number, text);
        } else {
            self.add_token(TokenType::Number);
        }
        Ok(())
    }

    fn numeric_literal_suffix(&mut self, next_char: String) -> Option<(String, usize)> {
        if self.current + 1 < self.size {
            let two_char: String = [
                self.char_at(self.current).to_ascii_uppercase(),
                self.char_at(self.current + 1).to_ascii_uppercase(),
            ]
            .iter()
            .collect();
            if self.config.numeric_literals.contains_key(&two_char) {
                let after_suffix = if self.current + 2 < self.size {
                    self.char_at(self.current + 2)
                } else {
                    ' '
                };
                if !after_suffix.is_alphanumeric() && after_suffix != '_' {
                    Some((two_char, 2))
                } else {
                    None
                }
            } else if self.config.numeric_literals.contains_key(&next_char) {
                let after_suffix = if self.current + 1 < self.size {
                    self.char_at(self.current + 1)
                } else {
                    ' '
                };
                if !after_suffix.is_alphanumeric() && after_suffix != '_' {
                    Some((next_char, 1))
                } else {
                    None
                }
            } else {
                None
            }
        } else if self.config.numeric_literals.contains_key(&next_char) {
            Some((next_char, 1))
        } else {
            None
        }
    }

    fn try_scan_hex_number_string(&mut self) -> Result<bool> {
        let next = if self.current + 1 < self.size {
            self.char_at(self.current + 1)
        } else {
            '\0'
        };
        if next == 'x' || next == 'X' {
            self.advance();
            self.advance();
            let hex_start = self.current;
            if !self.advance_ascii_hex_digits() {
                while !self.is_at_end() && (self.peek().is_ascii_hexdigit() || self.peek() == '_') {
                    if self.peek() == '_' && !self.peek_next().is_ascii_hexdigit() {
                        break;
                    }
                    self.advance();
                }
            }
            let has_hex_digits = self.current > hex_start;
            let next_is_identifier_part = {
                let next = self.peek();
                next.is_alphanumeric() || matches!(next, '_' | '$' | '#' | '@')
            };
            let is_empty_hex_string = self.config.allow_empty_hex_string
                && !self.config.hex_string_is_integer_type
                && !next_is_identifier_part;
            if has_hex_digits || is_empty_hex_string {
                let mut is_hex_float = false;
                if has_hex_digits && !self.is_at_end() && self.peek() == '.' {
                    let after_dot = if self.current + 1 < self.size {
                        self.char_at(self.current + 1)
                    } else {
                        '\0'
                    };
                    if after_dot.is_ascii_hexdigit() {
                        is_hex_float = true;
                        self.advance();
                        if !self.advance_ascii_hex_digits() {
                            while !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                                self.advance();
                            }
                        }
                    }
                }
                if has_hex_digits && !self.is_at_end() && (self.peek() == 'p' || self.peek() == 'P')
                {
                    is_hex_float = true;
                    self.advance();
                    if !self.is_at_end() && (self.peek() == '+' || self.peek() == '-') {
                        self.advance();
                    }
                    if !self.advance_ascii_digits() {
                        while !self.is_at_end() && self.peek().is_ascii_digit() {
                            self.advance();
                        }
                    }
                }
                if is_hex_float {
                    let raw_text = self.text_from_range(self.start, self.current);
                    let full_text = if self.config.numbers_can_be_underscore_separated
                        && raw_text.contains('_')
                    {
                        raw_text.replace('_', "")
                    } else {
                        raw_text
                    };
                    self.add_token_with_text(TokenType::Number, full_text);
                } else if self.config.hex_string_is_integer_type {
                    let raw_value = self.text_from_range(hex_start, self.current);
                    let hex_value = if self.config.numbers_can_be_underscore_separated
                        && raw_value.contains('_')
                    {
                        raw_value.replace('_', "")
                    } else {
                        raw_value
                    };
                    self.add_token_with_text(TokenType::HexNumber, hex_value);
                } else {
                    let raw_value = self.text_from_range(hex_start, self.current);
                    let hex_value = if self.config.numbers_can_be_underscore_separated
                        && raw_value.contains('_')
                    {
                        raw_value.replace('_', "")
                    } else {
                        raw_value
                    };
                    self.add_token_with_text(TokenType::HexString, hex_value);
                }
                return Ok(true);
            }
            self.current = self.start + 1;
        }

        Ok(false)
    }

    /// Scan a number that starts with a dot (e.g., .25, .5, .123e10)
    pub(crate) fn scan_number_starting_with_dot(&mut self) -> Result<()> {
        self.advance();

        if !self.advance_ascii_digits() {
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                if self.peek() == '_' && !self.peek_next().is_ascii_digit() {
                    break;
                }
                self.advance();
            }
        }

        if self.starts_numeric_exponent() {
            self.advance();
            if self.peek() == '+' || self.peek() == '-' {
                self.advance();
            }
            if !self.advance_ascii_digits() {
                while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                    if self.peek() == '_' && !self.peek_next().is_ascii_digit() {
                        break;
                    }
                    self.advance();
                }
            }
        }

        let source_text = self
            .cursor
            .source_range(self.source, self.start, self.current);
        let raw_owned = source_text
            .is_none()
            .then(|| self.text_from_range(self.start, self.current));
        let raw_text = source_text.or(raw_owned.as_deref()).ok_or_else(|| {
            crate::tokens::errors::Error::internal("non-ASCII numbers own their text")
        })?;
        let normalized = (self.config.numbers_can_be_underscore_separated
            && raw_text.contains('_'))
        .then(|| raw_text.replace('_', ""));
        if let Some(text) = normalized.or(raw_owned) {
            self.add_token_with_text(TokenType::Number, text);
        } else {
            self.add_token(TokenType::Number);
        }
        Ok(())
    }

    /// Looks up a keyword, upper-casing ASCII on the stack; long texts are `Var`.
    #[inline]
    pub(crate) fn lookup_keyword_ascii(
        keywords: &HashMap<String, TokenType>,
        text: &str,
    ) -> TokenType {
        if text.len() > MAX_KEYWORD_LEN {
            return TokenType::Var;
        }
        let mut buf = [0u8; 128];
        for (i, b) in text.bytes().enumerate() {
            buf[i] = b.to_ascii_uppercase();
        }
        if let Ok(upper) = std::str::from_utf8(&buf[..text.len()]) {
            keywords.get(upper).copied().unwrap_or(TokenType::Var)
        } else {
            TokenType::Var
        }
    }

    pub(crate) fn scan_identifier_or_keyword(&mut self) -> Result<()> {
        let first_char = self.peek();
        if !first_char.is_alphanumeric() && first_char != '_' {
            let c = self.advance();
            return Err(Error::tokenize(
                format!("Unexpected character: '{}'", c),
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        if !self.advance_ascii_identifier() {
            while !self.is_at_end() {
                let c = self.peek();
                if c == '#' {
                    let next_c = if self.current + 1 < self.size {
                        self.char_at(self.current + 1)
                    } else {
                        '\0'
                    };
                    if next_c == '>' || next_c == '-' {
                        break;
                    }
                    self.advance();
                } else if c.is_alphanumeric() || c == '_' || c == '$' || c == '@' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let source_text = self
            .cursor
            .source_range(self.source, self.start, self.current);
        let owned_text = source_text
            .is_none()
            .then(|| self.text_from_range(self.start, self.current));
        let text = source_text.or(owned_text.as_deref()).ok_or_else(|| {
            crate::tokens::errors::Error::internal("non-ASCII identifiers own their text")
        })?;

        if text.eq_ignore_ascii_case("NOT") && self.peek() == '=' {
            self.advance();
            self.add_token(TokenType::Neq);
            return Ok(());
        }

        let next_char = self.peek();
        let is_single_quote = next_char == '\'';
        let is_double_quote = next_char == '"' && self.config.quotes.contains_key("\"");
        let is_double_quote_for_raw = next_char == '"';

        if text.eq_ignore_ascii_case("R") && (is_single_quote || is_double_quote_for_raw) {
            let quote_char = if is_single_quote { '\'' } else { '"' };
            self.advance();

            if self.peek() == quote_char && self.peek_next() == quote_char {
                self.advance();
                self.advance();
                let string_value = self.scan_raw_triple_quoted_content(quote_char)?;
                self.add_token_with_text(TokenType::RawString, string_value);
            } else {
                let string_value = self.scan_raw_string_content(quote_char)?;
                self.add_token_with_text(TokenType::RawString, string_value);
            }
            return Ok(());
        }

        if (is_single_quote || is_double_quote)
            && self.try_scan_prefixed_string(is_double_quote, is_single_quote, text)?
        {
            return Ok(());
        }

        if text.eq_ignore_ascii_case("U")
            && self.peek() == '&'
            && self.current + 1 < self.size
            && self.char_at(self.current + 1) == '\''
        {
            self.advance();
            self.advance();
            let string_value = self.scan_string_content()?;
            self.add_token_with_text(TokenType::UnicodeString, string_value);
            return Ok(());
        }

        let token_type = Self::lookup_keyword_ascii(&self.config.keywords, text);

        if let Some(text) = owned_text {
            self.add_token_with_text(token_type, text);
        } else {
            self.add_token_from_source(token_type, self.start, self.current);
        }
        Ok(())
    }

    fn try_scan_prefixed_string(
        &mut self,
        is_double_quote: bool,
        is_single_quote: bool,
        text: &str,
    ) -> Result<bool> {
        if text.eq_ignore_ascii_case("N") {
            self.advance();
            let string_value = if is_single_quote {
                self.scan_string_content()?
            } else {
                self.scan_double_quoted_string_content()?
            };
            self.add_token_with_text(TokenType::NationalString, string_value);
            return Ok(true);
        } else if text.eq_ignore_ascii_case("E") {
            let lowercase = text == LOWERCASE_ESCAPE_STRING_PREFIX;
            let prefix = if lowercase { "e:" } else { "E:" };
            self.advance();
            let string_value = self.scan_string_content_with_escapes(true)?;
            self.add_token_with_text(
                TokenType::EscapeString,
                format!("{}{}", prefix, string_value),
            );
            return Ok(true);
        } else if text.eq_ignore_ascii_case("X") {
            self.advance();
            let string_value = if is_single_quote {
                self.scan_string_content()?
            } else {
                self.scan_double_quoted_string_content()?
            };
            self.add_token_with_text(TokenType::HexString, string_value);
            return Ok(true);
        } else if text.eq_ignore_ascii_case("B") && is_double_quote {
            self.advance();
            let string_value = self.scan_double_quoted_string_content()?;
            self.add_token_with_text(TokenType::ByteString, string_value);
            return Ok(true);
        } else if text.eq_ignore_ascii_case("B") && is_single_quote {
            self.advance();
            let string_value = self.scan_string_content()?;
            if self.config.b_prefix_is_byte_string {
                self.add_token_with_text(TokenType::ByteString, string_value);
            } else {
                self.add_token_with_text(TokenType::BitString, string_value);
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Scans string content; `force_backslash_escapes` serves PostgreSQL E'...' strings.
    pub(crate) fn scan_string_content_with_escapes(
        &mut self,
        force_backslash_escapes: bool,
    ) -> Result<String> {
        let use_backslash_escapes =
            force_backslash_escapes || self.config.string_escapes.contains(&'\\');
        if let Some((start, end)) = self.try_scan_simple_quoted_content('\'', use_backslash_escapes)
        {
            return Ok(self.text_from_range(start, end));
        }
        let mut value = String::new();

        while !self.is_at_end() {
            let c = self.peek();
            if c == '\'' {
                if self.peek_next() == '\'' {
                    value.push('\'');
                    self.advance();
                    self.advance();
                } else {
                    break;
                }
            } else if c == '\\' && use_backslash_escapes {
                value.push(self.advance());
                if !self.is_at_end() {
                    value.push(self.advance());
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated string",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        self.advance();
        Ok(value)
    }

    /// Scan string content (everything between quotes)
    pub(crate) fn scan_string_content(&mut self) -> Result<String> {
        self.scan_string_content_with_escapes(false)
    }

    /// Scans double-quoted string content, for prefixed strings like b"..." or N"...".
    pub(crate) fn scan_double_quoted_string_content(&mut self) -> Result<String> {
        let use_backslash_escapes = self.config.string_escapes.contains(&'\\');
        if let Some((start, end)) = self.try_scan_simple_quoted_content('"', use_backslash_escapes)
        {
            return Ok(self.text_from_range(start, end));
        }
        let mut value = String::new();

        while !self.is_at_end() {
            let c = self.peek();
            if c == '"' {
                if self.peek_next() == '"' {
                    value.push('"');
                    self.advance();
                    self.advance();
                } else {
                    break;
                }
            } else if c == '\\' && use_backslash_escapes {
                self.advance();
                if !self.is_at_end() {
                    let escaped = self.advance();
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '0' => value.push('\0'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        '\'' => value.push('\''),
                        'x' => {
                            let mut hex = String::new();
                            for _ in 0..2 {
                                if !self.is_at_end() && self.peek().is_ascii_hexdigit() {
                                    hex.push(self.advance());
                                }
                            }
                            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                value.push(byte as char);
                            } else {
                                value.push('\\');
                                value.push('x');
                                value.push_str(&hex);
                            }
                        }
                        _ => {
                            value.push('\\');
                            value.push(escaped);
                        }
                    }
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated double-quoted string",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        self.advance();
        Ok(value)
    }

    /// Scans BigQuery raw string content, where only escaped quotes are processed.
    pub(crate) fn scan_raw_string_content(&mut self, quote_char: char) -> Result<String> {
        if let Some((start, end)) = self.try_scan_simple_quoted_content(
            quote_char,
            self.config.string_escapes_allowed_in_raw_strings,
        ) {
            return Ok(self.text_from_range(start, end));
        }
        let mut value = String::new();

        while !self.is_at_end() {
            let c = self.peek();
            if c == quote_char {
                if self.peek_next() == quote_char {
                    value.push(quote_char);
                    self.advance();
                    self.advance();
                } else {
                    break;
                }
            } else if c == '\\'
                && self.peek_next() == quote_char
                && self.config.string_escapes_allowed_in_raw_strings
            {
                value.push('\\');
                value.push(quote_char);
                self.advance();
                self.advance();
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated raw string",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        self.advance();
        Ok(value)
    }

    /// Scans raw triple-quoted string content up to three closing quotes.
    pub(crate) fn scan_raw_triple_quoted_content(&mut self, quote_char: char) -> Result<String> {
        let mut value = String::new();

        while !self.is_at_end() {
            if self.peek() == quote_char {
                let mut quote_count = 0;
                while self.current + quote_count < self.size
                    && self.char_at(self.current + quote_count) == quote_char
                {
                    quote_count += 1;
                }
                if quote_count >= TRIPLE_QUOTE_LEN {
                    for _ in 0..quote_count - TRIPLE_QUOTE_LEN {
                        value.push(quote_char);
                    }
                    for _ in 0..quote_count {
                        self.advance();
                    }
                    return Ok(value);
                }
            }
            let ch = self.advance();
            value.push(ch);
        }

        Err(Error::tokenize(
            "Unterminated raw triple-quoted string",
            Span {
                start: self.start,
                end: self.current,
                line: self.line,
                column: self.column,
            },
        ))
    }

    /// Scans an identifier starting with `$` (ClickHouse).
    pub(crate) fn scan_dollar_identifier(&mut self) -> Result<()> {
        self.advance();

        while !self.is_at_end() {
            let c = self.peek();
            if c.is_alphanumeric() || c == '_' || c == '$' {
                self.advance();
            } else {
                break;
            }
        }

        self.add_token(TokenType::Var);
        Ok(())
    }

    pub(crate) fn scan_tsql_identifier(&mut self) -> Result<()> {
        let first = self.advance();

        if first == '#' && self.peek() == '#' {
            self.advance();
        }

        if !self.advance_ascii_identifier() {
            while !self.is_at_end() {
                let c = self.peek();
                if c.is_alphanumeric() || c == '_' || c == '$' || c == '#' || c == '@' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.add_token(TokenType::Var);
        Ok(())
    }

    /// Consumes raw data after INSERT ... FORMAT name up to a blank line or the end.
    pub(crate) fn try_scan_insert_format_raw_data(&mut self) -> Option<String> {
        let len = self.tokens.len();
        if len < INSERT_FORMAT_MIN_TOKENS {
            return None;
        }

        let last = &self.tokens[len - 1];
        if last.text(self.source).eq_ignore_ascii_case("VALUES") {
            return None;
        }
        if !matches!(last.token_type(), TokenType::Var | TokenType::Identifier) {
            return None;
        }

        let format_tok = &self.tokens[len - 2];
        if !format_tok.text(self.source).eq_ignore_ascii_case("FORMAT") {
            return None;
        }

        let has_insert = self.tokens[..len - 2]
            .iter()
            .rev()
            .take(20)
            .any(|t| t.token_type() == TokenType::Insert);
        if !has_insert {
            return None;
        }

        let raw_start = self.current;
        while !self.is_at_end() {
            let c = self.peek();
            if c == '\n' {
                let saved = self.current;
                self.advance();
                while !self.is_at_end() && self.peek() == '\r' {
                    self.advance();
                }
                if self.is_at_end() || self.peek() == '\n' {
                    let raw = self.text_from_range(raw_start, saved);
                    return Some(raw.trim().to_string());
                }
            } else {
                self.advance();
            }
        }

        let raw = self.text_from_range(raw_start, self.current);
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    pub(crate) fn add_token(&mut self, token_type: TokenType) {
        self.add_token_from_source(token_type, self.start, self.current);
    }

    pub(crate) fn add_token_from_source(
        &mut self,
        token_type: TokenType,
        text_start: usize,
        text_end: usize,
    ) {
        let span = Span::new(self.start, self.current, self.line, self.column);
        if let Some(stats) = &mut self.guard_stats {
            stats.observe(token_type, span);
        }
        let mut token = if self
            .cursor
            .source_range(self.source, text_start, text_end)
            .is_some()
        {
            T::from_source(
                token_type,
                TokenSource {
                    text: self.source,
                    shared: self.shared_source.as_ref(),
                },
                text_start..text_end,
                span,
            )
        } else {
            T::from_owned(
                token_type,
                self.cursor
                    .text_from_range(self.source, text_start, text_end),
                span,
            )
        };
        token.comments_mut().append(&mut self.comments);
        self.tokens.push(token);
    }

    pub(crate) fn add_token_with_text(&mut self, token_type: TokenType, text: String) {
        let span = Span::new(self.start, self.current, self.line, self.column);
        if let Some(stats) = &mut self.guard_stats {
            stats.observe(token_type, span);
        }
        let mut token = T::from_owned(token_type, text, span);
        token.comments_mut().append(&mut self.comments);
        self.tokens.push(token);
    }
}
