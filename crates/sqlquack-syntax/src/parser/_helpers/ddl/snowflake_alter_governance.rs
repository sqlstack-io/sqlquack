//! Parser ddl: the tag, contact and policy actions of a Snowflake ALTER TABLE, ICEBERG TABLE, VIEW or MATERIALIZED VIEW, checked against their grammar and kept as written.

use crate::expressions::statements::models::Raw;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The leading words of the table-level governance actions other than CONTACT and ADD of a column.
const GOVERNANCE_STARTS: &[&[&str]] = &[
    &["SET", "TAG"],
    &["SET", "AGGREGATION"],
    &["SET", "MASKING"],
    &["SET", "PROJECTION"],
    &["SET", "POLICY"],
    &["UNSET", "TAG"],
    &["UNSET", "AGGREGATION"],
    &["UNSET", "MASKING"],
    &["UNSET", "PROJECTION"],
    &["UNSET", "POLICY"],
    &["UNSET", "CONTACT"],
    &["ADD", "ROW"],
    &["DROP", "ROW"],
    &["DROP", "ALL"],
];

/// The words after a column name that make ALTER or MODIFY a governance action.
const COLUMN_GOVERNANCE_STARTS: &[&[&str]] = &[
    &["SET", "MASKING"],
    &["SET", "PROJECTION"],
    &["SET", "TAG"],
    &["SET", "POLICY"],
    &["UNSET", "MASKING"],
    &["UNSET", "PROJECTION"],
    &["UNSET", "TAG"],
    &["UNSET", "POLICY"],
];

/// The column options that make an added column a governance action.
const COLUMN_POLICY_WORDS: &[&str] = &["MASKING", "PROJECTION", "TAG"];

impl Parser {
    /// A Snowflake ALTER whose action sets or unsets a tag, contact or policy, kept as written when its grammar holds; None (position unchanged) for any other ALTER.
    pub(crate) fn parse_snowflake_alter_governance(
        &mut self,
        iceberg: bool,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        let alter_start = start - 1 - usize::from(iceberg);
        let materialized = !iceberg && self.match_text_seq(&["MATERIALIZED", "VIEW"]);
        let table = !materialized && self.match_token(TokenType::Table);
        let view = !materialized && !table && !iceberg && self.match_token(TokenType::View);
        if !materialized && !table && !view {
            self.current = start;
            return Ok(None);
        }
        if self.check_governance_head().is_err()
            || !self.starts_governance_action(table, materialized)
        {
            self.current = start;
            return Ok(None);
        }
        self.check_governance_action(table, materialized)?;
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        let sql = self.tokens_to_sql(alter_start, self.current);
        Ok(Some(Expression::Raw(Raw { sql })))
    }

    /// Checks `[ IF EXISTS ] <name>`.
    fn check_governance_head(&mut self) -> Result<()> {
        self.match_keywords(&[TokenType::If, TokenType::Exists]);
        self.check_iceberg_object_name()
    }

    /// Whether the action at the position is one this module checks; the position is unchanged.
    fn starts_governance_action(&mut self, table: bool, materialized: bool) -> bool {
        let start = self.current;
        let set = self.check(TokenType::Set);
        let contact =
            (set || self.check_text_seq(&["UNSET"])) && self.check_next_identifier("CONTACT");
        let purpose = contact
            && self.tokens.get(self.current + 2).is_some_and(|token| {
                !matches!(
                    token.token_type,
                    TokenType::Eq | TokenType::Comma | TokenType::Semicolon
                )
            });
        let found = if materialized {
            purpose
        } else if contact {
            purpose || !set || !table
        } else if GOVERNANCE_STARTS
            .iter()
            .any(|words| self.check_text_seq(words))
            || self.check(TokenType::Set) && self.check_next(TokenType::Join)
            || self.check_text_seq(&["UNSET"]) && self.check_next(TokenType::Join)
        {
            true
        } else if table && self.match_text_seq(&["ADD"]) {
            self.adds_governed_column()
        } else if self.match_text_seq(&["ALTER"]) || self.match_text_seq(&["MODIFY"]) {
            self.match_token(TokenType::Column);
            self.skip();
            COLUMN_GOVERNANCE_STARTS
                .iter()
                .any(|words| self.check_text_seq(words))
        } else {
            false
        };
        self.current = start;
        found
    }

