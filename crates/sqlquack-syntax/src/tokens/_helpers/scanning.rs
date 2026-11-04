//! Tokens: scanning.

use std::sync::Arc;

use crate::guard::models::TokenGuardStats;
use crate::tokens::errors::Error;
use crate::tokens::models::Span;
use crate::tokens::models::TokenType;
use crate::tokens::models::TokenizerConfig;
use crate::tokens::models::TokenizerState;
use crate::tokens::types::Result;
use crate::tokens::types::TokenOutput;
use crate::tokens::types::TokenizerCursor;

impl<'a, C: TokenizerCursor, T: TokenOutput> TokenizerState<'a, C, T> {
    pub(crate) fn new(sql: &'a str, config: &'a TokenizerConfig, cursor: C) -> Self {
        let size = cursor.len();
        Self {
            source: sql,
            shared_source: None,
            cursor,
            size,
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            column: 1,
            comments: Vec::new(),
            guard_stats: None,
            config,
        }
    }

    pub(crate) fn new_shared(
        sql: &'a str,
        source: Arc<str>,
        config: &'a TokenizerConfig,
        cursor: C,
    ) -> Self {
        let size = cursor.len();
        Self {
            source: sql,
            shared_source: Some(source),
            cursor,
            size,
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            column: 1,
            comments: Vec::new(),
            guard_stats: Some(TokenGuardStats::default()),
            config,
        }
    }

    pub(crate) fn tokenize(&mut self) -> Result<Vec<T>> {
        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

            self.start = self.current;
            self.scan_token()?;

            if self.config.insert_format_raw_data {
                if let Some(raw) = self.try_scan_insert_format_raw_data() {
                    if !raw.is_empty() {
                        self.start = self.current;
                        self.add_token_with_text(TokenType::Var, raw);
                    }
                }
            }
        }

        if !self.comments.is_empty() {
            if let Some(last) = self.tokens.last_mut() {
                last.trailing_comments_mut().append(&mut self.comments);
            }
        }

