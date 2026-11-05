//! Snowflake ALTER actions of TABLE, ICEBERG TABLE, SEQUENCE, VIEW and SHARE that the generic ALTER parsers lack.

use crate::expressions::names::models::Identifier;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::_helpers::ddl::snowflake_alter::alter_object;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The actions only an ICEBERG TABLE takes, as their leading words.
const ICEBERG_ACTIONS: &[&[&str]] = &[
    &["CREATE", "OR", "REPLACE", "VERSION_TAG"],
    &["CREATE", "VERSION_TAG"],
    &["REPLACE", "VERSION_TAG"],
    &["ALTER", "VERSION_TAG"],
    &["DROP", "VERSION_TAG"],
    &["REPLACE", "PARTITION", "BY"],
    &["UPGRADE", "TO", "ICEBERG", "VERSION"],
    &["OPTIMIZE"],
];

/// The storage lifecycle policy actions of an ALTER TABLE.
const LIFECYCLE_ACTIONS: &[&[&str]] = &[
    &["ADD", "STORAGE", "LIFECYCLE", "POLICY"],
    &["DROP", "STORAGE", "LIFECYCLE", "POLICY"],
];

/// The retention units of a version tag's RETAIN.
const RETAIN_UNITS: &[&[&str]] = &[&["DAYS"], &["HOURS"], &["MINUTES"]];

/// The operands and properties of one ALTER action.
type ActionParts = (Vec<Expression>, Vec<ObjectProperty>);

impl Parser {
    /// Parse a Snowflake ALTER action this module owns, or None (position unchanged) for any other.
    pub(crate) fn parse_alter_snowflake_table_action(
        &mut self,
        iceberg: bool,
    ) -> Result<Option<Expression>> {
        if self.check_identifier("USER")
            && self
                .peek_nth(1)
                .is_some_and(|token| token.token_type != TokenType::QuotedIdentifier)
            && self.text_at(1).as_deref() == Some("GROUP")
        {
            return Err(self.parse_error("Snowflake has ALTER ORGANIZATION USER GROUP"));
        }
        let start = self.current;
        let found = if self.match_token(TokenType::Table) {
            self.parse_alter_table_extension(iceberg)
        } else if iceberg {
            Ok(None)
        } else if self.match_token(TokenType::Sequence) {
            self.parse_alter_kind_extension("SEQUENCE", &[&["RENAME", "TO"], &["UNSET"]])
        } else if self.match_token(TokenType::View) {
            self.parse_alter_kind_extension("VIEW", &[&["RENAME", "TO"]])
        } else if self.match_identifier("SHARE") {
            return self.parse_alter_share();
        } else {
            Ok(None)
        };
        let found = found?;
        if found.is_none() {
            self.current = start;
        }
        Ok(found)
    }

    /// Parse the head and an ICEBERG or lifecycle action of an ALTER [ ICEBERG ] TABLE, or None.
    fn parse_alter_table_extension(&mut self, iceberg: bool) -> Result<Option<Expression>> {
        let head = self.parse_alter_object_head()?;
        if !iceberg && self.match_text_seq(&["RENAME"]) && !self.check(TokenType::Column) {
            self.expect_token(TokenType::To)?;
            self.parse_object_name()?;
            return Ok(None);
        }
        if !iceberg && self.match_text_seq(&["SWAP", "WITH"]) {
            self.parse_object_name()?;
            return Ok(None);
        }
        if !iceberg && self.check_identifier("OPTIMIZE") {
            return Err(self.parse_error("OPTIMIZE is an action of ALTER ICEBERG TABLE"));
        }
        if !iceberg && self.check(TokenType::Set) && self.check_next(TokenType::Semicolon) {
            return Err(self.parse_error("SET names at least one property"));
        }
        let choices = if iceberg {
            ICEBERG_ACTIONS
        } else {
            LIFECYCLE_ACTIONS
        };
        let Some(words) = choices.iter().find(|words| self.match_text_seq(words)) else {
            return Ok(None);
        };
        let kind = if iceberg { "ICEBERG TABLE" } else { "TABLE" };
        let (action, parts) = self.parse_table_extension_action(words.join(" "))?;
        Ok(Some(alter_object(kind, head, action, parts)))
    }