    /// Whether the rest of the statement names a column policy or tag.
    fn adds_governed_column(&self) -> bool {
        self.tokens[self.current..]
            .iter()
            .take_while(|token| token.token_type != TokenType::Semicolon)
            .any(|token| {
                token.token_type != TokenType::String
                    && token.token_type != TokenType::QuotedIdentifier
                    && COLUMN_POLICY_WORDS.contains(&token.text.to_ascii_uppercase().as_str())
            })
    }

    /// Checks one governance action.
    fn check_governance_action(&mut self, table: bool, materialized: bool) -> Result<()> {
        if materialized {
            return Err(self.parse_error("A materialized view has no contact purposes"));
        }
        if self.match_token(TokenType::Set) {
            return self.check_governance_set();
        }
        if self.match_text_seq(&["UNSET"]) {
            return self.check_governance_unset();
        }
        if self.match_text_seq(&["ADD", "ROW", "ACCESS", "POLICY"]) {
            return self.check_row_access_addition();
        }
        if self.match_text_seq(&["DROP", "ALL", "ROW", "ACCESS", "POLICIES"]) {
            return Ok(());
        }
        if self.match_text_seq(&["DROP", "ROW", "ACCESS", "POLICY"]) {
            self.check_iceberg_object_name()?;
            if self.match_token(TokenType::Comma) {
                if !self.match_text_seq(&["ADD", "ROW", "ACCESS", "POLICY"]) {
                    return Err(self.parse_error("Expected ADD ROW ACCESS POLICY"));
                }
                return self.check_row_access_addition();
            }
            return Ok(());
        }
        if table && self.match_text_seq(&["ADD"]) {
            return self.check_added_columns();
        }
        if self.match_text_seq(&["ALTER"]) || self.match_text_seq(&["MODIFY"]) {
            return self.check_column_governance();
        }
        Err(self.parse_error("Expected a tag, contact or policy action"))
    }

    /// Checks `<name> ON ( [ <column> [ , ... ] ] )`.
    fn check_row_access_addition(&mut self) -> Result<()> {
        self.check_iceberg_object_name()?;
        self.expect_token(TokenType::On)?;
        self.check_iceberg_name_list(true)
    }

    /// Checks what follows SET: tags, a contact, or an aggregation or join policy.
    fn check_governance_set(&mut self) -> Result<()> {
        if self.match_text_seq(&["TAG"]) {
            if self.match_token(TokenType::Eq) {
                return self.check_iceberg_value();
            }
            return self.check_tag_pairs();
        }
        if self.match_text_seq(&["CONTACT"]) {
            if self.match_token(TokenType::Eq) {
                return self.check_iceberg_value();
            }
            loop {
                self.expect_external_name()?;
                self.expect_token(TokenType::Eq)?;
                self.check_iceberg_object_name()?;
                if !self.match_token(TokenType::Comma) {
                    return Ok(());
                }
            }
        }
        if self.match_text_seq(&["AGGREGATION", "POLICY"]) {
            self.check_iceberg_object_name()?;
            if self.match_text_seq(&["ENTITY", "KEY"]) {
                self.check_iceberg_name_list(false)?;
            }
            self.match_text_seq(&["FORCE"]);
            return Ok(());
        }
        if self.match_token(TokenType::Join) && self.match_text_seq(&["POLICY"]) {
            self.check_iceberg_object_name()?;
            self.match_text_seq(&["FORCE"]);
            return Ok(());
        }
        Err(self.parse_error("Expected TAG, CONTACT, AGGREGATION POLICY or JOIN POLICY"))
    }

    /// Checks `<tag> = <value> [ , <tag> = <value> ... ]`, stopping before a comma that starts another column.
    fn check_tag_pairs(&mut self) -> Result<()> {
        loop {
            self.check_tag_name()?;
            self.expect_token(TokenType::Eq)?;
            self.check_iceberg_value()?;
            let pair_follows = self.check(TokenType::Comma)
                && self
                    .tokens
                    .get(self.current + 2)
                    .is_some_and(|token| token.token_type == TokenType::Eq);
            if !pair_follows {
                return Ok(());
            }
            self.skip();
        }
    }

    /// Checks a tag name: a quoted identifier or an unreserved word, never a string.
    fn check_tag_name(&mut self) -> Result<()> {
        if self.check(TokenType::Eq) {
            return Err(self.parse_error("Expected a tag name"));
        }
        self.check_iceberg_object_name()
    }

