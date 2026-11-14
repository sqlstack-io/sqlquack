//! Snowflake ALTER actions of TABLE, ICEBERG TABLE, SEQUENCE, VIEW and SHARE that the generic ALTER parsers lack.

use crate::parser::_helpers::ddl::snowflake_alter::alter_object;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::AlterTableAction;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The actions only an ICEBERG TABLE takes, as their leading words.
const ICEBERG_ACTIONS: &[&[&str]] = &[
    &["CREATE", "OR", "REPLACE", "VERSION_TAG"],
    &["CREATE", "VERSION_TAG"],
    &["REPLACE", "VERSION_TAG"],
    &["ALTER", "VERSION_TAG"],
    &["DROP", "VERSION_TAG"],
    &["REPLACE", "PARTITION", "BY"],
    &["ADD", "PARTITION", "BY"],
    &["DROP", "PARTITION", "BY"],
    &["UPGRADE", "TO", "ICEBERG", "VERSION"],
    &["OPTIMIZE"],
    &["CONVERT", "TO", "MANAGED"],
    &["REFRESH"],
];

/// The storage lifecycle policy actions of an ALTER TABLE.
const LIFECYCLE_ACTIONS: &[&[&str]] = &[
    &["ADD", "STORAGE", "LIFECYCLE", "POLICY"],
    &["DROP", "STORAGE", "LIFECYCLE", "POLICY"],
];

/// The search optimization actions of an ALTER [ ICEBERG ] TABLE.
const SEARCH_ACTIONS: &[&[&str]] = &[
    &["ADD", "SEARCH", "OPTIMIZATION"],
    &["DROP", "SEARCH", "OPTIMIZATION"],
];