        Ok(std::mem::take(&mut self.tokens))
    }

    #[inline]
    pub(crate) fn is_at_end(&self) -> bool {
        self.current >= self.size
    }

    #[inline]
    pub(crate) fn text_from_range(&self, start: usize, end: usize) -> String {
        self.cursor.text_from_range(self.source, start, end)
    }

    #[inline]
    pub(crate) fn char_at(&self, index: usize) -> char {
        self.cursor.char_at(index)
    }

    #[inline]
    pub(crate) fn range_contains(&self, start: usize, needle: char) -> bool {
        self.cursor.range_contains(start, needle)
    }

    #[inline]
    pub(crate) fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.char_at(self.current)
        }
    }

    #[inline]
    pub(crate) fn peek_next(&self) -> char {
        if self.current + 1 >= self.size {
            '\0'
        } else {
            self.char_at(self.current + 1)
        }
    }

    #[inline]
    pub(crate) fn advance(&mut self) -> char {
        let c = self.peek();
        self.current += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }

    #[inline]
    pub(crate) fn advance_ascii_to(&mut self, end: usize) -> bool {
        let Some(text) = self.cursor.source_range(self.source, self.current, end) else {
            return false;
        };

        let newline_count = text
            .as_bytes()
            .iter()
            .filter(|&&byte| byte == b'\n')
            .count();
        match text.as_bytes().iter().rposition(|&byte| byte == b'\n') {
            None => self.column += end - self.current,
            Some(last_newline) => {
                self.line += newline_count;
                self.column = text.len() - last_newline;
            }
        }
        self.current = end;
        true
    }

    #[inline]
    pub(crate) fn advance_ascii_digits(&mut self) -> bool {
        let Some(rest) = self
            .cursor
            .source_range(self.source, self.current, self.size)
        else {
            return false;
        };
        let bytes = rest.as_bytes();
        let mut length = 0;
        while length < bytes.len() {
            match bytes[length] {
                b'0'..=b'9' => length += 1,
                b'_' if bytes.get(length + 1).is_some_and(u8::is_ascii_digit) => length += 1,
                _ => break,
            }
        }
        self.current += length;
        self.column += length;
        true
    }

    #[inline]
    pub(crate) fn advance_ascii_hex_digits(&mut self) -> bool {
        let Some(rest) = self
            .cursor
            .source_range(self.source, self.current, self.size)
        else {
            return false;
        };
        let bytes = rest.as_bytes();
        let mut length = 0;
        while length < bytes.len() {
            match bytes[length] {
                byte if byte.is_ascii_hexdigit() => length += 1,
                b'_' if bytes.get(length + 1).is_some_and(u8::is_ascii_hexdigit) => length += 1,
                _ => break,
            }
        }
        self.current += length;
        self.column += length;
        true
    }

    #[inline]
    pub(crate) fn advance_ascii_identifier(&mut self) -> bool {
        let Some(rest) = self
            .cursor
            .source_range(self.source, self.current, self.size)
        else {
            return false;
        };
        let bytes = rest.as_bytes();
        let mut length = 0;
        while length < bytes.len() {
            let byte = bytes[length];
            if byte == b'#' && matches!(bytes.get(length + 1), Some(b'>') | Some(b'-')) {
                break;
            }
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'#' | b'@') {
                length += 1;
            } else {
                break;
            }
        }
        self.current += length;
        self.column += length;
        true
    }

    pub(crate) fn try_scan_simple_quoted_content(
        &mut self,
        quote: char,
        backslash_is_escape: bool,
    ) -> Option<(usize, usize)> {
        let content_start = self.current;
        let rest = self
            .cursor
            .source_range(self.source, content_start, self.size)?;
        let quote_offset = rest.find(quote)?;
        let content_end = content_start + quote_offset;

        if (content_end + 1 < self.size && self.char_at(content_end + 1) == quote)
            || (backslash_is_escape && rest[..quote_offset].contains('\\'))
        {
            return None;
        }

        self.advance_ascii_to(content_end);
        self.advance();
        Some((content_start, content_end))
    }

    pub(crate) fn skip_whitespace(&mut self) {
        let mut saw_newline = false;
        while !self.is_at_end() {
            let c = self.peek();
            match c {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    saw_newline = true;
                    self.advance();
                }
                '\u{00A0}' | '\u{2000}'..='\u{200B}' | '\u{3000}' | '\u{FEFF}' => {
                    self.advance();
                }
                '-' if self.peek_next() == '-' => {
                    self.scan_line_comment(saw_newline);
                    saw_newline = true;
                }
                '/' if self.peek_next() == '/' && self.config.hash_comments => {
                    self.scan_double_slash_comment();
                }
                '/' if self.peek_next() == '*' => {
                    if self.current + 2 < self.size && self.char_at(self.current + 2) == '+' {
                        break;
                    }
                    if self.scan_block_comment(saw_newline).is_err() {
                        return;
                    }
                }
                '/' if self.peek_next() == '/' && self.config.comments.contains_key("//") => {
                    let prev_non_ws = if self.current > 0 {
                        let mut i = self.current - 1;
                        while i > 0 && (self.char_at(i) == ' ' || self.char_at(i) == '\t') {
                            i -= 1;
                        }
                        self.char_at(i)
                    } else {
                        '\0'
                    };
                    if prev_non_ws == ':' || prev_non_ws == '/' {
                        break;
                    }
                    self.scan_line_comment(saw_newline);
                    saw_newline = true;
                }
                '#' if self.config.hash_comments => {
                    self.scan_hash_line_comment();
                }
                _ => break,
            }
        }
    }

    pub(crate) fn scan_hash_line_comment(&mut self) {
        self.advance();
        let start = self.current;
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
        let comment = self.text_from_range(start, self.current);
        let comment_text = comment.trim().to_string();
        if let Some(last) = self.tokens.last_mut() {
            last.trailing_comments_mut().push(comment_text);
        } else {
            self.comments.push(comment_text);
        }
    }

    pub(crate) fn scan_double_slash_comment(&mut self) {
        self.advance();
        self.advance();
        let start = self.current;
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
        let comment = self.text_from_range(start, self.current);
        let comment_text = comment.trim().to_string();
        if let Some(last) = self.tokens.last_mut() {
            last.trailing_comments_mut().push(comment_text);
        } else {
            self.comments.push(comment_text);
        }
    }

    pub(crate) fn scan_line_comment(&mut self, after_newline: bool) {
        self.advance();
        self.advance();
        let start = self.current;
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
        let comment_text = self.text_from_range(start, self.current);

        if after_newline || self.tokens.is_empty() {
            self.comments.push(comment_text);
        } else if let Some(last) = self.tokens.last_mut() {
            last.trailing_comments_mut().push(comment_text);
        }
    }

    pub(crate) fn scan_block_comment(&mut self, after_newline: bool) -> Result<()> {
        self.advance();
        self.advance();
        let content_start = self.current;
        let mut depth = 1;

        while !self.is_at_end() && depth > 0 {
            if self.peek() == '/' && self.peek_next() == '*' && self.config.nested_comments {
                self.advance();
                self.advance();
                depth += 1;
            } else if self.peek() == '*' && self.peek_next() == '/' {
                depth -= 1;
                if depth > 0 {
                    self.advance();
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        if depth > 0 {
            return Err(Error::tokenize(
                "Unterminated block comment",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        let content = self.text_from_range(content_start, self.current);
        self.advance();
        self.advance();

        let comment_text = format!("/*{}*/", content);

        if after_newline || self.tokens.is_empty() {
            self.comments.push(comment_text);
        } else if let Some(last) = self.tokens.last_mut() {
            last.trailing_comments_mut().push(comment_text);
        }

        Ok(())
    }

    /// Scan a hint comment /*+ ... */ and return it as a Hint token
    pub(crate) fn scan_hint(&mut self) -> Result<()> {
        self.advance();
        self.advance();
        self.advance();
        let hint_start = self.current;

        while !self.is_at_end() {
            if self.peek() == '*' && self.peek_next() == '/' {
                break;
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated hint comment",
                Span {
                    start: self.start,
                    end: self.current,
                    line: self.line,
                    column: self.column,
                },
            ));
        }

        let hint_text = self.text_from_range(hint_start, self.current);
        self.advance();
        self.advance();

        self.add_token_with_text(TokenType::Hint, hint_text.trim().to_string());

        Ok(())
    }

    /// Scan a positional parameter: $1, $2, etc.
    pub(crate) fn scan_positional_parameter(&mut self) -> Result<()> {
        self.advance();
        let start = self.current;

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        let number = self.text_from_range(start, self.current);
        self.add_token_with_text(TokenType::Parameter, number);
        Ok(())
    }

    /// Scans `$tag$content$tag$`, stored as "tag\x00content", or `None`.
    pub(crate) fn try_scan_tagged_dollar_string(&mut self) -> Result<Option<()>> {
        let saved_pos = self.current;

        self.advance();

        let tag_start = self.current;
        while !self.is_at_end()
            && (self.peek().is_alphanumeric() || self.peek() == '_' || !self.peek().is_ascii())
        {
            self.advance();
        }
        let tag = self.text_from_range(tag_start, self.current);

        if self.is_at_end() || self.peek() != '$' {
            self.current = saved_pos;
            return Ok(None);
        }
        self.advance();

        let content_start = self.current;
        let closing_tag = format!("${}$", tag);
        let closing_chars: Vec<char> = closing_tag.chars().collect();

        loop {
            if self.is_at_end() {
                self.current = saved_pos;
                return Ok(None);
            }

            if self.peek() == '$' && self.current + closing_chars.len() <= self.size {
                let matches = closing_chars.iter().enumerate().all(|(j, &ch)| {
                    self.current + j < self.size && self.char_at(self.current + j) == ch
                });
                if matches {
                    let content = self.text_from_range(content_start, self.current);
                    for _ in 0..closing_chars.len() {
                        self.advance();
                    }
                    let token_text = format!("{}\x00{}", tag, content);
                    self.add_token_with_text(TokenType::DollarString, token_text);
                    return Ok(Some(()));
                }
            }
            self.advance();
        }
    }

    /// Scans a dollar-quoted string, `$$content$$` or tagged.
    pub(crate) fn scan_dollar_quoted_string(&mut self) -> Result<()> {
        self.advance();
        self.advance();

        let start = self.current;
        while !self.is_at_end() {
            if self.peek() == '$'
                && self.current + 1 < self.size
                && self.char_at(self.current + 1) == '$'
            {
                break;
            }
            self.advance();
        }

        let content = self.text_from_range(start, self.current);

        if self.is_at_end() {
            return Err(Error::tokenize(
                "Unterminated dollar-quoted string",
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

        self.add_token_with_text(TokenType::DollarString, content);
        Ok(())
    }

    pub(crate) fn scan_token(&mut self) -> Result<()> {
        let c = self.peek();

        if c == '\'' {
            if self.config.quotes.contains_key("'''")
                && self.peek_next() == '\''
                && self.current + 2 < self.size
                && self.char_at(self.current + 2) == '\''
            {
                return self.scan_triple_quoted_string('\'');
            }
            return self.scan_string();
        }

        if c == '"'
            && self.config.quotes.contains_key("\"\"\"")
            && self.peek_next() == '"'
            && self.current + 2 < self.size
            && self.char_at(self.current + 2) == '"'
        {
            return self.scan_triple_quoted_string('"');
        }

        if c == '"'
            && self.config.quotes.contains_key("\"")
            && !self.config.identifiers.contains_key(&'"')
        {
            return self.scan_double_quoted_string();
        }

        if let Some(&end_quote) = self.config.identifiers.get(&c) {
            return self.scan_quoted_identifier(end_quote);
        }

        if c.is_ascii_digit() {
            return self.scan_number();
        }

        if c == '.' && self.peek_next().is_ascii_digit() {
            let prev_char = if self.current > 0 {
                self.char_at(self.current - 1)
            } else {
                '\0'
            };
            let is_after_ident = prev_char.is_alphanumeric()
                || prev_char == '_'
                || prev_char == '`'
                || prev_char == '"'
                || prev_char == ']'
                || prev_char == ')';
            if prev_char != '.' && !is_after_ident {
                return self.scan_number_starting_with_dot();
            }
        }

        if c == '/'
            && self.peek_next() == '*'
            && self.current + 2 < self.size
            && self.char_at(self.current + 2) == '+'
        {
            return self.scan_hint();
        }

        if let Some(token_type) = self.try_scan_multi_char_operator() {
            self.add_token(token_type);
            return Ok(());
        }

        if c == '$'
            && (self.peek_next().is_alphanumeric()
                || self.peek_next() == '_'
                || !self.peek_next().is_ascii())
        {
            if let Some(()) = self.try_scan_tagged_dollar_string()? {
                return Ok(());
            }
            if self.config.dollar_sign_is_identifier {
                return self.scan_dollar_identifier();
            }
        }

        if c == '$' && self.peek_next() == '$' {
            return self.scan_dollar_quoted_string();
        }

        if c == '$' && self.peek_next().is_ascii_digit() {
            return self.scan_positional_parameter();
        }

        if c == '$' && self.config.dollar_sign_is_identifier {
            return self.scan_dollar_identifier();
        }

        if (c == '#' || c == '@')
            && (self.peek_next().is_alphanumeric()
                || self.peek_next() == '_'
                || self.peek_next() == '#')
        {
            return self.scan_tsql_identifier();
        }

        if let Some(&token_type) = self.config.single_tokens.get(&c) {
            self.advance();
            self.add_token(token_type);
            return Ok(());
        }

        if c == '\u{2212}' {
            self.advance();
            self.add_token(TokenType::Dash);
            return Ok(());
        }

        if c == '\u{2044}' {
            self.advance();
            self.add_token(TokenType::Slash);
            return Ok(());
        }

        if c == '\u{2018}' || c == '\u{2019}' {
            return self.scan_unicode_quoted_string();
        }
        if c == '\u{201C}' || c == '\u{201D}' {
            return self.scan_unicode_quoted_identifier();
        }

        self.scan_identifier_or_keyword()
    }

    pub(crate) fn try_scan_multi_char_operator(&mut self) -> Option<TokenType> {
        let c = self.peek();
        let next = self.peek_next();
        let third = if self.current + 2 < self.size {
            self.char_at(self.current + 2)
        } else {
            '\0'
        };

        if c == '-' && next == '|' && third == '-' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::Adjacent);
        }

        if c == '|' && next == '|' && third == '/' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::DPipeSlash);
        }

        if c == '#' && next == '>' && third == '>' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::DHashArrow);
        }

        if c == '-' && next == '>' && third == '>' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::DArrow);
        }

        if c == '<' && next == '=' && third == '>' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::NullsafeEq);
        }

        if c == '<' && next == '-' && third == '>' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::LrArrow);
        }

        if c == '<' && next == '@' {
            self.advance();
            self.advance();
            return Some(TokenType::LtAt);
        }

        if c == '@' && next == '>' {
            self.advance();
            self.advance();
            return Some(TokenType::AtGt);
        }

        if c == '~' && next == '~' && third == '~' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::Glob);
        }

        if c == '~' && next == '~' && third == '*' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::ILike);
        }

        self.try_scan_later_multi_char_operator(c, next, third)
    }

    fn try_scan_later_multi_char_operator(
        &mut self,
        c: char,
        next: char,
        third: char,
    ) -> Option<TokenType> {
        let fourth = if self.current + 3 < self.size {
            self.char_at(self.current + 3)
        } else {
            '\0'
        };
        if c == '!' && next == '~' && third == '~' && fourth == '*' {
            self.advance();
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::NotILike);
        }

        if c == '!' && next == '~' && third == '~' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::NotLike);
        }

        if c == '!' && next == '~' && third == '*' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::NotIRLike);
        }

        if c == '!' && next == ':' && third == '>' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::NColonGt);
        }

        if c == '?' && next == ':' && third == ':' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::QDColon);
        }

        if c == '!' && next == '~' {
            self.advance();
            self.advance();
            return Some(TokenType::NotRLike);
        }

        if c == '~' && next == '~' {
            self.advance();
            self.advance();
            return Some(TokenType::Like);
        }

        if c == '~' && next == '*' {
            self.advance();
            self.advance();
            return Some(TokenType::IRLike);
        }

        if c == ':' && next == ':' && third == '$' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::DColonDollar);
        }
        if c == ':' && next == ':' && third == '%' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::DColonPercent);
        }
        if c == ':' && next == ':' && third == '?' {
            self.advance();
            self.advance();
            self.advance();
            return Some(TokenType::DColonQMark);
        }

        let token_type = match (c, next) {
            ('.', ':') => Some(TokenType::DotColon),
            ('=', '=') => Some(TokenType::Eq),
            ('<', '=') => Some(TokenType::Lte),
            ('>', '=') => Some(TokenType::Gte),
            ('!', '=') => Some(TokenType::Neq),
            ('<', '>') => Some(TokenType::Neq),
            ('^', '=') => Some(TokenType::Neq),
            ('<', '<') => Some(TokenType::LtLt),
            ('>', '>') => Some(TokenType::GtGt),
            ('|', '|') => Some(TokenType::DPipe),
            ('|', '/') => Some(TokenType::PipeSlash),
            ('/', '/') if self.config.double_slash_int_div => Some(TokenType::Div),
            (':', ':') => Some(TokenType::DColon),
            (':', '=') => Some(TokenType::ColonEq),
            (':', '>') => Some(TokenType::ColonGt),
            ('-', '>') => Some(TokenType::Arrow),
            ('=', '>') => Some(TokenType::FArrow),
            ('&', '&') => Some(TokenType::DAmp),
            ('&', '<') => Some(TokenType::AmpLt),
            ('&', '>') => Some(TokenType::AmpGt),
            ('@', '@') => Some(TokenType::AtAt),
            ('@', '?') => Some(TokenType::AtQMark),
            ('?', '|') => Some(TokenType::QMarkPipe),
            ('?', '&') => Some(TokenType::QMarkAmp),
            ('?', '?') => Some(TokenType::DQMark),
            ('#', '>') => Some(TokenType::HashArrow),
            ('#', '-') => Some(TokenType::HashDash),
            ('^', '@') => Some(TokenType::CaretAt),
            ('*', '*') => Some(TokenType::DStar),
            ('|', '>') => Some(TokenType::PipeGt),
            _ => None,
        };

        if token_type.is_some() {
            self.advance();
            self.advance();
        }

        token_type
    }
}