    /// Checks what follows a table-level UNSET: properties, tags, a contact, or an aggregation or join policy.
    fn check_governance_unset(&mut self) -> Result<()> {
        loop {
            let policy = self.match_text_seq(&["AGGREGATION", "POLICY"])
                || self.match_token(TokenType::Join) && {
                    self.match_text_seq(&["POLICY"]);
                    true
                };
            if policy {
                if !self.match_token(TokenType::Comma) {
                    return Ok(());
                }
                continue;
            }
            if self.match_text_seq(&["CONTACT"]) {
                if !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Comma)
                {
                    self.expect_external_name()?;
                }
            } else if self.match_text_seq(&["TAG"]) {
                let names = !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Comma);
                if names {
                    return self.check_unset_tag_names();
                }
            } else {
                self.expect_external_name()?;
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(());
            }
        }
    }

    /// Checks `<tag> [ , <tag> ... ]`, stopping before a comma that starts another column.
    fn check_unset_tag_names(&mut self) -> Result<()> {
        loop {
            self.check_tag_name()?;
            let next_column = self.tokens.get(self.current + 2).is_some_and(|token| {
                matches!(token.text.to_ascii_uppercase().as_str(), "SET" | "UNSET")
                    && token.token_type != TokenType::QuotedIdentifier
            }) || self.check_next(TokenType::Column);
            if !self.check(TokenType::Comma) || next_column {
                return Ok(());
            }
            self.skip();
        }
    }

    /// Checks `[ COLUMN ] [ IF NOT EXISTS ] <column> <type> [ <option> ... ] [ , ... ]`, refusing (errno 1003) a column with two policies or tag lists.
    fn check_added_columns(&mut self) -> Result<()> {
        self.match_token(TokenType::Column);
        self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        loop {
            let column_start = self.current;
            if self.check_table_column()? {
                return Err(self.parse_error("Expected a data type"));
            }
            let governed = self.tokens[column_start..self.current]
                .iter()
                .filter(|token| {
                    token.token_type != TokenType::String
                        && token.token_type != TokenType::QuotedIdentifier
                        && COLUMN_POLICY_WORDS.contains(&token.text.to_ascii_uppercase().as_str())
                })
                .count();
            if governed > 1 {
                return Err(self.parse_error("Expected one policy or tag list on an added column"));
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(());
            }
            self.match_token(TokenType::Column);
        }
    }

    /// Checks `[ COLUMN ] <column> { SET | UNSET } <policy or tags> [ , ... ]` after ALTER or MODIFY; UNSET TAG or MASKING POLICY needs COLUMN (errno 1003).
    fn check_column_governance(&mut self) -> Result<()> {
        loop {
            let column = self.match_token(TokenType::Column);
            if self.check(TokenType::QuotedIdentifier) {
                self.skip();
            } else {
                self.expect_external_name()?;
            }
            if self.match_token(TokenType::Set) {
                if self.match_text_seq(&["MASKING", "POLICY"]) {
                    self.check_iceberg_object_name()?;
                    if self.match_text_seq(&["USING"]) {
                        self.check_iceberg_name_list(false)?;
                    }
                    self.match_text_seq(&["FORCE"]);
                } else if self.match_text_seq(&["PROJECTION", "POLICY"]) {
                    self.check_iceberg_object_name()?;
                    self.match_text_seq(&["FORCE"]);
                } else if self.match_text_seq(&["TAG"]) {
                    self.check_tag_pairs()?;
                } else {
                    return Err(
                        self.parse_error("Expected MASKING POLICY, PROJECTION POLICY or TAG")
                    );
                }
            } else if self.match_text_seq(&["UNSET"]) {
                if !column && (self.check_text_seq(&["TAG"]) || self.check_text_seq(&["MASKING"])) {
                    return Err(self.parse_error("Expected COLUMN before the column name"));
                }
                if self.match_text_seq(&["TAG"]) {
                    self.check_unset_tag_names()?;
                } else if !self.match_text_seq(&["MASKING", "POLICY"])
                    && !self.match_text_seq(&["PROJECTION", "POLICY"])
                {
                    return Err(
                        self.parse_error("Expected MASKING POLICY, PROJECTION POLICY or TAG")
                    );
                }
            } else {
                return Err(self.parse_error("Expected SET or UNSET"));
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(());
            }
        }
    }
}
