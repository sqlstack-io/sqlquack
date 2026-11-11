//! Parser ddl: Snowflake CREATE DYNAMIC ICEBERG TABLE and CREATE INTERACTIVE TABLE, checked against their grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_create_table::tag_properties;
use crate::parser::_helpers::ddl::snowflake_external_table::policy_property;
use crate::parser::constants::SNOWFLAKE_DYNAMIC_TABLE_REQUIRED;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, MaskingPolicyColumnConstraint,
};
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tables::models::{ConstraintType, Tags};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::{DataType, Tuple};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE DYNAMIC ICEBERG TABLE or CREATE INTERACTIVE TABLE other than a clone; None for any other CREATE.
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
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists {
            if !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                return Err(self.parse_error("Expected IF NOT EXISTS"));
            }
            if or_replace {
                return Err(self.parse_error("OR REPLACE and IF NOT EXISTS cannot be combined"));
            }
        }
        let name = self.parse_iceberg_object_name()?;
        if self.check_identifier("CLONE") {
            self.current = start;
            return Ok(None);
        }
        let mut operands: Vec<Expression> = Vec::new();
        if self.check(TokenType::LParen) {
            operands.push(Expression::Tuple(Box::new(Tuple {
                expressions: self.parse_iceberg_columns()?,
            })));
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if interactive {
            if !self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
                return Err(self.parse_error("Expected CLUSTER BY"));
            }
            self.expect_token(TokenType::LParen)?;
            let expressions = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            let value = Expression::Tuple(Box::new(Tuple { expressions }));
            properties.push(property("CLUSTER BY", None, Some(value)));
        }
        properties.extend(self.parse_iceberg_properties(interactive)?);
        let source = if !interactive && self.match_text_seq(&["REFRESH", "USING"]) {
            self.expect_token(TokenType::LParen)?;
            if !self.check(TokenType::Insert) {
                return Err(self.parse_error("Expected INSERT in REFRESH USING ( )"));
            }
            operands.push(self.parse_statement()?);
            self.expect_token(TokenType::RParen)?;
            "REFRESH USING"
        } else {
            self.expect_token(TokenType::As)?;
            if !self.check(TokenType::Select) && !self.check(TokenType::With) {
                return Err(self.parse_error("Expected a query after AS"));
            }
            operands.push(self.parse_statement()?);
            "AS"
        };
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: match interactive {
                true => "INTERACTIVE TABLE",
                false => "DYNAMIC ICEBERG TABLE",
            }
            .to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        }))))
    }

    /// Checks an object name: a quoted identifier, or a word Snowflake does not reserve, optionally qualified.
    pub(super) fn check_iceberg_object_name(&mut self) -> Result<()> {
        self.parse_iceberg_object_name().map(|_| ())
    }

    /// An object name: a quoted identifier, or a word Snowflake does not reserve, optionally qualified.
    pub(super) fn parse_iceberg_object_name(&mut self) -> Result<TableRef> {
        if !self.check(TokenType::QuotedIdentifier) {
            let saved = self.current;
            self.expect_external_name()?;
            self.current = saved;
        }
        self.parse_object_name()
    }

    /// `( <name> [<type>] [<column option> ...] [ , ... ] )` as column definitions.
    fn parse_iceberg_columns(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let name = self.parse_iceberg_object_name()?;
            let mut column = ColumnDef::new("", DataType::Unknown);
            column.name = name.name;
            column.no_type = true;
            if !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                if self.check(TokenType::String) {
                    return Err(self.parse_error("Expected a data type"));
                }
                column.data_type = self.parse_data_type()?;
                column.no_type = false;
            }
            while !self.check(TokenType::Comma) && !self.check(TokenType::RParen) {
                column = self.parse_iceberg_column_option(column)?;
            }
            columns.push(Expression::ColumnDef(Box::new(column)));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(columns)
    }

    /// `COMMENT '<text>'`, `[WITH] MASKING POLICY <name> [USING ( <names> )]` or `[WITH] TAG ( ... )` on `column`.
    fn parse_iceberg_column_option(&mut self, mut column: ColumnDef) -> Result<ColumnDef> {
        if self.match_token(TokenType::Comment) {
            column.comment = Some(self.expect_token(TokenType::String)?.text.to_string());
            column.constraint_order.push(ConstraintType::Comment);
            return Ok(column);
        }
        self.match_token(TokenType::With);
        if self.match_text_seq(&["MASKING", "POLICY"]) {
            let policy = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            let mut expressions: Vec<Expression> = Vec::new();
            if self.match_text_seq(&["USING"]) {
                expressions.push(self.parse_iceberg_name_list(false)?);
            }
            column
                .options
                .push(Expression::MaskingPolicyColumnConstraint(Box::new(
                    MaskingPolicyColumnConstraint {
                        this: Box::new(policy),
                        expressions,
                    },
                )));
            return Ok(column);
        }
        if self.match_text_seq(&["TAG"]) {
            let expressions = self.parse_iceberg_tags()?;
            column
                .constraints
                .push(ColumnConstraint::Tags(Tags { expressions }));
            column.constraint_order.push(ConstraintType::Tags);
            return Ok(column);
        }
        Err(self.parse_error("Expected a column option"))
    }

    /// Checks `( <name> [ , ... ] )`, which may be empty only when `empty` is set.
    pub(super) fn check_iceberg_name_list(&mut self, empty: bool) -> Result<()> {
        self.parse_iceberg_name_list(empty).map(|_| ())
    }

    /// `( <name> [ , ... ] )` as a tuple of names, which may be empty only when `empty` is set.
    pub(super) fn parse_iceberg_name_list(&mut self, empty: bool) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions: Vec<Expression> = Vec::new();
        if empty && self.match_token(TokenType::RParen) {
            return Ok(Expression::Tuple(Box::new(Tuple { expressions })));
        }
        loop {
            expressions.push(Expression::Table(Box::new(
                self.parse_iceberg_object_name()?,
            )));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }

    /// `( <tag> = <value> [ , ... ] )`, each tag as `<tag> = <value>`.
    pub(super) fn parse_iceberg_tags(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut tags: Vec<Expression> = Vec::new();
        loop {
            let tag = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_iceberg_value()?;
            tags.push(Expression::PropertyEQ(Box::new(BinaryOp::new(tag, value))));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tags)
    }

    /// The properties before AS or REFRESH USING; a dynamic table needs TARGET_LAG and WAREHOUSE.
    fn parse_iceberg_properties(&mut self, interactive: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        while !self.is_at_end()
            && !self.check(TokenType::Semicolon)
            && !self.check(TokenType::As)
            && !self.check_text_seq(&["REFRESH", "USING"])
        {
            let (key, found) = match interactive {
                true => self.parse_interactive_property()?,
                false => self.parse_iceberg_property()?,
            };
            keys.extend(key);
            properties.extend(found);
        }
        let missing = SNOWFLAKE_DYNAMIC_TABLE_REQUIRED
            .iter()
            .any(|required| !keys.contains(&required.to_string()));
        if !interactive && missing {
            return Err(self.parse_error("Expected TARGET_LAG and WAREHOUSE"));
        }
        Ok(properties)
    }

    /// One property, with the key of a `<key> = <value>` one.
    fn parse_iceberg_property(&mut self) -> Result<(Option<String>, Vec<ObjectProperty>)> {
        if self.check(TokenType::Copy) && !self.check_next(TokenType::Eq) {
            self.skip();
            if self.match_text_seq(&["GRANTS"]) {
                return Ok((None, vec![property("COPY GRANTS", None, None)]));
            }
            if self.match_text_seq(&["TAGS"]) {
                return Ok((None, vec![property("COPY TAGS", None, None)]));
            }
            return Err(self.parse_error("Expected GRANTS or TAGS after COPY"));
        }
        if self.match_text_seq(&["REQUIRE", "USER"]) {
            return Ok((None, vec![property("REQUIRE USER", None, None)]));
        }
        if self.match_text_seq(&["EXECUTE", "AS", "USER"]) {
            let mut expressions = vec![Expression::Table(Box::new(
                self.parse_iceberg_object_name()?,
            ))];
            if self.match_text_seq(&["USE", "SECONDARY", "ROLES"]) {
                expressions.push(match self.match_token(TokenType::All) {
                    true => Expression::Var(Box::new(Var {
                        this: "ALL".to_owned(),
                    })),
                    false => Expression::Table(Box::new(self.parse_iceberg_object_name()?)),
                });
            }
            let value = Expression::Tuple(Box::new(Tuple { expressions }));
            return Ok((None, vec![property("EXECUTE AS USER", None, Some(value))]));
        }
        if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            self.expect_token(TokenType::LParen)?;
            let expressions = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            let value = Expression::Tuple(Box::new(Tuple { expressions }));
            return Ok((None, vec![property("CLUSTER BY", None, Some(value))]));
        }
        if self.match_text_seq(&["FROZEN", "WHERE"]) {
            self.expect_token(TokenType::LParen)?;
            let condition = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok((None, vec![property("FROZEN WHERE", None, Some(condition))]));
        }
        let saved = self.current;
        self.match_token(TokenType::With);
        if self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            let policy = self.parse_iceberg_object_name()?;
            self.expect_token(TokenType::On)?;
            let Expression::Tuple(columns) = self.parse_iceberg_name_list(true)? else {
                return Err(sqlquack_syntax::tokens::errors::Error::internal(
                    "the parser reached a state its checks rule out",
                ));
            };
            return Ok((None, vec![policy_property(policy, columns.expressions)]));
        }
        self.current = saved;
        self.parse_interactive_property()
    }

    /// One property an interactive table allows, with the key of a `<key> = <value>` one.
    fn parse_interactive_property(&mut self) -> Result<(Option<String>, Vec<ObjectProperty>)> {
        self.match_token(TokenType::With);
        if self.match_text_seq(&["TAG"]) {
            return Ok((None, tag_properties(self.parse_iceberg_tags()?)));
        }
        if self.match_text_seq(&["STORAGE", "LIFECYCLE", "POLICY"]) {
            let policy = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            self.expect_token(TokenType::On)?;
            let columns = self.parse_iceberg_name_list(false)?;
            let value = Expression::Tuple(Box::new(Tuple {
                expressions: vec![policy, columns],
            }));
            return Ok((
                None,
                vec![property("STORAGE LIFECYCLE POLICY", None, Some(value))],
            ));
        }
        let key = self.peek().text.to_ascii_uppercase();
        self.expect_external_name()?;
        self.expect_token(TokenType::Eq)?;
        let value = if self.match_token(TokenType::LParen) {
            let mut expressions: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) {
                expressions.push(Expression::Identifier(self.parse_external_name()?));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            Expression::Tuple(Box::new(Tuple { expressions }))
        } else {
            self.parse_iceberg_value()?
        };
        Ok((Some(key.clone()), vec![property(&key, None, Some(value))]))
    }

    /// Checks a value: a literal, a quoted identifier, or a word that is not the key of the next property.
    pub(super) fn check_iceberg_value(&mut self) -> Result<()> {
        self.parse_iceberg_value().map(|_| ())
    }

    /// A value: a literal, a quoted identifier, or a word that is not the key of the next property.
    pub(super) fn parse_iceberg_value(&mut self) -> Result<Expression> {
        let literal = matches!(
            self.peek().token_type,
            TokenType::String | TokenType::Number | TokenType::True | TokenType::False
        );
        if literal && !self.is_at_end() {
            return self.parse_primary();
        }
        if self.check(TokenType::QuotedIdentifier) {
            return Ok(Expression::Identifier(self.parse_external_name()?));
        }
        if self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a value"));
        }
        Ok(Expression::Identifier(self.parse_external_name()?))
    }
}
