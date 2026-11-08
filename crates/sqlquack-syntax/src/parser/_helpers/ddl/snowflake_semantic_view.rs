//! Parser ddl: Snowflake CREATE SEMANTIC VIEW, checked before the statement is kept as written.

use crate::parser::constants::SNOWFLAKE_VERIFIED_QUERY_FIELDS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Refuses a Snowflake CREATE SEMANTIC VIEW that breaks its grammar, leaving the position unchanged.
    pub(crate) fn check_snowflake_semantic_view(&mut self) -> Result<()> {
        let start = self.current;
        let checked = self.check_semantic_view_body();
        self.current = start;
        self.config.in_semantic_view = false;
        checked
    }

    /// Skips EXCLUDING after a window's PARTITION BY while a semantic view is checked.
    pub(crate) fn skip_semantic_view_excluding(&mut self) {
        if self.config.in_semantic_view {
            self.match_text_seq(&["EXCLUDING"]);
        }
    }

    fn check_semantic_view_body(&mut self) -> Result<()> {
        self.config.in_semantic_view = true;
        self.match_text_seq(&["SEMANTIC", "VIEW"]);
        if self.check(TokenType::If)
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        self.parse_object_name()?;
        if !self.match_text_seq(&["TABLES"]) {
            return Err(self.parse_error("Expected TABLES after the semantic view name"));
        }
        self.semantic_list(Self::check_semantic_table)?;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            self.check_semantic_view_clause()?;
        }
        Ok(())
    }

    fn check_semantic_view_clause(&mut self) -> Result<()> {
        if self.match_text_seq(&["RELATIONSHIPS"]) {
            self.semantic_list(Self::check_semantic_relationship)
        } else if self.match_text_seq(&["FACTS"])
            || self.match_text_seq(&["DIMENSIONS"])
            || self.match_text_seq(&["METRICS"])
        {
            self.semantic_list(Self::check_semantic_item)
        } else if self.match_text_seq(&["AI_VERIFIED_QUERIES"]) {
            self.semantic_list(Self::check_semantic_verified_query)
        } else if self.match_text_seq(&["AI_SQL_GENERATION"])
            || self.match_text_seq(&["AI_QUESTION_CATEGORIZATION"])
        {
            self.expect_semantic_string()
        } else if self.match_text_seq(&["COPY", "GRANTS"]) {
            Ok(())
        } else if self.match_text_seq(&["COMMENT"]) {
            self.expect_token(TokenType::Eq)?;
            self.expect_semantic_string()
        } else if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            self.check_semantic_tags()
        } else {
            Err(self.parse_error("Unexpected clause in CREATE SEMANTIC VIEW"))
        }
    }

    /// Checks `( <item> [ , ... ] )`, which needs at least one item.
    fn semantic_list(&mut self, item: fn(&mut Self) -> Result<()>) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            item(self)?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    fn expect_semantic_string(&mut self) -> Result<()> {
        match self.match_token(TokenType::String) {
            true => Ok(()),
            false => Err(self.parse_error("Expected a string")),
        }
    }

    /// Checks one unqualified name: an identifier or a keyword Snowflake does not reserve.
    fn expect_semantic_name(&mut self) -> Result<()> {
        let token_type = self.peek().token_type;
        let name_like = matches!(
            token_type,
            TokenType::Identifier | TokenType::QuotedIdentifier | TokenType::Var
        ) || token_type.is_keyword();
        if self.is_at_end() || !name_like || self.is_snowflake_reserved_keyword() {
            return Err(self.parse_error("Expected a name"));
        }
        self.skip();
        Ok(())
    }

    fn check_semantic_alias(&mut self) -> Result<()> {
        if self
            .peek_nth(1)
            .is_some_and(|token| token.token_type == TokenType::As)
        {
            self.expect_semantic_name()?;
            self.skip();
        }
        Ok(())
    }

    fn check_semantic_table(&mut self) -> Result<()> {
        self.check_semantic_alias()?;
        self.parse_object_name()?;
        while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
            if self.match_text_seq(&["PRIMARY", "KEY"]) || self.match_text_seq(&["UNIQUE"]) {
                self.semantic_list(Self::expect_semantic_name)?;
            } else if self.match_text_seq(&["CONSTRAINT"]) {
                self.expect_semantic_name()?;
                if !self.match_text_seq(&["DISTINCT", "RANGE"]) {
                    return Err(self.parse_error("Expected DISTINCT RANGE"));
                }
                self.check_semantic_range()?;
            } else {
                self.check_semantic_property(false)?;
            }
        }
        Ok(())
    }

    fn check_semantic_range(&mut self) -> Result<()> {
        if !self.match_text_seq(&["BETWEEN"]) {
            return Err(self.parse_error("Expected BETWEEN"));
        }
        self.expect_semantic_name()?;
        if !self.match_text_seq(&["AND"]) {
            return Err(self.parse_error("Expected AND"));
        }
        self.expect_semantic_name()?;
        match self.match_text_seq(&["EXCLUSIVE"]) {
            true => Ok(()),
            false => Err(self.parse_error("Expected EXCLUSIVE")),
        }
    }

    /// Checks COMMENT, SYNONYMS or TAG, and with `item` also a Cortex Search service.
    fn check_semantic_property(&mut self, item: bool) -> Result<()> {
        if self.match_text_seq(&["COMMENT"]) {
            self.expect_token(TokenType::Eq)?;
            self.expect_semantic_string()
        } else if self.match_text_seq(&["WITH", "SYNONYMS"]) {
            self.match_token(TokenType::Eq);
            self.semantic_list(Self::expect_semantic_string)
        } else if item && self.match_text_seq(&["WITH", "CORTEX", "SEARCH", "SERVICE"]) {
            self.parse_object_name()?;
            match self.match_token(TokenType::Using) {
                true => self.expect_semantic_name(),
                false => Ok(()),
            }
        } else if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            self.check_semantic_tags()
        } else {
            Err(self.parse_error("Unexpected property in CREATE SEMANTIC VIEW"))
        }
    }

    fn check_semantic_tags(&mut self) -> Result<()> {
        self.semantic_list(|parser| {
            parser.parse_object_name()?;
            parser.expect_token(TokenType::Eq)?;
            match parser.check(TokenType::String) {
                true => parser.expect_semantic_string(),
                false => parser.expect_semantic_name(),
            }
        })
    }

    fn check_semantic_relationship(&mut self) -> Result<()> {
        self.check_semantic_alias()?;
        self.parse_object_name()?;
        self.semantic_list(Self::expect_semantic_name)?;
        if !self.match_text_seq(&["REFERENCES"]) {
            return Err(self.parse_error("Expected REFERENCES"));
        }
        self.parse_object_name()?;
        if !self.match_token(TokenType::LParen) {
            return Ok(());
        }
        if self.check_text_seq(&["BETWEEN"]) {
            self.check_semantic_range()?;
        } else {
            loop {
                self.match_text_seq(&["ASOF"]);
                self.expect_semantic_name()?;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks a fact, dimension or metric: `[PUBLIC | PRIVATE] <table>.<name> [...] AS <expr> [<property> ...]`.
    fn check_semantic_item(&mut self) -> Result<()> {
        if !self.check_next(TokenType::Dot) {
            let _ = self.match_text_seq(&["PUBLIC"]) || self.match_text_seq(&["PRIVATE"]);
        }
        self.expect_semantic_name()?;
        self.expect_token(TokenType::Dot)?;
        self.expect_semantic_name()?;
        loop {
            if self.match_text_seq(&["LABELS"]) {
                self.expect_token(TokenType::Eq)?;
                self.semantic_list(Self::expect_semantic_name)?;
            } else if self.match_token(TokenType::Using) {
                self.semantic_list(Self::expect_semantic_name)?;
            } else if self.match_text_seq(&["NON", "ADDITIVE", "BY"]) {
                self.semantic_list(Self::check_semantic_ordered_name)?;
            } else {
                break;
            }
        }
        self.expect_token(TokenType::As)?;
        self.parse_expression()?;
        while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
            self.check_semantic_property(true)?;
        }
        Ok(())
    }

    fn check_semantic_ordered_name(&mut self) -> Result<()> {
        self.expect_semantic_name()?;
        let _ = self.match_token(TokenType::Asc) || self.match_token(TokenType::Desc);
        if self.match_text_seq(&["NULLS"])
            && !(self.match_text_seq(&["FIRST"]) || self.match_text_seq(&["LAST"]))
        {
            return Err(self.parse_error("Expected FIRST or LAST after NULLS"));
        }
        Ok(())
    }

    /// Checks `<name> AS ( <key> <value> ... )`, which needs QUESTION and SQL.
    fn check_semantic_verified_query(&mut self) -> Result<()> {
        self.expect_semantic_name()?;
        self.expect_token(TokenType::As)?;
        self.expect_token(TokenType::LParen)?;
        let mut keys: Vec<String> = Vec::new();
        while !self.check(TokenType::RParen) {
            if !matches!(
                self.peek().token_type,
                TokenType::Identifier | TokenType::Var
            ) {
                return Err(self.parse_error("Expected a verified query field"));
            }
            keys.push(self.advance_text()?.to_ascii_uppercase());
            let literal = matches!(
                self.peek().token_type,
                TokenType::String | TokenType::Number | TokenType::True | TokenType::False
            );
            match literal {
                true => self.skip(),
                false => self.expect_semantic_name()?,
            }
        }
        if !SNOWFLAKE_VERIFIED_QUERY_FIELDS
            .iter()
            .all(|field| keys.contains(&field.to_string()))
        {
            return Err(self.parse_error("Expected QUESTION and SQL in a verified query"));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }
}
