//! Parser ddl: Snowflake CREATE DYNAMIC ICEBERG TABLE and CREATE INTERACTIVE TABLE, checked against their grammar and kept as written.

use crate::parser::constants::SNOWFLAKE_DYNAMIC_TABLE_REQUIRED;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE DYNAMIC ICEBERG TABLE or CREATE INTERACTIVE TABLE other than a clone, checked and kept as written; None for any other CREATE.
    pub(crate) fn parse_snowflake_create_query_table(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let partial_interactive = self.check_text_seq(&["INTERACTIVE"])
            && !self.check_text_seq(&["INTERACTIVE", "TABLE"])
            && !self.check_text_seq(&["INTERACTIVE", "WAREHOUSE"])
            && !self.check_text_seq(&["INTERACTIVE", "MATERIALIZED", "VIEW"]);
        if partial_interactive {
            return Err(self
                .parse_error("Expected TABLE, WAREHOUSE or MATERIALIZED VIEW after INTERACTIVE"));
        }
        let interactive = self.check_text_seq(&["INTERACTIVE", "TABLE"]);
        if !interactive && !self.check_text_seq(&["DYNAMIC", "ICEBERG"]) {
            return Ok(None);
        }
        let start = self.current;
        if interactive {
            self.match_text_seq(&["INTERACTIVE", "TABLE"]);
            if or_alter {
                return Err(self.parse_error("CREATE OR ALTER INTERACTIVE TABLE is not supported"));
            }
        } else {
            self.match_text_seq(&["DYNAMIC", "ICEBERG"]);
            if !self.match_token(TokenType::Table) {
                return Err(self.parse_error("Expected TABLE after DYNAMIC ICEBERG"));
            }
        }
        if self.check(TokenType::If) {
            if !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                return Err(self.parse_error("Expected IF NOT EXISTS"));
            }
            if or_replace {
                return Err(self.parse_error("OR REPLACE and IF NOT EXISTS cannot be combined"));
            }
        }
        self.check_iceberg_object_name()?;
        if self.check_identifier("CLONE") {
            self.current = start;
            return Ok(None);
        }
        if self.check(TokenType::LParen) {
            self.check_iceberg_columns()?;
        }
        if interactive {
            if !self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
                return Err(self.parse_error("Expected CLUSTER BY"));
            }
            self.expect_token(TokenType::LParen)?;
            self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
        }
        self.check_iceberg_properties(interactive)?;
        if !interactive && self.match_text_seq(&["REFRESH", "USING"]) {
            self.expect_token(TokenType::LParen)?;
            if !self.check(TokenType::Insert) {
                return Err(self.parse_error("Expected INSERT in REFRESH USING ( )"));
            }
            self.parse_statement()?;
            self.expect_token(TokenType::RParen)?;
        } else {
            self.expect_token(TokenType::As)?;
            if !self.check(TokenType::Select) && !self.check(TokenType::With) {
                return Err(self.parse_error("Expected a query after AS"));
            }
            self.parse_statement()?;
        }
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        let mut sql = String::from("CREATE");
        if or_replace {
            sql.push_str(" OR REPLACE");
        }
        if or_alter {
            sql.push_str(" OR ALTER");
        }
        sql.push(' ');
        sql.push_str(&self.tokens_to_sql(start, self.current));
        Ok(Some(Expression::Raw(Raw { sql })))
    }

    /// Checks an object name: a quoted identifier, or a word Snowflake does not reserve, optionally qualified.
    pub(super) fn check_iceberg_object_name(&mut self) -> Result<()> {
        if !self.check(TokenType::QuotedIdentifier) {
            let saved = self.current;
            self.expect_external_name()?;
            self.current = saved;
        }
        self.parse_object_name()?;
        Ok(())
    }

    /// Checks `( <name> [<type>] [<column option> ...] [ , ... ] )`.
    fn check_iceberg_columns(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            self.check_iceberg_object_name()?;
            if !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                if self.check(TokenType::String) {
                    return Err(self.parse_error("Expected a data type"));
                }
                self.parse_data_type()?;
            }
            while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                self.check_iceberg_column_option()?;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks `COMMENT '<text>'`, `[WITH] MASKING POLICY <name> [USING ( <names> )]` or `[WITH] TAG ( ... )`.
    fn check_iceberg_column_option(&mut self) -> Result<()> {
        if self.match_token(TokenType::Comment) {
            return self.expect_token(TokenType::String).map(|_| ());
        }
        self.match_token(TokenType::With);
        if self.match_text_seq(&["MASKING", "POLICY"]) {
            self.check_iceberg_object_name()?;
            if self.match_text_seq(&["USING"]) {
                self.check_iceberg_name_list(false)?;
            }
            return Ok(());
        }
        if self.match_text_seq(&["TAG"]) {
            return self.check_iceberg_tags();
        }
        Err(self.parse_error("Expected a column option"))
    }

    /// Checks `( <name> [ , ... ] )`, which may be empty only when `empty` is set.
    pub(super) fn check_iceberg_name_list(&mut self, empty: bool) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        if empty && self.match_token(TokenType::RParen) {
            return Ok(());
        }
        loop {
            self.check_iceberg_object_name()?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks `( <tag> = <value> [ , ... ] )`.
    pub(super) fn check_iceberg_tags(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            self.check_iceberg_object_name()?;
            self.expect_token(TokenType::Eq)?;
            self.check_iceberg_value()?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks the properties before AS or REFRESH USING; a dynamic table needs TARGET_LAG and WAREHOUSE.
    fn check_iceberg_properties(&mut self, interactive: bool) -> Result<()> {
        let mut keys: Vec<String> = Vec::new();
        while !self.is_at_end()
            && !self.check(TokenType::Semicolon)
            && !self.check(TokenType::As)
            && !self.check_text_seq(&["REFRESH", "USING"])
        {
            let property = match interactive {
                true => self.check_interactive_property()?,
                false => self.check_iceberg_property()?,
            };
            if let Some(key) = property {
                keys.push(key);
            }
        }
        let missing = SNOWFLAKE_DYNAMIC_TABLE_REQUIRED
            .iter()
            .any(|required| !keys.contains(&required.to_string()));
        if !interactive && missing {
            return Err(self.parse_error("Expected TARGET_LAG and WAREHOUSE"));
        }
        Ok(())
    }

    /// Checks one property, returning the key of a `<key> = <value>` one.
    fn check_iceberg_property(&mut self) -> Result<Option<String>> {
        if self.check(TokenType::Copy) && !self.check_next(TokenType::Eq) {
            self.skip();
            if self.match_text_seq(&["GRANTS"]) || self.match_text_seq(&["TAGS"]) {
                return Ok(None);
            }
            return Err(self.parse_error("Expected GRANTS or TAGS after COPY"));
        }
        if self.match_text_seq(&["REQUIRE", "USER"]) {
            return Ok(None);
        }
        if self.match_text_seq(&["EXECUTE", "AS", "USER"]) {
            self.check_iceberg_object_name()?;
            if self.match_text_seq(&["USE", "SECONDARY", "ROLES"])
                && !self.match_token(TokenType::All)
            {
                self.check_iceberg_object_name()?;
            }
            return Ok(None);
        }
        if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            self.expect_token(TokenType::LParen)?;
            self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(None);
        }
        if self.match_text_seq(&["FROZEN", "WHERE"]) {
            self.expect_token(TokenType::LParen)?;
            self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(None);
        }
        let saved = self.current;
        self.match_token(TokenType::With);
        if self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            self.check_iceberg_object_name()?;
            self.expect_token(TokenType::On)?;
            self.check_iceberg_name_list(true)?;
            return Ok(None);
        }
        self.current = saved;
        self.check_interactive_property()
    }

    /// Checks one property an interactive table allows, returning the key of a `<key> = <value>` one.
    fn check_interactive_property(&mut self) -> Result<Option<String>> {
        self.match_token(TokenType::With);
        if self.match_text_seq(&["TAG"]) {
            self.check_iceberg_tags()?;
            return Ok(None);
        }
        if self.match_text_seq(&["STORAGE", "LIFECYCLE", "POLICY"]) {
            self.check_iceberg_object_name()?;
            self.expect_token(TokenType::On)?;
            self.check_iceberg_name_list(false)?;
            return Ok(None);
        }
        let key = self.peek().text.to_ascii_uppercase();
        self.expect_external_name()?;
        self.expect_token(TokenType::Eq)?;
        if self.match_token(TokenType::LParen) {
            while !self.check(TokenType::RParen) {
                self.expect_external_name()?;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        } else {
            self.check_iceberg_value()?;
        }
        Ok(Some(key))
    }

    /// Checks a value: a literal, a quoted identifier, or a word that is not the key of the next property.
    pub(super) fn check_iceberg_value(&mut self) -> Result<()> {
        let literal = matches!(
            self.peek().token_type,
            TokenType::String
                | TokenType::Number
                | TokenType::True
                | TokenType::False
                | TokenType::QuotedIdentifier
        );
        if literal && !self.is_at_end() {
            self.skip();
            return Ok(());
        }
        if self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a value"));
        }
        self.expect_external_name()
    }
}
