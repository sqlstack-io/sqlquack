//! Parser ddl: a Snowflake CREATE TABLE the table model cannot hold, such as one with column or table policies, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_external_table::policy_property;
use crate::parser::constants::SNOWFLAKE_TABLE_PROPERTY_KEYS;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, ComputedColumn, ForeignKeyRef, MaskingPolicyColumnConstraint,
};
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tables::models::{ConstraintType, Tags};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::{DataType, Tuple};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// One constraint: its keywords, its name, the columns it covers or its CHECK condition, what it references, and its property words.
struct Constraint {
    kind: &'static str,
    name: Option<Identifier>,
    operand: Option<Expression>,
    references: Option<(Expression, Option<Expression>)>,
    properties: Vec<String>,
}

impl Parser {
    /// A Snowflake CREATE TABLE (CREATE and TABLE at the given positions) the table parser could not finish, built from its grammar when that holds.
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
        let Ok(mut table) = self.parse_snowflake_create_table(or_replace) else {
            self.current = end;
            return parsed;
        };
        let words: Vec<String> = self.tokens[create_start + 1..table_start]
            .iter()
            .map(|token| token.text.to_ascii_uppercase())
            .collect();
        table.or_alter = words.windows(2).any(|pair| pair == ["OR", "ALTER"]);
        let modifier: Vec<&str> = words
            .iter()
            .map(String::as_str)
            .filter(|word| !matches!(*word, "OR" | "REPLACE" | "ALTER"))
            .collect();
        table.modifier = (!modifier.is_empty()).then(|| modifier.join(" "));
        Ok(Expression::CreateObject(Box::new(table)))
    }

    /// A Snowflake CREATE TABLE from its TABLE word to the end of the statement.
    fn parse_snowflake_create_table(&mut self, or_replace: bool) -> Result<CreateObject> {
        self.expect_token(TokenType::Table)?;
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
        let like = self.match_token(TokenType::Like);
        let template = !like && self.match_text_seq(&["USING", "TEMPLATE"]);
        let columns = !like && !template && self.check(TokenType::LParen);
        let mut untyped = false;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if like {
            operands.push(Expression::Table(Box::new(
                self.parse_iceberg_object_name()?,
            )));
        } else if template {
            operands.push(self.parse_external_template()?);
        } else if columns {
            let (elements, constraints, missing_type) = self.parse_checked_table_elements()?;
            untyped = missing_type;
            operands.push(Expression::Tuple(Box::new(Tuple {
                expressions: elements,
            })));
            properties.extend(constraints);
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) && !self.check(TokenType::As) {
            properties.extend(self.parse_checked_table_property()?);
        }
        let query = !like && !template && self.match_token(TokenType::As);
        if query {
            if !self.check(TokenType::Select) && !self.check(TokenType::With) {
                return Err(self.parse_error("Expected a query after AS"));
            }
            operands.push(self.parse_statement()?);
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
        let source = match (like, template, query) {
            (true, _, _) => "LIKE",
            (_, true, _) => "USING TEMPLATE",
            (_, _, true) => "AS",
            _ => "",
        };
        Ok(CreateObject {
            kind: "TABLE".to_owned(),
            modifier: None,
            or_replace,
            or_alter: false,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        })
    }

    /// `( <column or constraint> [ , ... ] )`: the columns, the out-of-line constraints as properties, and whether a column has no data type.
    fn parse_checked_table_elements(
        &mut self,
    ) -> Result<(Vec<Expression>, Vec<ObjectProperty>, bool)> {
        self.expect_token(TokenType::LParen)?;
        let mut columns: Vec<Expression> = Vec::new();
        let mut constraints: Vec<ObjectProperty> = Vec::new();
        let mut untyped = false;
        loop {
            let constraint = self.check(TokenType::Constraint)
                || self.check(TokenType::Unique)
                || self.check(TokenType::PrimaryKey)
                || self.check(TokenType::ForeignKey)
                || self.check(TokenType::Check);
            if constraint {
                constraints.push(constraint_property(self.parse_checked_constraint(false)?));
            } else {
                let column = self.parse_checked_table_column()?;
                untyped |= column.no_type;
                columns.push(Expression::ColumnDef(Box::new(column)));
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok((columns, constraints, untyped))
    }

    /// Checks `<name> [<type>] [<column option> ...]`, returning whether the type is missing.
    pub(super) fn check_table_column(&mut self) -> Result<bool> {
        self.parse_checked_table_column()
            .map(|column| column.no_type)
    }

    /// `<name> [<type>] [<column option> ...]`; `no_type` marks a missing type.
    fn parse_checked_table_column(&mut self) -> Result<ColumnDef> {
        let name = self.parse_external_name()?;
        let untyped = self.check(TokenType::Comma)
            || self.check(TokenType::RParen)
            || self.check(TokenType::With)
            || self.check(TokenType::Comment)
            || self.check_text_seq(&["MASKING", "POLICY"])
            || self.check_text_seq(&["PROJECTION", "POLICY"])
            || self.check_text_seq(&["TAG"]);
        let mut column = ColumnDef::new("", DataType::Unknown);
        column.name = name;
        column.no_type = untyped;
        if !untyped {
            if self.check(TokenType::String) {
                return Err(self.parse_error("Expected a data type"));
            }
            column.data_type = self.parse_data_type()?;
        }
        while !self.check(TokenType::Comma)
            && !self.check(TokenType::RParen)
            && !self.check(TokenType::Semicolon)
            && !self.is_at_end()
        {
            self.parse_checked_column_option(&mut column)?;
        }
        Ok(column)
    }

    /// One column option on `column`, including an inline constraint.
    fn parse_checked_column_option(&mut self, column: &mut ColumnDef) -> Result<()> {
        if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
            column.nullable = Some(false);
            column.constraint_order.push(ConstraintType::NotNull);
            return Ok(());
        }
        if self.match_token(TokenType::Null) {
            column.nullable = Some(true);
            column.constraint_order.push(ConstraintType::Null);
            return Ok(());
        }
        let constraint = self.check(TokenType::Constraint)
            || self.check(TokenType::Unique)
            || self.check(TokenType::PrimaryKey)
            || self.check(TokenType::ForeignKey)
            || self.check(TokenType::References)
            || self.check(TokenType::Check);
        if constraint {
            let constraint = self.parse_checked_constraint(true)?;
            add_inline_constraint(column, constraint);
            return Ok(());
        }
        if self.match_token(TokenType::Collate) {
            let collation = self.expect_token(TokenType::String)?.text.to_string();
            column
                .constraints
                .push(ColumnConstraint::Collate(Identifier::new(collation)));
            column.constraint_order.push(ConstraintType::Collate);
            return Ok(());
        }
        if self.match_token(TokenType::Comment) {
            column.comment = Some(self.expect_token(TokenType::String)?.text.to_string());
            column.constraint_order.push(ConstraintType::Comment);
            return Ok(());
        }
        if self.match_token(TokenType::Default) {
            column.default = Some(self.parse_unary()?);
            column.constraint_order.push(ConstraintType::Default);
            return Ok(());
        }
        if self.match_token(TokenType::AutoIncrement) || self.match_text_seq(&["IDENTITY"]) {
            column.auto_increment = true;
            column.constraint_order.push(ConstraintType::AutoIncrement);
            return self.parse_checked_identity_options(column);
        }
        if self.match_token(TokenType::As) {
            let expression = self.parse_expression()?;
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
            return Ok(());
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
            return Ok(());
        }
        if self.match_text_seq(&["PROJECTION", "POLICY"]) {
            let policy = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            column
                .options
                .push(Expression::PropertyEQ(Box::new(BinaryOp::new(
                    Expression::Var(Box::new(Var {
                        this: "PROJECTION POLICY".to_owned(),
                    })),
                    policy,
                ))));
            return Ok(());
        }
        if self.match_text_seq(&["TAG"]) {
            let expressions = self.parse_iceberg_tags()?;
            column
                .constraints
                .push(ColumnConstraint::Tags(Tags { expressions }));
            column.constraint_order.push(ConstraintType::Tags);
            return Ok(());
        }
        Err(self.parse_error("Expected a column option"))
    }

    /// `[ ( <start> , <step> ) | START <start> INCREMENT <step> ] [ ORDER | NOORDER ]` on `column`.
    fn parse_checked_identity_options(&mut self, column: &mut ColumnDef) -> Result<()> {
        if self.match_token(TokenType::LParen) {
            column.auto_increment_start = Some(Box::new(self.parse_checked_identity_number()?));
            self.expect_token(TokenType::Comma)?;
            column.auto_increment_increment = Some(Box::new(self.parse_checked_identity_number()?));
            self.expect_token(TokenType::RParen)?;
        } else if self.match_text_seq(&["START"]) {
            column.auto_increment_start = Some(Box::new(self.parse_checked_identity_number()?));
            if !self.match_text_seq(&["INCREMENT"]) {
                return Err(self.parse_error("Expected INCREMENT"));
            }
            column.auto_increment_increment = Some(Box::new(self.parse_checked_identity_number()?));
        }
        if self.match_token(TokenType::Order) {
            column.auto_increment_order = Some(true);
        } else if self.match_text_seq(&["NOORDER"]) {
            column.auto_increment_order = Some(false);
        }
        Ok(())
    }

    /// A number, possibly negative.
    fn parse_checked_identity_number(&mut self) -> Result<Expression> {
        if self.check(TokenType::Dash) {
            if !self.check_next(TokenType::Number) {
                self.skip();
                return Err(self.parse_error("Expected a number"));
            }
            return self.parse_unary();
        }
        if !self.check(TokenType::Number) {
            return Err(self.parse_error("Expected a number"));
        }
        self.parse_primary()
    }

    /// `[CONSTRAINT <name>]` and a constraint with its properties; `inline` takes no column list.
    fn parse_checked_constraint(&mut self, inline: bool) -> Result<Constraint> {
        let name = match self.match_token(TokenType::Constraint) {
            true => Some(self.parse_external_name()?),
            false => None,
        };
        let (kind, references) = if self.match_token(TokenType::Unique) {
            ("UNIQUE", false)
        } else if self.match_keywords(&[TokenType::PrimaryKey, TokenType::Key]) {
            ("PRIMARY KEY", false)
        } else if self.match_keywords(&[TokenType::ForeignKey, TokenType::Key])
            || inline && self.check(TokenType::References)
        {
            ("FOREIGN KEY", true)
        } else if self.match_token(TokenType::Check) {
            self.expect_token(TokenType::LParen)?;
            let condition = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Constraint {
                kind: "CHECK",
                name,
                operand: Some(condition),
                references: None,
                properties: self.parse_checked_constraint_properties(false)?,
            });
        } else {
            return Err(self.parse_error("Expected UNIQUE, PRIMARY KEY, FOREIGN KEY or CHECK"));
        };
        let operand = match inline {
            true => None,
            false => Some(self.parse_iceberg_name_list(false)?),
        };
        let references = match references {
            true => {
                self.expect_token(TokenType::References)?;
                let table = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
                let columns = match self.check(TokenType::LParen) {
                    true => Some(self.parse_iceberg_name_list(false)?),
                    false => None,
                };
                Some((table, columns))
            }
            false => None,
        };
        let properties = self.parse_checked_constraint_properties(references.is_some())?;
        Ok(Constraint {
            kind,
            name,
            operand,
            references,
            properties,
        })
    }

    /// The property words after a constraint; MATCH and ON UPDATE or DELETE follow a foreign key, VALIDATE follows ENABLE or DISABLE (errno 1003).
    fn parse_checked_constraint_properties(&mut self, references: bool) -> Result<Vec<String>> {
        let mut words: Vec<String> = Vec::new();
        loop {
            let start = self.current;
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
                return Ok(words);
            }
            let phrase: Vec<String> = self.tokens[start..self.current]
                .iter()
                .map(|token| token.text.to_ascii_uppercase())
                .collect();
            words.push(phrase.join(" "));
        }
    }

    /// One property after the columns, the LIKE source or the name; a TAG or CONTACT list gives one per entry.
    pub(super) fn parse_checked_table_property(&mut self) -> Result<Vec<ObjectProperty>> {
        if self.match_keywords(&[TokenType::Cluster, TokenType::By]) {
            self.expect_token(TokenType::LParen)?;
            let expressions = self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            let value = Expression::Tuple(Box::new(Tuple { expressions }));
            return Ok(vec![property("CLUSTER BY", None, Some(value))]);
        }
        if self.check(TokenType::Copy) && !self.check_next(TokenType::Eq) {
            self.skip();
            if self.match_text_seq(&["GRANTS"]) {
                return Ok(vec![property("COPY GRANTS", None, None)]);
            }
            if self.match_text_seq(&["TAGS"]) {
                return Ok(vec![property("COPY TAGS", None, None)]);
            }
            return Err(self.parse_error("Expected GRANTS or TAGS after COPY"));
        }
        if self.match_token(TokenType::Copy) {
            self.expect_token(TokenType::Eq)?;
            self.expect_token(TokenType::LParen)?;
            let mut expressions: Vec<Expression> = Vec::new();
            while !self.check(TokenType::RParen) {
                let word = self.peek().text.to_ascii_uppercase();
                if !self.match_text_seq(&["COMMENTS"])
                    && !self.match_text_seq(&["TAGS"])
                    && !self.match_text_seq(&["GRANTS"])
                {
                    return Err(self.parse_error("Expected COMMENTS, TAGS or GRANTS"));
                }
                expressions.push(Expression::Var(Box::new(Var { this: word })));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            let value = Expression::Tuple(Box::new(Tuple { expressions }));
            return Ok(vec![property("COPY", None, Some(value))]);
        }
        let key = self.peek().text.to_ascii_uppercase();
        if self.check_next(TokenType::Eq) && SNOWFLAKE_TABLE_PROPERTY_KEYS.contains(&key.as_str()) {
            self.skip();
            self.skip();
            let value = self.parse_iceberg_value()?;
            return Ok(vec![property(&key, None, Some(value))]);
        }
        let with = self.match_token(TokenType::With);
        let key = self.peek().text.to_ascii_uppercase();
        if with && self.check_next(TokenType::Eq) && key == "COMMENT" {
            self.skip();
            self.skip();
            let value = self.parse_iceberg_value()?;
            return Ok(vec![property(&key, None, Some(value))]);
        }
        if with && self.match_text_seq(&["CONTACT"]) {
            return self.parse_checked_table_contacts();
        }
        if self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            let policy = self.parse_iceberg_object_name()?;
            self.expect_token(TokenType::On)?;
            let Expression::Tuple(columns) = self.parse_iceberg_name_list(true)? else {
                return Err(sqlquack_syntax::tokens::errors::Error::internal(
                    "the parser reached a state its checks rule out",
                ));
            };
            return Ok(vec![policy_property(policy, columns.expressions)]);
        }
        if self.match_text_seq(&["AGGREGATION", "POLICY"]) {
            return self.parse_keyed_policy("AGGREGATION POLICY", &["ENTITY", "KEY"]);
        }
        if self.match_keywords(&[TokenType::Join]) && self.match_text_seq(&["POLICY"]) {
            return self.parse_keyed_policy("JOIN POLICY", &["ALLOWED", "JOIN", "KEYS"]);
        }
        if self.match_text_seq(&["STORAGE", "LIFECYCLE", "POLICY"]) {
            let policy = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            self.expect_token(TokenType::On)?;
            let columns = self.parse_iceberg_name_list(false)?;
            let value = Expression::Tuple(Box::new(Tuple {
                expressions: vec![policy, columns],
            }));
            return Ok(vec![property(
                "STORAGE LIFECYCLE POLICY",
                None,
                Some(value),
            )]);
        }
        if self.match_text_seq(&["TAG"]) {
            return Ok(tag_properties(self.parse_iceberg_tags()?));
        }
        Err(self.parse_error("Expected a table property"))
    }

    /// `<policy> [ <keys words> ( <names> ) ]` after an aggregation or join policy's keywords.
    fn parse_keyed_policy(&mut self, kind: &str, keys: &[&str]) -> Result<Vec<ObjectProperty>> {
        let mut expressions = vec![Expression::Table(Box::new(
            self.parse_iceberg_object_name()?,
        ))];
        if self.match_text_seq(keys) {
            expressions.push(self.parse_iceberg_name_list(false)?);
        }
        let value = Expression::Tuple(Box::new(Tuple { expressions }));
        Ok(vec![property(kind, None, Some(value))])
    }

    /// `( <purpose> = <contact> [ , ... ] )`, one CONTACT property per purpose.
    fn parse_checked_table_contacts(&mut self) -> Result<Vec<ObjectProperty>> {
        self.expect_token(TokenType::LParen)?;
        let mut contacts: Vec<ObjectProperty> = Vec::new();
        loop {
            let purpose = self.parse_external_name()?;
            self.expect_token(TokenType::Eq)?;
            let contact = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
            contacts.push(property("CONTACT", Some(purpose), Some(contact)));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(contacts)
    }
}

