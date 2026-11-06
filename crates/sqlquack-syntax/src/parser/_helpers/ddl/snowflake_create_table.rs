//! Parser ddl: a Snowflake CREATE TABLE the table model cannot hold, such as one with column or table policies, checked against its grammar.

use crate::expressions::statements::models::Raw;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::SNOWFLAKE_TABLE_PROPERTY_KEYS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE TABLE (CREATE and TABLE at the given positions) the table parser could not finish, kept as written when its grammar holds, so translation refuses its policies.
    pub(crate) fn keep_snowflake_create_table(
        &mut self,
        parsed: Result<Expression>,
        (create_start, table_start): (usize, usize),
        or_replace: bool,
        checkable: bool,
    ) -> Result<Expression> {
        let parsed = match parsed {
            Ok(table) if self.is_at_end() || self.check(TokenType::Semicolon) => return Ok(table),
            _ if !self.is_snowflake() || !checkable => return parsed,
            parsed => parsed,
        };
        let end = self.current;
        self.current = table_start;
        if self.check_snowflake_create_table(or_replace).is_err() {
            self.current = end;
            return parsed;
        }
        let sql = self.tokens_to_sql(create_start, self.current);
        Ok(Expression::Raw(Raw { sql }))
    }

    /// Checks a Snowflake CREATE TABLE from its TABLE word to the end of the statement.
    fn check_snowflake_create_table(&mut self, or_replace: bool) -> Result<()> {
        self.expect_token(TokenType::Table)?;
        if self.check(TokenType::If) {
            if !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                return Err(self.parse_error("Expected IF NOT EXISTS"));
            }
            if or_replace {
                return Err(self.parse_error("OR REPLACE and IF NOT EXISTS cannot be combined"));
            }
        }
        self.check_iceberg_object_name()?;
        let like = self.match_token(TokenType::Like);
        let template = !like && self.match_text_seq(&["USING", "TEMPLATE"]);
        let columns = !like && !template && self.check(TokenType::LParen);
        let mut untyped = false;
        if like {
            self.check_iceberg_object_name()?;
        } else if template {
            self.check_external_template()?;
        } else if columns {
            untyped = self.check_table_elements()?;
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) && !self.check(TokenType::As) {
            self.check_table_property()?;
        }
        let query = !like && !template && self.match_token(TokenType::As);
        if query {
            if !self.check(TokenType::Select) && !self.check(TokenType::With) {
                return Err(self.parse_error("Expected a query after AS"));
            }
            self.parse_statement()?;
        }
        if !like && !template && !columns && !query {
            return Err(self.parse_error("Expected columns, LIKE or AS"));
        }
        if untyped && !query {
            return Err(self.parse_error("Expected a data type"));
        }
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        Ok(())
    }

    /// Checks `( <column or constraint> [ , ... ] )`, returning whether a column has no data type.
    fn check_table_elements(&mut self) -> Result<bool> {
        self.expect_token(TokenType::LParen)?;
        let mut untyped = false;
        loop {
            let constraint = self.check(TokenType::Constraint)
                || self.check(TokenType::Unique)
                || self.check(TokenType::PrimaryKey)
                || self.check(TokenType::ForeignKey)
                || self.check(TokenType::Check);
            if constraint {
                self.check_table_constraint(false)?;
            } else {
                untyped |= self.check_table_column()?;
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(untyped)
    }

    /// Checks `<name> [<type>] [<column option> ...]`, returning whether the type is missing.
    pub(super) fn check_table_column(&mut self) -> Result<bool> {
        if !self.check(TokenType::QuotedIdentifier) {
            self.expect_external_name()?;
        } else {
            self.skip();
        }
        let untyped = self.check(TokenType::Comma)
            || self.check(TokenType::RParen)
            || self.check(TokenType::With)
            || self.check(TokenType::Comment)
            || self.check_text_seq(&["MASKING", "POLICY"])
            || self.check_text_seq(&["PROJECTION", "POLICY"])
            || self.check_text_seq(&["TAG"]);
        if !untyped {
            if self.check(TokenType::String) {
                return Err(self.parse_error("Expected a data type"));
            }
            self.parse_data_type()?;
        }
        while !self.check(TokenType::Comma)
            && !self.check(TokenType::RParen)
            && !self.check(TokenType::Semicolon)
            && !self.is_at_end()
        {
            self.check_column_option()?;
        }
        Ok(untyped)
    }

    /// Checks one column option, including an inline constraint.
    fn check_column_option(&mut self) -> Result<()> {
        if self.match_keywords(&[TokenType::Not, TokenType::Null])
            || self.match_token(TokenType::Null)
        {
            return Ok(());
        }
        let constraint = self.check(TokenType::Constraint)
            || self.check(TokenType::Unique)
            || self.check(TokenType::PrimaryKey)
            || self.check(TokenType::ForeignKey)
            || self.check(TokenType::References)
            || self.check(TokenType::Check);
        if constraint {
            return self.check_table_constraint(true);
        }
        if self.match_token(TokenType::Collate) || self.match_token(TokenType::Comment) {
            return self.expect_token(TokenType::String).map(|_| ());
        }
        if self.match_token(TokenType::Default) {
            return self.parse_unary().map(|_| ());
        }
        if self.match_token(TokenType::AutoIncrement) || self.match_text_seq(&["IDENTITY"]) {
            return self.check_identity_options();
        }
        if self.match_token(TokenType::As) {
            return self.parse_expression().map(|_| ());
        }
        self.match_token(TokenType::With);
        if self.match_text_seq(&["MASKING", "POLICY"]) {
            self.check_iceberg_object_name()?;
            if self.match_text_seq(&["USING"]) {
                self.check_iceberg_name_list(false)?;
            }
            return Ok(());
        }
        if self.match_text_seq(&["PROJECTION", "POLICY"]) {
            return self.check_iceberg_object_name();
        }
        if self.match_text_seq(&["TAG"]) {
            return self.check_iceberg_tags();
        }
        Err(self.parse_error("Expected a column option"))
    }

    /// Checks `[ ( <start> , <step> ) | START <start> INCREMENT <step> ] [ ORDER | NOORDER ]`.
    fn check_identity_options(&mut self) -> Result<()> {
        if self.match_token(TokenType::LParen) {
            self.check_identity_number()?;
            self.expect_token(TokenType::Comma)?;
            self.check_identity_number()?;
            self.expect_token(TokenType::RParen)?;
        } else if self.match_text_seq(&["START"]) {
            self.check_identity_number()?;
            if !self.match_text_seq(&["INCREMENT"]) {
                return Err(self.parse_error("Expected INCREMENT"));
            }
            self.check_identity_number()?;
        }
        let _ = self.match_token(TokenType::Order) || self.match_text_seq(&["NOORDER"]);
        Ok(())
    }

    /// Checks a number, possibly negative.
    fn check_identity_number(&mut self) -> Result<()> {
        self.match_token(TokenType::Dash);
        self.expect_token(TokenType::Number).map(|_| ())
    }

    /// Checks `[CONSTRAINT <name>]` and a constraint with its properties; `inline` takes no column list.
    fn check_table_constraint(&mut self, inline: bool) -> Result<()> {
        if self.match_token(TokenType::Constraint) {
            if !self.check(TokenType::QuotedIdentifier) {
                self.expect_external_name()?;
            } else {
                self.skip();
            }
        }
        let references = if self.match_token(TokenType::Unique)
            || self.match_keywords(&[TokenType::PrimaryKey, TokenType::Key])
        {
            false
        } else if self.match_keywords(&[TokenType::ForeignKey, TokenType::Key])
            || inline && self.check(TokenType::References)
        {
            true
        } else if self.match_token(TokenType::Check) {
            self.expect_token(TokenType::LParen)?;
            self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return self.check_constraint_properties(false);
        } else {
            return Err(self.parse_error("Expected UNIQUE, PRIMARY KEY, FOREIGN KEY or CHECK"));
        };
        if !inline {
            self.check_iceberg_name_list(false)?;
        }
        if references {
            self.expect_token(TokenType::References)?;
            self.check_iceberg_object_name()?;
            if self.check(TokenType::LParen) {
                self.check_iceberg_name_list(false)?;
            }
        }
        self.check_constraint_properties(references)
    }

    /// Checks the properties after a constraint; MATCH and ON UPDATE or DELETE follow a foreign key, VALIDATE follows ENABLE or DISABLE (errno 1003).
    fn check_constraint_properties(&mut self, references: bool) -> Result<()> {
        loop {
            let matched = if self.match_text_seq(&["ENABLE"]) || self.match_text_seq(&["DISABLE"]) {
                let _ = self.match_text_seq(&["VALIDATE"]) || self.match_text_seq(&["NOVALIDATE"]);
                true
            } else if self.match_text_seq(&["NOT", "ENFORCED"])
                || self.match_text_seq(&["NOT", "DEFERRABLE"])
                || self.match_text_seq(&["ENFORCED"])
                || self.match_text_seq(&["DEFERRABLE"])
                || self.match_text_seq(&["RELY"])
                || self.match_text_seq(&["NORELY"])
                || self.match_text_seq(&["INITIALLY", "DEFERRED"])
                || self.match_text_seq(&["INITIALLY", "IMMEDIATE"])
            {
                true
            } else if references && self.match_token(TokenType::Match) {
                if !self.match_text_seq(&["FULL"])
                    && !self.match_text_seq(&["SIMPLE"])
                    && !self.match_text_seq(&["PARTIAL"])
                {
                    return Err(self.parse_error("Expected FULL, SIMPLE or PARTIAL"));
                }
                true
            } else if references
                && (self.match_keywords(&[TokenType::On, TokenType::Update])
                    || self.match_keywords(&[TokenType::On, TokenType::Delete]))
            {
                let action = self.match_token(TokenType::Cascade)
                    || self.match_token(TokenType::Restrict)
                    || self.match_text_seq(&["SET", "NULL"])
                    || self.match_text_seq(&["SET", "DEFAULT"])
                    || self.match_text_seq(&["NO", "ACTION"]);
                if !action {
                    return Err(self.parse_error("Expected a referential action"));
                }
                true
            } else {
                false
            };
            if !matched {
                return Ok(());
            }
        }
    }

    /// Checks one property after the columns, the LIKE source or the name.
    fn check_table_property(&mut self) -> Result<()> {
        if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            self.expect_token(TokenType::LParen)?;
            self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(());
        }
        if self.check(TokenType::Copy) && !self.check_next(TokenType::Eq) {
            self.skip();
            if self.match_text_seq(&["GRANTS"]) || self.match_text_seq(&["TAGS"]) {
                return Ok(());
            }
            return Err(self.parse_error("Expected GRANTS or TAGS after COPY"));
        }
        if self.match_token(TokenType::Copy) {
            self.expect_token(TokenType::Eq)?;
            self.expect_token(TokenType::LParen)?;
            while !self.check(TokenType::RParen) {
                if !self.match_text_seq(&["COMMENTS"])
                    && !self.match_text_seq(&["TAGS"])
                    && !self.match_text_seq(&["GRANTS"])
                {
                    return Err(self.parse_error("Expected COMMENTS, TAGS or GRANTS"));
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            return self.expect_token(TokenType::RParen).map(|_| ());
        }
        let key = self.peek().text.to_ascii_uppercase();
        if self.check_next(TokenType::Eq) && SNOWFLAKE_TABLE_PROPERTY_KEYS.contains(&key.as_str()) {
            self.skip();
            self.skip();
            return self.check_iceberg_value();
        }
        let with = self.match_token(TokenType::With);
        if with && self.match_text_seq(&["CONTACT"]) {
            return self.check_table_contacts();
        }
        if self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            self.check_iceberg_object_name()?;
            self.expect_token(TokenType::On)?;
            return self.check_iceberg_name_list(true);
        }
        if self.match_text_seq(&["AGGREGATION", "POLICY"]) {
            self.check_iceberg_object_name()?;
            if self.match_text_seq(&["ENTITY", "KEY"]) {
                self.check_iceberg_name_list(false)?;
            }
            return Ok(());
        }
        if self.match_keywords(&[TokenType::Join]) && self.match_text_seq(&["POLICY"]) {
            self.check_iceberg_object_name()?;
            if self.match_text_seq(&["ALLOWED", "JOIN", "KEYS"]) {
                self.check_iceberg_name_list(false)?;
            }
            return Ok(());
        }
        if self.match_text_seq(&["STORAGE", "LIFECYCLE", "POLICY"]) {
            self.check_iceberg_object_name()?;
            self.expect_token(TokenType::On)?;
            return self.check_iceberg_name_list(false);
        }
        if self.match_text_seq(&["TAG"]) {
            return self.check_iceberg_tags();
        }
        Err(self.parse_error("Expected a table property"))
    }

    /// Checks `( <purpose> = <contact> [ , ... ] )`.
    fn check_table_contacts(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            self.expect_external_name()?;
            self.expect_token(TokenType::Eq)?;
            self.check_iceberg_object_name()?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen).map(|_| ())
    }
}
