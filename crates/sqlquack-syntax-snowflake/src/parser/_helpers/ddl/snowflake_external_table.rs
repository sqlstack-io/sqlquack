//! Parser ddl: Snowflake CREATE EXTERNAL TABLE, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::SNOWFLAKE_EXTERNAL_TABLE_REQUIRED;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, ComputedColumn, ForeignKeyRef,
};
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tables::models::ConstraintType;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::{DataType, Tuple};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE [OR REPLACE] EXTERNAL TABLE; None for any other CREATE, outside the faithful Snowflake modes, and for a bare USING TEMPLATE ( <query> ) or '<text>', which the table parser keeps whole.
    pub(crate) fn parse_snowflake_create_external_table(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if or_alter
            || !self.applies_snowflake_context_rules()
            || !self.check_text_seq(&["EXTERNAL", "TABLE"])
        {
            return Ok(None);
        }
        let start = self.current;
        self.match_text_seq(&["EXTERNAL", "TABLE"]);
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        let template = self.match_text_seq(&["USING", "TEMPLATE"]);
        let query_or_text =
            template && (self.check(TokenType::LParen) || self.check(TokenType::String));
        let operand = match template {
            true => self.parse_external_template()?,
            false => Expression::Tuple(Box::new(Tuple {
                expressions: self.parse_external_columns()?,
            })),
        };
        let after_template = self.current;
        let properties = self.parse_external_properties(!template)?;
        if query_or_text && self.current == after_template {
            self.current = start;
            return Ok(None);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "EXTERNAL TABLE".to_owned(),
            modifier: None,
            or_replace,
            or_alter: false,
            if_not_exists,
            name,
            source: match template {
                true => "USING TEMPLATE".to_owned(),
                false => String::new(),
            },
            operands: vec![operand],
            properties,
        }))))
    }

    /// `( <query> )`, `$<variable>` or a string after USING TEMPLATE.
    pub(super) fn parse_external_template(&mut self) -> Result<Expression> {
        if self.check(TokenType::String) {
            return self.parse_primary();
        }
        let variable = matches!(
            self.peek().token_type,
            TokenType::Var | TokenType::Identifier
        ) && self.peek().text.len() > 1
            && self.peek().text.starts_with('$');
        if variable {
            let text = self.peek().text.to_string();
            self.skip();
            return Ok(Expression::Identifier(Identifier::new(text)));
        }
        if self.check(TokenType::Dollar) {
            let variable = self.peek_nth(1).filter(|token| {
                token.span.start == self.peek().span.end
                    && (matches!(token.token_type, TokenType::Var | TokenType::Identifier)
                        || token.token_type.is_keyword())
            });
            let Some(variable) = variable else {
                return Err(self.parse_error("Expected a variable after $"));
            };
            let text = format!("${}", variable.text);
            self.skip();
            self.skip();
            return Ok(Expression::Identifier(Identifier::new(text)));
        }
        self.expect_token(TokenType::LParen)?;
        if !self.check(TokenType::Select) && !self.check(TokenType::With) {
            return Err(self.parse_error("Expected a query in USING TEMPLATE ( )"));
        }
        let query = self.parse_statement()?;
        self.expect_token(TokenType::RParen)?;
        Ok(query)
    }

    /// `( <name> [<type>] AS <expr> [<constraint>] [ , ... ] )`, and the bare `<name>`, `<name> REFERENCES ...` and `REFERENCES ...` Snowflake parses before refusing.
    fn parse_external_columns(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::LParen)?;
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let bare_references = self.check_text_seq(&["REFERENCES"])
                && !self.check_next(TokenType::Comma)
                && !self.check_next(TokenType::RParen);
            if bare_references {
                let mut column = ColumnDef::new("", DataType::Unknown);
                column.no_type = true;
                column = self.parse_external_constraint(column)?;
                let constraint = column.constraints.pop();
                columns.extend(
                    constraint.map(|constraint| Expression::ColumnConstraint(Box::new(constraint))),
                );
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            }
            let name = self.parse_external_name()?;
            let mut column = ColumnDef::new("", DataType::Unknown);
            column.name = name;
            if self.check(TokenType::Comma)
                || self.check(TokenType::RParen)
                || self.check_text_seq(&["REFERENCES"])
            {
                column.no_type = true;
                column = self.parse_external_constraint(column)?;
                columns.push(Expression::ColumnDef(Box::new(column)));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            }
            match self.check(TokenType::As) {
                true => column.no_type = true,
                false => {
                    if self.check(TokenType::String) {
                        return Err(self.parse_error("Expected a data type"));
                    }
                    column.data_type = self.parse_data_type()?;
                }
            }
            self.expect_token(TokenType::As)?;
            let expression_start = self.current;
            let expression = self.parse_expression()?;
            let negated_constraint = self.tokens
                [expression_start..(self.current + 1).min(self.tokens.len())]
                .windows(2)
                .any(|pair| {
                    pair[0].token_type == TokenType::Not && pair[1].token_type == TokenType::Unique
                });
            if negated_constraint {
                return Err(self.parse_error("Expected a column expression before the constraint"));
            }
            column
                .constraints
                .push(ColumnConstraint::ComputedColumn(ComputedColumn {
                    expression: Box::new(expression),
                    persisted: false,
                    not_null: false,
                    persistence_kind: None,
                    data_type: None,
                }));
            column.constraint_order.push(ConstraintType::ComputedColumn);
            column = self.parse_external_constraint(column)?;
            columns.push(Expression::ColumnDef(Box::new(column)));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(columns)
    }

    /// An optional inline `[CONSTRAINT <name>] UNIQUE | PRIMARY KEY | [FOREIGN KEY] REFERENCES <table> [( <columns> )]` on `column`.
    fn parse_external_constraint(&mut self, mut column: ColumnDef) -> Result<ColumnDef> {
        let named = self.match_text_seq(&["CONSTRAINT"]);
        let constraint_name = match named {
            true => Some(self.parse_external_name()?.name),
            false => None,
        };
        if self.match_text_seq(&["UNIQUE"]) {
            column.unique = true;
            column.unique_constraint_name = constraint_name;
            column.constraint_order.push(ConstraintType::Unique);
            return Ok(column);
        }
        if self.match_text_seq(&["PRIMARY", "KEY"]) {
            column.primary_key = true;
            column.primary_key_constraint_name = constraint_name;
            column.constraint_order.push(ConstraintType::PrimaryKey);
            return Ok(column);
        }
        let foreign = self.match_text_seq(&["FOREIGN", "KEY"]);
        if !self.match_text_seq(&["REFERENCES"]) {
            return match named || foreign {
                true => Err(self.parse_error("Expected a constraint")),
                false => Ok(column),
            };
        }
        let table = self.parse_object_name()?;
        let mut columns: Vec<Identifier> = Vec::new();
        if self.match_token(TokenType::LParen) {
            loop {
                columns.push(self.parse_external_name()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        column
            .constraints
            .push(ColumnConstraint::References(Box::new(ForeignKeyRef {
                table,
                columns,
                on_delete: None,
                on_update: None,
                on_update_first: false,
                match_type: None,
                match_after_actions: false,
                constraint_name,
                deferrable: None,
                has_foreign_key_keywords: foreign,
            })));
        column.constraint_order.push(ConstraintType::References);
        Ok(column)
    }

    /// Checks one name: an identifier, or a word Snowflake does not reserve.
    pub(super) fn expect_external_name(&mut self) -> Result<()> {
        self.parse_external_name().map(|_| ())
    }

    /// One name: an identifier, or a word Snowflake does not reserve.
    pub(super) fn parse_external_name(&mut self) -> Result<Identifier> {
        let token_type = self.peek().token_type;
        let name_like = matches!(
            token_type,
            TokenType::Identifier | TokenType::QuotedIdentifier | TokenType::Var
        ) || token_type.is_keyword();
        if self.is_at_end() || !name_like || self.is_snowflake_reserved_keyword() {
            return Err(self.parse_error("Expected a name"));
        }
        let text = self.peek().text.to_string();
        self.skip();
        Ok(match token_type == TokenType::QuotedIdentifier {
            true => Identifier::quoted(text),
            false => Identifier::new(text),
        })
    }

    /// The properties after the columns or template; the column form needs LOCATION and FILE_FORMAT.
    fn parse_external_properties(&mut self, columns: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            properties.extend(self.parse_external_property()?);
        }
        if columns && lacks_property(&properties, SNOWFLAKE_EXTERNAL_TABLE_REQUIRED) {
            return Err(self.parse_error("Expected LOCATION and FILE_FORMAT"));
        }
        Ok(properties)
    }

    /// One property, or one per tag of a TAG list.
    fn parse_external_property(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.match_text_seq(&["COPY", "GRANTS"]) {
            return Ok(vec![property("COPY GRANTS", None, None)]);
        }
        if self.match_keywords(&[TokenType::Partition, TokenType::By]) {
            self.expect_token(TokenType::LParen)?;
            let expressions = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            let value = Expression::Tuple(Box::new(Tuple { expressions }));
            return Ok(vec![property("PARTITION BY", None, Some(value))]);
        }
        if self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            let policy = self.parse_object_name()?;
            self.expect_token(TokenType::On)?;
            self.expect_token(TokenType::LParen)?;
            let mut columns: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) {
                columns.push(Expression::Identifier(self.parse_external_name()?));
                self.match_token(TokenType::Comma);
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(vec![policy_property(policy, columns)]);
        }
        if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            return self.parse_external_tags();
        }
        let key = self.parse_external_name()?;
        let kind = key.name.to_ascii_uppercase();
        self.expect_token(TokenType::Eq)?;
        let value = match kind == SNOWFLAKE_EXTERNAL_TABLE_REQUIRED[0] {
            true => self.parse_external_stage()?,
            false => self.parse_external_value()?,
        };
        Ok(vec![property(&kind, None, Some(value))])
    }

    /// `( <tag> = <value> [ , ... ] )`, one TAG property per tag.
    pub(super) fn parse_external_tags(&mut self) -> Result<Vec<ObjectProperty>> {
        self.expect_token(TokenType::LParen)?;
        let mut tags: Vec<ObjectProperty> = Vec::new();
        loop {
            let tag = self.parse_object_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = match self.check(TokenType::String) {
                true => self.parse_primary()?,
                false => self.parse_external_word()?,
            };
            tags.push(ObjectProperty {
                kind: "TAG".to_owned(),
                name: None,
                value: Some(Expression::PropertyEQ(Box::new(BinaryOp::new(
                    Expression::Table(Box::new(tag)),
                    value,
                )))),
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(tags)
    }

    /// A stage location: one unbroken run of tokens that starts with @, holds no string and quotes no blank.
    fn parse_external_stage(&mut self) -> Result<Expression> {
        let at = self.check(TokenType::DAt) || self.peek().text.starts_with('@');
        if self.is_at_end() || !at {
            return Err(self.parse_error("Expected a stage location"));
        }
        let blank_quoted = self.check(TokenType::DAt)
            && self.peek_nth(1).is_some_and(|token| {
                token.token_type == TokenType::QuotedIdentifier
                    && token.text.contains(char::is_whitespace)
            });
        if blank_quoted {
            return Err(self.parse_error("Expected a stage name after @"));
        }
        let start = self.current;
        loop {
            self.skip();
            let adjacent = !self.is_at_end()
                && self.tokens[self.current - 1].span.end == self.peek().span.start;
            if !adjacent {
                let path = self.tokens_to_sql(start, self.current);
                return Ok(Expression::Literal(Box::new(Literal::String(path))));
            }
            if self.check(TokenType::String) {
                return Err(self.parse_error("A stage name part cannot be a string literal"));
            }
        }
    }

    /// A value: a literal, a word, or a parenthesized list of values and `<key> = <value>` pairs.
    fn parse_external_value(&mut self) -> Result<Expression> {
        if !self.match_token(TokenType::LParen) {
            return self.parse_external_scalar();
        }
        let mut expressions: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) {
            if self.check_next(TokenType::Eq) {
                let key = self.parse_external_name()?;
                self.skip();
                let value = self.parse_external_value()?;
                expressions.push(Expression::PropertyEQ(Box::new(BinaryOp::new(
                    Expression::Identifier(key),
                    value,
                ))));
            } else {
                expressions.push(self.parse_external_scalar()?);
            }
            self.match_token(TokenType::Comma);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }

    /// A string, number or boolean literal, or a bare word.
    pub(super) fn parse_external_scalar(&mut self) -> Result<Expression> {
        let literal = matches!(
            self.peek().token_type,
            TokenType::String | TokenType::Number | TokenType::True | TokenType::False
        );
        match literal && !self.is_at_end() {
            true => self.parse_primary(),
            false => self.parse_external_word(),
        }
    }

    /// A bare word value, which cannot be the key of the next property.
    fn parse_external_word(&mut self) -> Result<Expression> {
        if self.check(TokenType::QuotedIdentifier) || self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a value"));
        }
        Ok(Expression::Identifier(self.parse_external_name()?))
    }
}

/// A `ROW ACCESS POLICY <policy> ON ( <columns> )` property: the policy and its column tuple.
pub(super) fn policy_property(policy: TableRef, columns: Vec<Expression>) -> ObjectProperty {
    property(
        "ROW ACCESS POLICY",
        None,
        Some(Expression::Tuple(Box::new(Tuple {
            expressions: vec![
                Expression::Table(Box::new(policy)),
                Expression::Tuple(Box::new(Tuple {
                    expressions: columns,
                })),
            ],
        }))),
    )
}

/// Whether `properties` lacks one of the unnamed `required` properties.
pub(super) fn lacks_property(properties: &[ObjectProperty], required: &[&str]) -> bool {
    required
        .iter()
        .any(|kind| !has_unnamed_property(properties, kind))
}

/// Whether `properties` has an unnamed property of `kind`.
fn has_unnamed_property(properties: &[ObjectProperty], kind: &str) -> bool {
    properties
        .iter()
        .any(|property| property.name.is_none() && property.kind == kind)
}