    /// Parse what follows the words of a table extension action.
    fn parse_table_extension_action(
        &mut self,
        mut action: String,
    ) -> Result<(String, ActionParts)> {
        let parts = match action.as_str() {
            "CREATE VERSION_TAG" | "CREATE OR REPLACE VERSION_TAG" => {
                if self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                    action.push_str(" IF NOT EXISTS");
                }
                let tag = self.parse_tag_operand()?;
                let mut properties: Vec<ObjectProperty> = Vec::new();
                if self.match_text_seq(&["AS", "OF"]) {
                    properties = self.parse_version_point()?;
                }
                (vec![tag], properties)
            }
            "REPLACE VERSION_TAG" | "ALTER VERSION_TAG" => {
                let tag = self.parse_tag_operand()?;
                self.expect_alter_words(&[&["AS", "OF"]])?;
                (vec![tag], self.parse_version_point()?)
            }
            "DROP VERSION_TAG" => {
                if self.match_keywords(&[TokenType::If, TokenType::Exists]) {
                    action.push_str(" IF EXISTS");
                }
                (vec![self.parse_tag_operand()?], vec![])
            }
            "REPLACE PARTITION BY" => {
                let old = self.parse_partition_expression()?;
                self.expect_token(TokenType::With)?;
                (vec![old, self.parse_partition_expression()?], vec![])
            }
            "UPGRADE TO ICEBERG VERSION" => (vec![self.parse_number_operand()?], vec![]),
            "OPTIMIZE" => {
                self.expect_token(TokenType::LParen)?;
                self.expect_alter_words(&[&["REMOVE_POSITION_DELETES"]])?;
                self.expect_token(TokenType::RParen)?;
                (
                    vec![],
                    vec![property("REMOVE_POSITION_DELETES", None, None)],
                )
            }
            "ADD STORAGE LIFECYCLE POLICY" => {
                let policy = self.parse_tag_operand()?;
                self.expect_token(TokenType::On)?;
                (vec![policy, self.parse_column_names()?], vec![])
            }
            _ => (vec![], vec![]),
        };
        Ok((action, parts))
    }

    /// Parse the head and one of `actions` of an ALTER `kind`, or None for another action.
    fn parse_alter_kind_extension(
        &mut self,
        kind: &str,
        actions: &[&[&str]],
    ) -> Result<Option<Expression>> {
        let head = self.parse_alter_object_head()?;
        let Some(words) = actions.iter().find(|words| self.match_text_seq(words)) else {
            return Ok(None);
        };
        let action = words.join(" ");
        let parts: ActionParts = match action.as_str() {
            "UNSET" => {
                self.expect_alter_words(&[&["COMMENT"]])?;
                (vec![], vec![property("COMMENT", None, None)])
            }
            _ => (vec![self.parse_tag_operand()?], vec![]),
        };
        Ok(Some(alter_object(kind, head, action, parts)))
    }

    /// Parse ALTER SHARE: UNSET COMMENT typed; ADD, REMOVE, SET and other UNSETs are left to the fallback.
    fn parse_alter_share(&mut self) -> Result<Option<Expression>> {
        let start = self.current - 1;
        let head = self.parse_alter_object_head()?;
        if self.match_text_seq(&["UNSET", "COMMENT"]) {
            let parts = (vec![], vec![property("COMMENT", None, None)]);
            return Ok(Some(alter_object("SHARE", head, "UNSET".to_owned(), parts)));
        }
        match (
            self.text_at(0).unwrap_or_default().as_str(),
            self.text_at(1).as_deref(),
        ) {
            ("UNSET", None | Some(";")) => {
                return Err(self.parse_error("Expected a property after UNSET"))
            }
            ("ADD" | "REMOVE" | "SET" | "UNSET", _) => {}
            _ => return Err(self.parse_error("Expected ADD, REMOVE, SET or UNSET")),
        }
        self.current = start;
        Ok(None)
    }

    /// Parse a name operand: an identifier, never a string or an unquoted reserved keyword.
    fn parse_tag_operand(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse `{ VERSION <snapshot_id> | TIMESTAMP <timestamp_ms> } [ RETAIN <num> { DAYS | HOURS | MINUTES } ]`.
    fn parse_version_point(&mut self) -> Result<Vec<ObjectProperty>> {
        let point = self.expect_alter_words(&[&["VERSION"], &["TIMESTAMP"]])?;
        let mut properties = vec![property(&point, None, Some(self.parse_number_operand()?))];
        if self.match_identifier("RETAIN") {
            let count = self.parse_number_operand()?;
            let unit = self.expect_alter_words(RETAIN_UNITS)?;
            properties.push(property("RETAIN", Some(Identifier::new(unit)), Some(count)));
        }
        Ok(properties)
    }

    /// Parse an integer literal.
    fn parse_number_operand(&mut self) -> Result<Expression> {
        if !self.check(TokenType::Number) {
            return Err(self.parse_error("Expected a number"));
        }
        self.parse_primary()
    }

    /// Parse `( <expr> )`, one partition expression.
    fn parse_partition_expression(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let expression = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        Ok(expression)
    }

    /// Parse `( <col_name> [ , ... ] )`.
    fn parse_column_names(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions = vec![self.parse_tag_operand()?];
        while self.match_token(TokenType::Comma) {
            expressions.push(self.parse_tag_operand()?);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }
}