/// The reclustering actions of an ALTER [ ICEBERG ] TABLE.
const RECLUSTER_ACTIONS: &[&[&str]] = &[
    &["RECLUSTER"],
    &["SUSPEND", "RECLUSTER"],
    &["RESUME", "RECLUSTER"],
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
            self.parse_alter_kind_extension("SEQUENCE", &[&["RENAME", "TO"], &["UNSET"], &["SET"]])
        } else if self.match_token(TokenType::View) {
            self.parse_alter_view_extension()
        } else if self.match_identifier("SHARE") {
            return self.parse_alter_share().map(Some);
        } else {
            Ok(None)
        };
        let found = found?;
        if found.is_none() {
            self.current = start;
        }
        Ok(found)
    }

    /// Parse the head and an ICEBERG, lifecycle, search optimization or reclustering action of an ALTER [ ICEBERG ] TABLE, or None.
    fn parse_alter_table_extension(&mut self, iceberg: bool) -> Result<Option<Expression>> {
        let head = self.parse_alter_object_head()?;
        if !iceberg
            && self.match_text_seq(&["RENAME"])
            && !self.check(TokenType::Column)
            && !self.check(TokenType::Constraint)
        {
            self.expect_token(TokenType::To)?;
            self.parse_object_name()?;
            return Ok(None);
        }
        if !iceberg && self.match_text_seq(&["SWAP", "WITH"]) {
            self.parse_object_name()?;
            return Ok(None);
        }
        if !iceberg && self.check_text_seq(&["DROP", "PARTITION", "BY"]) {
            return Err(self.parse_error("DROP PARTITION BY is an action of ALTER ICEBERG TABLE"));
        }
        let renames_unnamed = self.check(TokenType::Rename)
            && !matches!(
                self.peek_nth(1).map(|token| token.token_type),
                Some(TokenType::Column | TokenType::Constraint | TokenType::To)
            );
        if iceberg && renames_unnamed {
            return Err(self.parse_error("Expected COLUMN or TO after RENAME"));
        }
        let modifies_function = self.check_text_seq(&["MODIFY", "DATA", "FUNCTION"])
            || self.check_text_seq(&["MODIFY", "DATA", "METRIC"])
                && !self.check_text_seq(&["MODIFY", "DATA", "METRIC", "FUNCTION"])
            || self.check_text_seq(&["MODIFY", "METRIC", "FUNCTION"]);
        if modifies_function {
            return Err(self.parse_error("Expected MODIFY DATA METRIC FUNCTION"));
        }
        if !iceberg && self.check_identifier("OPTIMIZE") {
            return Err(self.parse_error("OPTIMIZE is an action of ALTER ICEBERG TABLE"));
        }
        if !iceberg && self.check(TokenType::Set) && self.check_next(TokenType::Semicolon) {
            return Err(self.parse_error("SET names at least one property"));
        }
        let drops_version_tag_column = self.check_text_seq(&["DROP", "VERSION_TAG"])
            && self.tokens.get(self.current + 2).is_none_or(|token| {
                matches!(token.token_type, TokenType::Semicolon | TokenType::Comma)
            });
        if drops_version_tag_column {
            return Ok(None);
        }
        let choices = if iceberg {
            ICEBERG_ACTIONS
        } else {
            LIFECYCLE_ACTIONS
        };
        let Some(words) = SEARCH_ACTIONS
            .iter()
            .chain(RECLUSTER_ACTIONS)
            .chain(choices)
            .find(|words| self.match_text_seq(words))
        else {
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
            "ADD PARTITION BY" | "DROP PARTITION BY" => {
                (vec![self.parse_partition_expression()?], vec![])
            }
            "ADD SEARCH OPTIMIZATION" | "DROP SEARCH OPTIMIZATION" => {
                (self.parse_search_targets(&action)?, vec![])
            }
            "RECLUSTER" => {
                let mut properties: Vec<ObjectProperty> = Vec::new();
                if self.match_identifier("MAX_SIZE") {
                    self.expect_token(TokenType::Eq)?;
                    // Snowflake parses a string here and rejects it only semantically.
                    let size = if self.check(TokenType::String) {
                        self.parse_primary()?
                    } else {
                        self.parse_number_operand()?
                    };
                    properties.push(property("MAX_SIZE", None, Some(size)));
                }
                if self.match_token(TokenType::Where) {
                    properties.push(property("WHERE", None, Some(self.parse_expression()?)));
                }
                (vec![], properties)
            }
            "REFRESH" => match self.check(TokenType::String) {
                true => (vec![self.parse_primary()?], vec![]),
                false => (vec![], vec![]),
            },
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
            "CONVERT TO MANAGED" => {
                let mut properties: Vec<ObjectProperty> = Vec::new();
                while !self.is_at_end() && !self.check(TokenType::Semicolon) {
                    let key = self.peek().text.to_ascii_uppercase();
                    self.expect_external_name()?;
                    self.expect_token(TokenType::Eq)?;
                    properties.push(property(&key, None, Some(self.parse_primary()?)));
                }
                (vec![], properties)
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

    /// Snowflake's `KEY`, `UNIQUE [ ( <col> , ... ) ]` or `FOREIGN KEY [ ( <col> , ... ) ]` after DROP, with an optional CASCADE or RESTRICT; None (position unchanged) for any other drop.
    pub(crate) fn parse_snowflake_drop_key(&mut self) -> Result<Option<AlterTableAction>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        let start = self.current;
        let listed = if self.match_token(TokenType::Unique)
            || self.match_keywords(&[TokenType::ForeignKey, TokenType::Key])
        {
            true
        } else if self.check(TokenType::Key)
            && self.peek_nth(1).is_none_or(|token| {
                matches!(token.token_type, TokenType::Semicolon | TokenType::Comma)
            })
        {
            self.skip();
            false
        } else {
            return Ok(None);
        };
        if listed && self.check(TokenType::LParen) {
            self.parse_column_names()?;
        }
        let _ = self.match_token(TokenType::Cascade) || self.match_token(TokenType::Restrict);
        Ok(Some(AlterTableAction::Raw {
            sql: format!("DROP {}", self.tokens_to_sql(start, self.current)),
        }))
    }

    /// Snowflake's `<property> = <value> [ ... ]` after a table's SET, separated by spaces or commas; None (position unchanged) when no `<name> =` follows SET or a stage option list does.
    pub(crate) fn parse_snowflake_table_set(&mut self) -> Result<Option<AlterTableAction>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        if self.check(TokenType::Eq) {
            return Err(self.parse_error("Expected a property name"));
        }
        let mut properties: Vec<(String, Expression)> = Vec::new();
        while self.check_next(TokenType::Eq)
            && !self.check_identifier("STAGE_FILE_FORMAT")
            && !self.check_identifier("STAGE_COPY_OPTIONS")
        {
            let key = self.expect_identifier_or_keyword()?;
            self.expect_token(TokenType::Eq)?;
            properties.push((key, self.parse_expression()?));
            if self.check(TokenType::Comma)
                && self
                    .tokens
                    .get(self.current + 2)
                    .is_some_and(|token| token.token_type == TokenType::Eq)
            {
                self.skip();
            }
        }
        Ok((!properties.is_empty()).then_some(AlterTableAction::SetProperty { properties }))
    }

    /// Snowflake's `CONSTRAINT <name> TO <name>` after RENAME; None (position unchanged) for any other rename.
    pub(crate) fn parse_snowflake_rename_constraint(&mut self) -> Result<Option<AlterTableAction>> {
        let start = self.current;
        if !self.is_snowflake() || !self.match_token(TokenType::Constraint) {
            return Ok(None);
        }
        self.parse_external_name()?;
        self.expect_token(TokenType::To)?;
        self.parse_external_name()?;
        Ok(Some(AlterTableAction::Raw {
            sql: format!("RENAME {}", self.tokens_to_sql(start, self.current)),
        }))
    }

    /// Parse the head and a RENAME TO, property SET or property UNSET of an ALTER VIEW, or None for another action.
    fn parse_alter_view_extension(&mut self) -> Result<Option<Expression>> {
        let head = self.parse_alter_head_or_omitted("VIEW")?;
        if self.is_statement_end_at(0) {
            return Err(self.parse_error("Expected an ALTER VIEW action"));
        }
        let sets_property = self.check(TokenType::Set)
            && (self.check_next_identifier("SECURE")
                || self
                    .tokens
                    .get(self.current + 2)
                    .is_some_and(|token| token.token_type == TokenType::Eq));
        let unsets_property =
            self.check_text_seq(&["UNSET"]) && !self.check_next_identifier("TBLPROPERTIES");
        let action = if self.match_text_seq(&["RENAME", "TO"]) {
            return Ok(Some(alter_object(
                "VIEW",
                head,
                "RENAME TO".to_owned(),
                (vec![self.parse_tag_operand()?], vec![]),
            )));
        } else if sets_property || unsets_property {
            self.skip();
            if sets_property {
                "SET"
            } else {
                "UNSET"
            }
        } else {
            return Ok(None);
        };
        let (action, parts) = self.parse_resource_action("VIEW", action.to_owned())?;
        Ok(Some(alter_object("VIEW", head, action, parts)))
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
            "SET" => (vec![], self.parse_sequence_set()?),
            _ => (vec![self.parse_tag_operand()?], vec![]),
        };
        Ok(Some(alter_object(kind, head, action, parts)))
    }

    /// A Snowflake ALTER SEQUENCE SET: at least one of INCREMENT [ BY ] [ = ] <n>, ORDER, NOORDER and COMMENT = <value>.
    fn parse_sequence_set(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_statement_end_at(0) {
            if self.match_token(TokenType::Increment) || self.match_identifier("INCREMENT") {
                self.match_token(TokenType::By);
                self.match_token(TokenType::Eq);
                let start = self.current;
                self.parse_signed_integer()?;
                self.current = start;
                properties.push(property(
                    "INCREMENT",
                    None,
                    Some(self.parse_object_value()?),
                ));
            } else if self.match_token(TokenType::Order) {
                properties.push(property("ORDER", None, None));
            } else if self.match_identifier("NOORDER") {
                properties.push(property("NOORDER", None, None));
            } else if self.match_token(TokenType::Comment) || self.match_identifier("COMMENT") {
                self.expect_token(TokenType::Eq)?;
                properties.push(property("COMMENT", None, Some(self.parse_object_value()?)));
            } else {
                return Err(self.parse_error("Expected INCREMENT, ORDER, NOORDER or COMMENT"));
            }
        }
        if properties.is_empty() {
            return Err(self.parse_error("Expected a property to set"));
        }
        Ok(properties)
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

    /// Parse `( <expr> )`, one partition expression; a transform may take no arguments.
    fn parse_partition_expression(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let expression = self.parse_partition_item()?;
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