/// One TAG property per `<tag> = <value>`.
pub(super) fn tag_properties(tags: Vec<Expression>) -> Vec<ObjectProperty> {
    tags.into_iter()
        .map(|tag| property("TAG", None, Some(tag)))
        .collect()
}

/// An out-of-line constraint as a property: its keywords, its name, and a tuple of its columns or condition, its reference and its property words.
fn constraint_property(constraint: Constraint) -> ObjectProperty {
    let mut expressions: Vec<Expression> = constraint.operand.into_iter().collect();
    if let Some((table, columns)) = constraint.references {
        expressions.push(table);
        expressions.extend(columns);
    }
    expressions.extend(
        constraint
            .properties
            .into_iter()
            .map(|word| Expression::Var(Box::new(Var { this: word }))),
    );
    property(
        constraint.kind,
        constraint.name,
        Some(Expression::Tuple(Box::new(Tuple { expressions }))),
    )
}

/// An inline constraint on `column`, with its property words recorded against it.
fn add_inline_constraint(column: &mut ColumnDef, constraint: Constraint) {
    let name = constraint.name.map(|name| name.name);
    let kind = match constraint.kind {
        "UNIQUE" => {
            column.unique = true;
            column.unique_constraint_name = name;
            ConstraintType::Unique
        }
        "PRIMARY KEY" => {
            column.primary_key = true;
            column.primary_key_constraint_name = name;
            ConstraintType::PrimaryKey
        }
        "CHECK" => {
            column.check_constraint_name = name;
            let condition = constraint
                .operand
                .expect("a CHECK constraint has a condition");
            column.constraints.push(ColumnConstraint::Check(condition));
            ConstraintType::Check
        }
        _ => {
            let (table, columns) = constraint
                .references
                .expect("a foreign key references a table");
            let Expression::Table(table) = table else {
                unreachable!("a reference names a table");
            };
            let columns = match columns {
                Some(Expression::Tuple(tuple)) => tuple
                    .expressions
                    .into_iter()
                    .filter_map(|column| match column {
                        Expression::Table(table) => Some(table.name),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            column
                .constraints
                .push(ColumnConstraint::References(Box::new(ForeignKeyRef {
                    table: *table,
                    columns,
                    on_delete: None,
                    on_update: None,
                    on_update_first: false,
                    match_type: None,
                    match_after_actions: false,
                    constraint_name: name,
                    deferrable: None,
                    has_foreign_key_keywords: false,
                })));
            ConstraintType::References
        }
    };
    column.constraint_order.push(kind);
    let index = column.constraint_order.len() - 1;
    column
        .constraint_properties
        .extend(constraint.properties.into_iter().map(|word| (index, word)));
}
