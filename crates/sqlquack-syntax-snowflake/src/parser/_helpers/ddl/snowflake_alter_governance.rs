//! Parser ddl: the tag, contact and policy actions of a Snowflake ALTER TABLE, ICEBERG TABLE, VIEW or MATERIALIZED VIEW, checked against their grammar.

use crate::parser::_helpers::ddl::snowflake_alter::{alter_object, property};
use crate::parser::_helpers::ddl::snowflake_external_table::policy_property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// One governance action: its keywords, operands and properties.
type GovernanceAction = (String, Vec<Expression>, Vec<ObjectProperty>);

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
    /// A Snowflake ALTER whose action sets or unsets a tag, contact or policy, when its grammar holds; None (position unchanged) for any other ALTER.
    pub(crate) fn parse_snowflake_alter_governance(
        &mut self,
        iceberg: bool,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        let materialized = !iceberg && self.match_text_seq(&["MATERIALIZED", "VIEW"]);
        let table = !materialized && self.match_token(TokenType::Table);
        let view = !materialized && !table && !iceberg && self.match_token(TokenType::View);
        if !materialized && !table && !view {
            self.current = start;
            return Ok(None);
        }
        let head = self.current;
        if self.check_governance_head().is_err()
            || !self.starts_governance_action(table, materialized)
        {
            self.current = start;
            return Ok(None);
        }
        self.current = head;
        let if_exists = self.match_keywords(&[TokenType::If, TokenType::Exists]);
        let name = self.parse_iceberg_object_name()?;
        let (action, operands, properties) = self.parse_governance_action(table, materialized)?;
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        let kind = match (iceberg, materialized, table) {
            (true, ..) => "ICEBERG TABLE",
            (_, true, _) => "MATERIALIZED VIEW",
            (_, _, true) => "TABLE",
            _ => "VIEW",
        };
        Ok(Some(alter_object(
            kind,
            (name, if_exists),
            action,
            (operands, properties),
        )))
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

    /// One governance action.
    fn parse_governance_action(
        &mut self,
        table: bool,
        materialized: bool,
    ) -> Result<GovernanceAction> {
        if materialized {
            return Err(self.parse_error("A materialized view has no contact purposes"));
        }
        if self.match_token(TokenType::Set) {
            return Ok(("SET".to_owned(), Vec::new(), self.parse_governance_set()?));
        }
        if self.match_text_seq(&["UNSET"]) {
            return Ok((
                "UNSET".to_owned(),
                Vec::new(),
                self.parse_governance_unset()?,
            ));
        }
        if self.match_text_seq(&["ADD", "ROW", "ACCESS", "POLICY"]) {
            let added = self.parse_row_access_addition()?;
            return Ok(("ADD".to_owned(), Vec::new(), vec![added]));
        }
        if self.match_text_seq(&["DROP", "ALL", "ROW", "ACCESS", "POLICIES"]) {
            return Ok((
                "DROP ALL ROW ACCESS POLICIES".to_owned(),
                Vec::new(),
                Vec::new(),
            ));
        }
        if self.match_text_seq(&["DROP", "ROW", "ACCESS", "POLICY"]) {
            let dropped = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            let mut properties: Vec<ObjectProperty> = Vec::new();
            if self.match_token(TokenType::Comma) {
                if !self.match_text_seq(&["ADD", "ROW", "ACCESS", "POLICY"]) {
                    return Err(self.parse_error("Expected ADD ROW ACCESS POLICY"));
                }
                properties.push(self.parse_row_access_addition()?);
            }
            return Ok((
                "DROP ROW ACCESS POLICY".to_owned(),
                vec![dropped],
                properties,
            ));
        }
        if table && self.match_text_seq(&["ADD"]) {
            return Ok((
                "ADD COLUMN".to_owned(),
                self.parse_added_columns()?,
                Vec::new(),
            ));
        }
        let modify = self.match_text_seq(&["MODIFY"]);
        if modify || self.match_text_seq(&["ALTER"]) {
            let action = match modify {
                true => "MODIFY COLUMN",
                false => "ALTER COLUMN",
            };
            return Ok((
                action.to_owned(),
                Vec::new(),
                self.parse_column_governance()?,
            ));
        }
        Err(self.parse_error("Expected a tag, contact or policy action"))
    }

    /// `<name> ON ( [ <column> [ , ... ] ] )` after ADD ROW ACCESS POLICY.
    fn parse_row_access_addition(&mut self) -> Result<ObjectProperty> {
        let policy = self.parse_iceberg_object_name()?;
        self.expect_token(TokenType::On)?;
        let Expression::Tuple(columns) = self.parse_iceberg_name_list(true)? else {
            unreachable!("a name list is a tuple");
        };
        Ok(policy_property(policy, columns.expressions))
    }

    /// What follows SET: tags, a contact, or an aggregation or join policy.
    fn parse_governance_set(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.match_text_seq(&["TAG"]) {
            if self.match_token(TokenType::Eq) {
                return Ok(vec![property(
                    "TAG",
                    None,
                    Some(self.parse_iceberg_value()?),
                )]);
            }
            return Ok(tag_pairs(self.parse_tag_pairs()?));
        }
        if self.match_text_seq(&["CONTACT"]) {
            if self.match_token(TokenType::Eq) {
                let value = self.parse_iceberg_value()?;
                return Ok(vec![property("CONTACT", None, Some(value))]);
            }
            let mut contacts: Vec<ObjectProperty> = Vec::new();
            loop {
                let purpose = self.parse_external_name()?;
                self.expect_token(TokenType::Eq)?;
                let contact = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
                contacts.push(property("CONTACT", Some(purpose), Some(contact)));
                if !self.match_token(TokenType::Comma) {
                    return Ok(contacts);
                }
            }
        }
        if self.match_text_seq(&["AGGREGATION", "POLICY"]) {
            let mut expressions = vec![Expression::Table(Box::new(
                self.parse_iceberg_object_name()?,
            ))];
            if self.match_text_seq(&["ENTITY", "KEY"]) {
                expressions.push(self.parse_iceberg_name_list(false)?);
            }
            return Ok(self.forced_policy("AGGREGATION POLICY", expressions));
        }
        if self.match_token(TokenType::Join) && self.match_text_seq(&["POLICY"]) {
            let policy = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            return Ok(self.forced_policy("JOIN POLICY", vec![policy]));
        }
        Err(self.parse_error("Expected TAG, CONTACT, AGGREGATION POLICY or JOIN POLICY"))
    }

    /// The policy property of `kind` with its operands, then FORCE when written.
    fn forced_policy(&mut self, kind: &str, expressions: Vec<Expression>) -> Vec<ObjectProperty> {
        let value = Expression::Tuple(Box::new(Tuple { expressions }));
        let mut properties = vec![property(kind, None, Some(value))];
        if self.match_text_seq(&["FORCE"]) {
            properties.push(property("FORCE", None, None));
        }
        properties
    }

    /// `<tag> = <value> [ , <tag> = <value> ... ]`, stopping before a comma that starts another column.
    fn parse_tag_pairs(&mut self) -> Result<Vec<Expression>> {
        let mut pairs: Vec<Expression> = Vec::new();
        loop {
            let tag = Expression::Table(Box::new(self.parse_tag_name()?));
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_iceberg_value()?;
            pairs.push(Expression::PropertyEQ(Box::new(BinaryOp::new(tag, value))));
            let pair_follows = self.check(TokenType::Comma)
                && self
                    .tokens
                    .get(self.current + 2)
                    .is_some_and(|token| token.token_type == TokenType::Eq);
            if !pair_follows {
                return Ok(pairs);
            }
            self.skip();
        }
    }

    /// A tag name: a quoted identifier or an unreserved word, never a string.
    fn parse_tag_name(&mut self) -> Result<sqlquack_syntax::expressions::names::models::TableRef> {
        if self.check(TokenType::Eq) {
            return Err(self.parse_error("Expected a tag name"));
        }
        self.parse_iceberg_object_name()
    }

    /// What follows a table-level UNSET: properties, tags, a contact, or an aggregation or join policy, each without a value.
    fn parse_governance_unset(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            if self.match_text_seq(&["AGGREGATION", "POLICY"]) {
                properties.push(property("AGGREGATION POLICY", None, None));
            } else if self.match_token(TokenType::Join) {
                self.match_text_seq(&["POLICY"]);
                properties.push(property("JOIN POLICY", None, None));
            } else if self.match_text_seq(&["CONTACT"]) {
                let purpose = match !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Comma)
                {
                    true => Some(self.parse_external_name()?),
                    false => None,
                };
                properties.push(property("CONTACT", purpose, None));
            } else if self.match_text_seq(&["TAG"]) {
                let names = !self.is_at_end()
                    && !self.check(TokenType::Semicolon)
                    && !self.check(TokenType::Comma);
                if names {
                    let tags = self.parse_unset_tag_names()?;
                    properties.push(property("TAG", None, Some(tuple(tags))));
                    return Ok(properties);
                }
                properties.push(property("TAG", None, None));
            } else {
                let name = self.parse_external_name()?;
                properties.push(property(&name.name.to_ascii_uppercase(), None, None));
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// `<tag> [ , <tag> ... ]`, stopping before a comma that starts another column.
    fn parse_unset_tag_names(&mut self) -> Result<Vec<Expression>> {
        let mut tags: Vec<Expression> = Vec::new();
        loop {
            tags.push(Expression::Table(Box::new(self.parse_tag_name()?)));
            let next_column = self.tokens.get(self.current + 2).is_some_and(|token| {
                matches!(token.text.to_ascii_uppercase().as_str(), "SET" | "UNSET")
                    && token.token_type != TokenType::QuotedIdentifier
            }) || self.check_next(TokenType::Column);
            if !self.check(TokenType::Comma) || next_column {
                return Ok(tags);
            }
            self.skip();
        }
    }

    /// `[ COLUMN ] [ IF NOT EXISTS ] <column> <type> [ <option> ... ] [ , ... ]` as column definitions, refusing (errno 1003) a column with two policies or tag lists.
    fn parse_added_columns(&mut self) -> Result<Vec<Expression>> {
        self.match_token(TokenType::Column);
        self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let column_start = self.current;
            let column = self.parse_checked_table_column()?;
            if column.no_type {
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
            columns.push(Expression::ColumnDef(Box::new(column)));
            if !self.match_token(TokenType::Comma) {
                return Ok(columns);
            }
            self.match_token(TokenType::Column);
        }
    }

    /// `[ COLUMN ] <column> { SET | UNSET } <policy or tags> [ , ... ]` after ALTER or MODIFY, one property per column named for it; UNSET TAG or MASKING POLICY needs COLUMN (errno 1003).
    fn parse_column_governance(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let column = self.match_token(TokenType::Column);
            let name = Some(self.parse_external_name()?);
            if self.match_token(TokenType::Set) {
                let (kind, mut expressions) = if self.match_text_seq(&["MASKING", "POLICY"]) {
                    let mut expressions = vec![Expression::Table(Box::new(
                        self.parse_iceberg_object_name()?,
                    ))];
                    if self.match_text_seq(&["USING"]) {
                        expressions.push(self.parse_iceberg_name_list(false)?);
                    }
                    ("SET MASKING POLICY", expressions)
                } else if self.match_text_seq(&["PROJECTION", "POLICY"]) {
                    let policy = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
                    ("SET PROJECTION POLICY", vec![policy])
                } else if self.match_text_seq(&["TAG"]) {
                    ("SET TAG", self.parse_tag_pairs()?)
                } else {
                    return Err(
                        self.parse_error("Expected MASKING POLICY, PROJECTION POLICY or TAG")
                    );
                };
                if kind != "SET TAG" && self.match_text_seq(&["FORCE"]) {
                    expressions.push(Expression::Var(Box::new(Var {
                        this: "FORCE".to_owned(),
                    })));
                }
                properties.push(property(kind, name, Some(tuple(expressions))));
            } else if self.match_text_seq(&["UNSET"]) {
                if !column && (self.check_text_seq(&["TAG"]) || self.check_text_seq(&["MASKING"])) {
                    return Err(self.parse_error("Expected COLUMN before the column name"));
                }
                if self.match_text_seq(&["TAG"]) {
                    let tags = self.parse_unset_tag_names()?;
                    properties.push(property("UNSET TAG", name, Some(tuple(tags))));
                } else if self.match_text_seq(&["MASKING", "POLICY"]) {
                    properties.push(property("UNSET MASKING POLICY", name, None));
                } else if self.match_text_seq(&["PROJECTION", "POLICY"]) {
                    properties.push(property("UNSET PROJECTION POLICY", name, None));
                } else {
                    return Err(
                        self.parse_error("Expected MASKING POLICY, PROJECTION POLICY or TAG")
                    );
                }
            } else {
                return Err(self.parse_error("Expected SET or UNSET"));
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }
}

/// A tuple of the expressions.
fn tuple(expressions: Vec<Expression>) -> Expression {
    Expression::Tuple(Box::new(Tuple { expressions }))
}

/// One TAG property per `<tag> = <value>` pair.
fn tag_pairs(pairs: Vec<Expression>) -> Vec<ObjectProperty> {
    pairs
        .into_iter()
        .map(|pair| property("TAG", None, Some(pair)))
        .collect()
}
