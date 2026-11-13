//! `alter_statement` (googlesql.tm) as `AlterObject` with clause operands, and shared column parts.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, ComputedColumn, ForeignKeyRef, GeneratedAsIdentity,
    ReferentialAction,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::AlterObject;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{table, tuple, var};
use crate::parse::constants::{
    ALTER_ACTION_WORDS, ALTERABLE_KINDS, GENERIC_ENTITY_TYPES, INDEX_KINDS, SCHEMA_OBJECT_KINDS,
    SUB_ENTITY_TYPES,
};
use crate::scan::models::TokenKind;

/// A clause as the tree keeps it: its keywords as a word, then its operands.
pub(crate) fn clause(keywords: &str, operands: Vec<Expression>) -> Expression {
    let mut parts: Vec<Expression> = vec![var(keywords)];
    parts.extend(operands);
    tuple(parts)
}

/// A name as an identifier operand.
pub(crate) fn named(name: Identifier) -> Expression {
    Expression::Identifier(name)
}

/// A path as an identifier operand, its parts joined with `.`.
pub(crate) fn path_operand(name: &TableRef) -> Expression {
    let mut parts: Vec<String> = Vec::new();
    for part in [&name.catalog, &name.schema].into_iter().flatten() {
        parts.push(part.name.clone());
    }
    parts.push(name.name.name.clone());
    Expression::Identifier(Identifier::new(parts.join(".")))
}

/// An `ALTER` of `kind` named `name` whose actions are `operands`.
pub(crate) fn altered(
    kind: &str,
    name: TableRef,
    if_exists: bool,
    operands: Vec<Expression>,
) -> Expression {
    Expression::AlterObject(Box::new(AlterObject {
        kind: kind.to_owned(),
        name: Some(name),
        arguments: Vec::new(),
        if_exists,
        action: String::new(),
        operands,
        properties: Vec::new(),
    }))
}

impl Parser<'_> {
    /// `ALTER` of a table, schema object, generic entity, row access or data policy, with actions.
    pub(crate) fn alter_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("ALTER")?;
        let start: usize = self.offset();
        if self.kw("TABLE") {
            self.at += 1;
            if self.kw("FUNCTION") {
                return Err(self.error_with("ALTER TABLE FUNCTION is not supported", start, true));
            }
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: TableRef = self.table_reference()?;
            let actions: Vec<Expression> = self.alter_actions()?;
            return Ok(altered("TABLE", name, if_exists, actions));
        }
        if self.kw("ROW") && self.kw_at(1, "ACCESS") && self.kw_at(2, "POLICY") {
            self.at += 3;
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let policy: Identifier = self.ident()?;
            self.expect_kw("ON")?;
            let on: TableRef = self.table_path()?;
            let mut actions: Vec<Expression> = vec![clause("POLICY", vec![named(policy)])];
            actions.extend(self.comma_list(|parser| parser.row_access_policy_action())?);
            return Ok(altered("ROW ACCESS POLICY", on, if_exists, actions));
        }
        if self.eat_kws(&["ALL", "ROW", "ACCESS", "POLICIES", "ON"]) {
            let on: TableRef = self.table_path()?;
            let action: Expression = self.row_access_policy_action()?;
            return Ok(altered("ALL ROW ACCESS POLICIES", on, false, vec![action]));
        }
        if self.eat_kw("DATA_POLICY") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: TableRef = self.table_path()?;
            let actions: Vec<Expression> = self.comma_list(|parser| {
                parser.expect_kw("SET")?;
                match parser.eat_kw("CONDITION") {
                    true => {
                        parser.expect_char('(')?;
                        let condition: Expression = parser.expression()?;
                        parser.expect_char(')')?;
                        Ok(clause("SET CONDITION", vec![condition]))
                    }
                    false => {
                        parser.expect_kw("OPTIONS")?;
                        Ok(clause("SET OPTIONS", parser.options_list()?))
                    }
                }
            })?;
            return Ok(altered("DATA_POLICY", name, if_exists, actions));
        }
        if let Some(kind) = self.schema_object_kind() {
            if !ALTERABLE_KINDS.contains(&kind) {
                return Err(self.error_with(
                    &format!("ALTER {kind} is not supported"),
                    start,
                    true,
                ));
            }
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: TableRef = self.table_path()?;
            let mut actions: Vec<Expression> = Vec::new();
            if self.kw("ON") {
                if !INDEX_KINDS.contains(&kind) {
                    return Err(self.error_with(
                        &format!("ALTER {kind} does not support the ON clause"),
                        self.offset(),
                        true,
                    ));
                }
                self.at += 1;
                actions.push(clause("ON", vec![path_operand(&self.table_path()?)]));
            }
            actions.extend(self.alter_actions()?);
            return Ok(altered(kind, name, if_exists, actions));
        }
        let Some(entity) = self.generic_entity_type() else {
            return Err(self.error());
        };
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let name: TableRef = match self.at_alter_action() {
            true => table(Vec::new()),
            false => self.table_path()?,
        };
        let actions: Vec<Expression> = self.alter_actions()?;
        Ok(altered(&entity, name, if_exists, actions))
    }

    /// `schema_object_kind`, its longest spelling at the cursor, consumed.
    pub(crate) fn schema_object_kind(&mut self) -> Option<&'static str> {
        let (words, kind) = SCHEMA_OBJECT_KINDS
            .iter()
            .find(|(words, _)| self.kws_ahead(words))?;
        self.at += words.len();
        Some(kind)
    }

    /// Whether `words` follow the cursor, in order.
    fn kws_ahead(&self, words: &[&str]) -> bool {
        for (offset, word) in words.iter().enumerate() {
            if !self.kw_at(offset, word) {
                return false;
            }
        }
        true
    }

    /// `generic_entity_type`: one of the entity types BigQuery supports, consumed.
    pub(crate) fn generic_entity_type(&mut self) -> Option<String> {
        let word: String = self.word_at(0)?;
        if !GENERIC_ENTITY_TYPES.contains(&word.as_str()) {
            return None;
        }
        self.at += 1;
        Some(word)
    }

    /// Whether an `alter_action` begins here rather than an entity's name.
    fn at_alter_action(&self) -> bool {
        self.kw_in(0, &ALTER_ACTION_WORDS) && !self.char_at(1, '.') && !self.ends_at(1)
    }

    /// `path_expression` as a table reference.
    pub(crate) fn table_path(&mut self) -> Parsing<TableRef> {
        Ok(table(self.multipart_identifier()?))
    }

    /// `alter_action_list`.
    fn alter_actions(&mut self) -> Parsing<Vec<Expression>> {
        self.comma_list(|parser| parser.alter_action())
    }

    /// `alter_action`.
    fn alter_action(&mut self) -> Parsing<Expression> {
        let word: String = self.word_at(0).unwrap_or_default();
        match word.as_str() {
            "SET" => self.set_action(),
            "ADD" => self.add_action(),
            "DROP" => self.drop_action(),
            "ALTER" => self.alter_sub_action(),
            "RENAME" => {
                self.at += 1;
                if self.eat_kw("TO") {
                    return Ok(clause("RENAME TO", vec![path_operand(&self.table_path()?)]));
                }
                self.expect_kw("COLUMN")?;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                let from: Identifier = self.ident()?;
                self.expect_kw("TO")?;
                let to: Identifier = self.ident()?;
                Ok(clause(
                    &exists("RENAME COLUMN", if_exists),
                    vec![named(from), named(to)],
                ))
            }
            "REPLACE" => {
                self.at += 1;
                self.expect_kws(&["ROW", "DELETION", "POLICY"])?;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                let policy: Expression = self.parenthesized_expression()?;
                Ok(clause(
                    &exists("REPLACE ROW DELETION POLICY", if_exists),
                    vec![policy],
                ))
            }
            "REBUILD" => {
                self.at += 1;
                Ok(clause("REBUILD", Vec::new()))
            }
            _ => Err(self.error()),
        }
    }

    /// The `SET` actions: options, a body, a default collation.
    fn set_action(&mut self) -> Parsing<Expression> {
        self.expect_kw("SET")?;
        if self.eat_kw("OPTIONS") {
            return Ok(clause("SET OPTIONS", self.options_list()?));
        }
        if self.eat_kw("AS") {
            let body: Expression = match self.eat_kw("JSON") {
                true => clause("JSON", vec![self.string_literal()?]),
                false => self.string_literal()?,
            };
            return Ok(clause("SET AS", vec![body]));
        }
        self.expect_kw("DEFAULT")?;
        self.expect_kw("COLLATE")?;
        Ok(clause(
            "SET DEFAULT COLLATE",
            vec![self.string_or_parameter()?],
        ))
    }

    /// The `ADD` actions: a constraint, a column, a row deletion policy, or a sub-entity.
    fn add_action(&mut self) -> Parsing<Expression> {
        self.expect_kw("ADD")?;
        if self.kw_in(0, &["CHECK", "FOREIGN"]) {
            return Ok(clause("ADD", vec![self.table_constraint_spec()?]));
        }
        if self.kw("PRIMARY") {
            return Ok(clause("ADD", vec![self.primary_key_spec()?]));
        }
        if self.eat_kw("CONSTRAINT") {
            let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
            let name: Identifier = self.ident()?;
            let constraint: Expression = match self.kw("PRIMARY") {
                true => self.primary_key_spec()?,
                false => self.table_constraint_spec()?,
            };
            return Ok(clause(
                &not_exists("ADD CONSTRAINT", if_not_exists),
                vec![named(name), constraint],
            ));
        }
        if self.eat_kw("COLUMN") {
            let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
            let column: ColumnDef = self.table_column_definition()?;
            let mut operands: Vec<Expression> = vec![Expression::ColumnDef(Box::new(column))];
            if self.kw_in(0, &["PRECEDING", "FOLLOWING"]) {
                let position: String = self.word_at(0).unwrap_or_default();
                self.at += 1;
                operands.push(clause(&position, vec![named(self.ident()?)]));
            }
            if self.eat_kws(&["FILL", "USING"]) {
                operands.push(clause("FILL USING", vec![self.expression()?]));
            }
            return Ok(clause(&not_exists("ADD COLUMN", if_not_exists), operands));
        }
        if self.eat_kws(&["STORED", "COLUMN"]) {
            let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
            return Ok(clause(
                &not_exists("ADD STORED COLUMN", if_not_exists),
                vec![named(self.ident()?)],
            ));
        }
        if self.eat_kws(&["ROW", "DELETION", "POLICY"]) {
            let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
            let policy: Expression = self.parenthesized_expression()?;
            return Ok(clause(
                &not_exists("ADD ROW DELETION POLICY", if_not_exists),
                vec![policy],
            ));
        }
        let sub: String = self.sub_entity_type()?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let mut operands: Vec<Expression> = vec![named(self.ident()?)];
        if self.kw("OPTIONS") {
            self.at += 1;
            operands.push(clause("OPTIONS", self.options_list()?));
        }
        Ok(clause(
            &not_exists(&format!("ADD {sub}"), if_not_exists),
            operands,
        ))
    }

    /// The `DROP` actions: a constraint, the primary key, a column, a policy, or a sub-entity.
    fn drop_action(&mut self) -> Parsing<Expression> {
        self.expect_kw("DROP")?;
        let keywords: &str = if self.eat_kw("CONSTRAINT") {
            "DROP CONSTRAINT"
        } else if self.eat_kws(&["PRIMARY", "KEY"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            return Ok(clause(&exists("DROP PRIMARY KEY", if_exists), Vec::new()));
        } else if self.eat_kws(&["ROW", "DELETION", "POLICY"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            return Ok(clause(
                &exists("DROP ROW DELETION POLICY", if_exists),
                Vec::new(),
            ));
        } else if self.eat_kws(&["STORED", "COLUMN"]) {
            "DROP STORED COLUMN"
        } else if self.eat_kw("COLUMN") {
            "DROP COLUMN"
        } else {
            let sub: String = self.sub_entity_type()?;
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.ident()?;
            return Ok(clause(
                &exists(&format!("DROP {sub}"), if_exists),
                vec![named(name)],
            ));
        };
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let name: Identifier = self.ident()?;
        Ok(clause(&exists(keywords, if_exists), vec![named(name)]))
    }

    /// The `ALTER` actions: of a constraint, of a column, or of a sub-entity.
    fn alter_sub_action(&mut self) -> Parsing<Expression> {
        self.expect_kw("ALTER")?;
        if self.eat_kw("CONSTRAINT") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.ident()?;
            let action: Expression = if self.eat_kw("ENFORCED") {
                var("ENFORCED")
            } else if self.eat_kws(&["NOT", "ENFORCED"]) {
                var("NOT ENFORCED")
            } else {
                self.expect_kws(&["SET", "OPTIONS"])?;
                clause("SET OPTIONS", self.options_list()?)
            };
            return Ok(clause(
                &exists("ALTER CONSTRAINT", if_exists),
                vec![named(name), action],
            ));
        }
        if self.eat_kw("COLUMN") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.ident()?;
            let action: Expression = self.alter_column_action()?;
            return Ok(clause(
                &exists("ALTER COLUMN", if_exists),
                vec![named(name), action],
            ));
        }
        let sub: String = self.sub_entity_type()?;
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let name: Identifier = self.ident()?;
        let action: Expression = self.alter_action()?;
        Ok(clause(
            &exists(&format!("ALTER {sub}"), if_exists),
            vec![named(name), action],
        ))
    }

    /// What `ALTER COLUMN name` sets (type, options, default, generation) or drops.
    fn alter_column_action(&mut self) -> Parsing<Expression> {
        if self.eat_kw("DROP") {
            if self.eat_kw("DEFAULT") {
                return Ok(var("DROP DEFAULT"));
            }
            if self.eat_kws(&["NOT", "NULL"]) {
                return Ok(var("DROP NOT NULL"));
            }
            self.expect_kw("GENERATED")?;
            return Ok(var("DROP GENERATED"));
        }
        self.expect_kw("SET")?;
        if self.eat_kws(&["DATA", "TYPE"]) {
            return Ok(clause("SET DATA TYPE", self.field_schema()?));
        }
        if self.eat_kw("OPTIONS") {
            return Ok(clause("SET OPTIONS", self.options_list()?));
        }
        if self.eat_kw("DEFAULT") {
            return Ok(clause("SET DEFAULT", vec![self.expression()?]));
        }
        self.expect_kw("GENERATED")?;
        let always: bool = match self.eat_kws(&["BY", "DEFAULT"]) {
            true => false,
            false => {
                self.eat_kw("ALWAYS");
                true
            }
        };
        self.expect_kw("AS")?;
        let identity: GeneratedAsIdentity = self.identity_column_info(always)?;
        Ok(clause("SET GENERATED", vec![generated_identity(identity)]))
    }

    /// `field_schema`: a type, `COLLATE`, `NOT NULL` and `OPTIONS`, as operands.
    fn field_schema(&mut self) -> Parsing<Vec<Expression>> {
        let mut operands: Vec<Expression> = vec![Expression::DataType(self.column_type()?)];
        if self.eat_kws(&["NOT", "NULL"]) {
            operands.push(var("NOT NULL"));
        }
        if self.eat_kw("OPTIONS") {
            operands.push(clause("OPTIONS", self.options_list()?));
        }
        Ok(operands)
    }

    /// `column_schema_inner [COLLATE ...]`, which the type reader takes whole.
    fn column_type(&mut self) -> Parsing<DataType> {
        if self.kw_in(0, &["GENERATED", "AS"]) {
            return Err(self.error());
        }
        self.data_type()
    }

    /// `table_column_definition`: a name, its schema, its attributes and its options.
    pub(crate) fn table_column_definition(&mut self) -> Parsing<ColumnDef> {
        let name: Identifier = self.ident()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), DataType::Unknown);
        column.name = name;
        if self.at_generated() {
            let generated: ColumnConstraint = self.generated_column_info()?;
            column.constraints.push(generated);
        } else if !self.at_column_end() && !self.at_column_attribute() {
            column.data_type = self.column_type()?;
            if self.eat_kw("DEFAULT") {
                column.default = Some(self.expression()?);
                if self.at_generated() {
                    return Err(self.error());
                }
            } else if self.at_generated() {
                let generated: ColumnConstraint = self.generated_column_info()?;
                column.constraints.push(generated);
                if self.kw("DEFAULT") {
                    return Err(self.error());
                }
            }
        }
        let mut enforceable: bool = false;
        loop {
            if self.eat_kws(&["PRIMARY", "KEY"]) {
                column.primary_key = true;
                enforceable = true;
            } else if self.eat_kws(&["NOT", "NULL"]) {
                column.nullable = Some(false);
                enforceable = false;
            } else if self.eat_kw("HIDDEN") {
                column.visible = Some(false);
                enforceable = false;
            } else if self.kw("REFERENCES") || self.kw("CONSTRAINT") {
                let name: Option<String> = match self.eat_kw("CONSTRAINT") {
                    true => Some(self.ident()?.name),
                    false => None,
                };
                let mut reference: ForeignKeyRef = self.foreign_key_reference()?;
                reference.constraint_name = name;
                column
                    .constraints
                    .push(ColumnConstraint::References(Box::new(reference)));
                enforceable = true;
            } else if self.kw("ENFORCED") || (self.kw("NOT") && self.kw_at(1, "ENFORCED")) {
                if !enforceable {
                    return Err(self.error_with(
                        "Syntax error: Unexpected constraint enforcement clause",
                        self.offset(),
                        true,
                    ));
                }
                self.eat_kw("NOT");
                self.at += 1;
                enforceable = false;
            } else {
                break;
            }
        }
        if self.eat_kw("OPTIONS") {
            self.options_list()?;
        }
        Ok(column)
    }

    fn at_generated(&self) -> bool {
        self.kw("GENERATED") || self.kw("AS")
    }

    fn at_column_end(&self) -> bool {
        self.terminal_at(0) || self.kw_in(0, &["OPTIONS", "PRECEDING", "FOLLOWING", "FILL"])
    }

    fn at_column_attribute(&self) -> bool {
        (self.kw("PRIMARY") && self.kw_at(1, "KEY"))
            || (self.kw("NOT") && self.kw_at(1, "NULL"))
            || self.kw_in(0, &["HIDDEN", "REFERENCES", "CONSTRAINT"])
    }

    /// `generated_column_info`: a generated `AS ( expression )` or `AS IDENTITY ( ... )`.
    fn generated_column_info(&mut self) -> Parsing<ColumnConstraint> {
        let always: bool = match self.eat_kw("GENERATED") {
            true => match self.eat_kws(&["BY", "DEFAULT"]) {
                true => false,
                false => {
                    self.eat_kw("ALWAYS");
                    true
                }
            },
            false => true,
        };
        self.expect_kw("AS")?;
        if self.kw("IDENTITY") {
            let identity: GeneratedAsIdentity = self.identity_column_info(always)?;
            return Ok(ColumnConstraint::GeneratedAsIdentity(identity));
        }
        let expression: Expression = self.parenthesized_expression()?;
        let persisted: bool = self.eat_kw("STORED");
        let volatile: bool = persisted && self.eat_kw("VOLATILE");
        Ok(ColumnConstraint::ComputedColumn(ComputedColumn {
            expression: Box::new(expression),
            persisted,
            not_null: false,
            persistence_kind: volatile.then(|| "STORED VOLATILE".to_owned()),
            data_type: None,
        }))
    }

    /// `identity_column_info`: `IDENTITY ( ... )` with its start, step, bounds and cycling.
    fn identity_column_info(&mut self, always: bool) -> Parsing<GeneratedAsIdentity> {
        self.expect_kw("IDENTITY")?;
        self.expect_char('(')?;
        let mut identity: GeneratedAsIdentity = GeneratedAsIdentity {
            always,
            on_null: false,
            start: None,
            increment: None,
            minvalue: None,
            maxvalue: None,
            cycle: None,
        };
        if self.eat_kws(&["START", "WITH"]) {
            identity.start = Some(Box::new(self.signed_number()?));
        }
        if self.eat_kws(&["INCREMENT", "BY"]) {
            identity.increment = Some(Box::new(self.signed_number()?));
        }
        if self.eat_kw("MAXVALUE") {
            identity.maxvalue = Some(Box::new(self.signed_number()?));
        }
        if self.eat_kw("MINVALUE") {
            identity.minvalue = Some(Box::new(self.signed_number()?));
        }
        if self.eat_kw("CYCLE") {
            identity.cycle = Some(true);
        } else if self.eat_kws(&["NO", "CYCLE"]) {
            identity.cycle = Some(false);
        }
        self.expect_char(')')?;
        Ok(identity)
    }

    /// `signed_numerical_literal`: an optional `-` and a number.
    fn signed_number(&mut self) -> Parsing<Expression> {
        let negative: bool = self.eat_char('-');
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        if !matches!(token.kind, TokenKind::Number(_)) {
            return Err(self.error());
        }
        let value: Expression = self.primary_expression()?;
        Ok(match negative {
            true => clause("-", vec![value]),
            false => value,
        })
    }

    /// `( expression )`.
    pub(crate) fn parenthesized_expression(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let expression: Expression = self.expression()?;
        self.expect_char(')')?;
        Ok(expression)
    }

    /// `foreign_key_reference`: `REFERENCES path ( columns )`, a match mode and its actions.
    pub(crate) fn foreign_key_reference(&mut self) -> Parsing<ForeignKeyRef> {
        self.expect_kw("REFERENCES")?;
        let target: TableRef = self.table_path()?;
        let columns: Vec<Identifier> = self.identifier_list()?;
        if self.eat_kw("MATCH")
            && !(self.eat_kw("SIMPLE") || self.eat_kw("FULL") || self.eat_kw("NOT_DISTINCT"))
        {
            return Err(self.error());
        }
        let mut reference: ForeignKeyRef = ForeignKeyRef {
            table: target,
            columns,
            on_delete: None,
            on_update: None,
            on_update_first: false,
            match_type: None,
            match_after_actions: false,
            constraint_name: None,
            deferrable: None,
            has_foreign_key_keywords: false,
        };
        for _ in 0..2 {
            if self.kw("ON") && self.kw_at(1, "UPDATE") && reference.on_update.is_none() {
                self.at += 2;
                reference.on_update_first = reference.on_delete.is_none();
                reference.on_update = Some(self.foreign_key_action()?);
            } else if self.kw("ON") && self.kw_at(1, "DELETE") && reference.on_delete.is_none() {
                self.at += 2;
                reference.on_delete = Some(self.foreign_key_action()?);
            }
        }
        Ok(reference)
    }

    fn foreign_key_action(&mut self) -> Parsing<ReferentialAction> {
        if self.eat_kws(&["NO", "ACTION"]) {
            Ok(ReferentialAction::NoAction)
        } else if self.eat_kw("RESTRICT") {
            Ok(ReferentialAction::Restrict)
        } else if self.eat_kw("CASCADE") {
            Ok(ReferentialAction::Cascade)
        } else if self.eat_kws(&["SET", "NULL"]) {
            Ok(ReferentialAction::SetNull)
        } else {
            Err(self.error())
        }
    }

    /// `[NOT] ENFORCED`, when written: whether the constraint is enforced.
    pub(crate) fn enforced(&mut self) -> Option<bool> {
        if self.eat_kw("ENFORCED") {
            return Some(true);
        }
        if self.eat_kws(&["NOT", "ENFORCED"]) {
            return Some(false);
        }
        None
    }

    /// `[NOT] ENFORCED`, when written, as an operand.
    fn constraint_enforcement(&mut self) -> Option<Expression> {
        self.enforced().map(|enforced| match enforced {
            true => var("ENFORCED"),
            false => var("NOT ENFORCED"),
        })
    }

    /// `PRIMARY KEY ( [column, ...] )`'s columns, which take no ordering.
    pub(crate) fn primary_key_columns(&mut self) -> Parsing<Vec<Identifier>> {
        self.expect_kws(&["PRIMARY", "KEY"])?;
        self.expect_char('(')?;
        if self.eat_char(')') {
            return Ok(Vec::new());
        }
        let columns: Vec<Identifier> = self.comma_list(|parser| {
            let start: usize = parser.offset();
            let column: Identifier = parser.ident()?;
            if parser.kw_in(0, &["ASC", "DESC", "NULLS"]) {
                return Err(parser.error_with(
                    "Ordering for primary keys is not supported",
                    start,
                    true,
                ));
            }
            Ok(column)
        })?;
        self.expect_char(')')?;
        Ok(columns)
    }

    /// `primary_key_spec`: `PRIMARY KEY ( [column, ...] ) [[NOT] ENFORCED] [OPTIONS (...)]`.
    pub(crate) fn primary_key_spec(&mut self) -> Parsing<Expression> {
        let columns: Vec<Expression> = self.primary_key_columns()?.into_iter().map(named).collect();
        let mut operands: Vec<Expression> = vec![tuple(columns)];
        operands.extend(self.constraint_enforcement());
        if self.eat_kw("OPTIONS") {
            operands.push(clause("OPTIONS", self.options_list()?));
        }
        Ok(clause("PRIMARY KEY", operands))
    }

    /// `table_constraint_spec`: a `CHECK` or `FOREIGN KEY`, its enforcement and its options.
    pub(crate) fn table_constraint_spec(&mut self) -> Parsing<Expression> {
        let mut operands: Vec<Expression> = Vec::new();
        let keywords: &str = match self.eat_kw("CHECK") {
            true => {
                operands.push(self.parenthesized_expression()?);
                "CHECK"
            }
            false => {
                self.expect_kws(&["FOREIGN", "KEY"])?;
                let columns: Vec<Identifier> = self.identifier_list()?;
                operands.push(tuple(columns.into_iter().map(named).collect()));
                let reference: ForeignKeyRef = self.foreign_key_reference()?;
                operands.push(clause(
                    "REFERENCES",
                    std::iter::once(path_operand(&reference.table))
                        .chain(reference.columns.into_iter().map(named))
                        .collect(),
                ));
                "FOREIGN KEY"
            }
        };
        operands.extend(self.constraint_enforcement());
        if self.eat_kw("OPTIONS") {
            operands.push(clause("OPTIONS", self.options_list()?));
        }
        Ok(clause(keywords, operands))
    }

    /// `row_access_policy_alter_action`.
    fn row_access_policy_action(&mut self) -> Parsing<Expression> {
        if self.eat_kws(&["GRANT", "TO"]) {
            return Ok(clause("GRANT TO", self.grantee_list()?));
        }
        if self.eat_kws(&["FILTER", "USING"]) {
            return Ok(clause(
                "FILTER USING",
                vec![self.parenthesized_expression()?],
            ));
        }
        if self.eat_kws(&["REVOKE", "FROM"]) {
            if self.eat_kw("ALL") {
                return Ok(clause("REVOKE FROM ALL", Vec::new()));
            }
            return Ok(clause("REVOKE FROM", self.grantee_list()?));
        }
        self.expect_kws(&["RENAME", "TO"])?;
        Ok(clause("RENAME TO", vec![named(self.ident()?)]))
    }

    /// `( grantee, ... )` with at least one grantee.
    pub(crate) fn grantee_list(&mut self) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| parser.string_or_parameter())
    }

    /// `string_literal_or_parameter`: a string, `@parameter` or `@@system_variable`.
    pub(crate) fn string_or_parameter(&mut self) -> Parsing<Expression> {
        if self.string_at(0) {
            return self.string_literal();
        }
        if self.is_char('@') || self.is_op("@@") {
            return self.primary_expression();
        }
        Err(self.error())
    }

    /// `generic_sub_entity_type`: one BigQuery supports, consumed.
    fn sub_entity_type(&mut self) -> Parsing<String> {
        match self.word_at(0) {
            Some(word) if SUB_ENTITY_TYPES.contains(&word.as_str()) => {
                self.at += 1;
                Ok(word)
            }
            _ => Err(self.error()),
        }
    }

    /// The keywords `names`, in order, or a syntax error at the first missing one.
    pub(crate) fn expect_kws(&mut self, names: &[&str]) -> Parsing<()> {
        for name in names {
            self.expect_kw(name)?;
        }
        Ok(())
    }
}

/// `keywords` with `IF EXISTS` when written.
fn exists(keywords: &str, if_exists: bool) -> String {
    match if_exists {
        true => format!("{keywords} IF EXISTS"),
        false => keywords.to_owned(),
    }
}

/// `keywords` with `IF NOT EXISTS` when written.
fn not_exists(keywords: &str, if_not_exists: bool) -> String {
    match if_not_exists {
        true => format!("{keywords} IF NOT EXISTS"),
        false => keywords.to_owned(),
    }
}

/// An identity generation as an operand.
fn generated_identity(identity: GeneratedAsIdentity) -> Expression {
    let mut operands: Vec<Expression> = Vec::new();
    for (keywords, value) in [
        ("START WITH", identity.start),
        ("INCREMENT BY", identity.increment),
        ("MAXVALUE", identity.maxvalue),
        ("MINVALUE", identity.minvalue),
    ] {
        if let Some(value) = value {
            operands.push(clause(keywords, vec![*value]));
        }
    }
    if let Some(cycle) = identity.cycle {
        operands.push(var(if cycle { "CYCLE" } else { "NO CYCLE" }));
    }
    let keywords: &str = match identity.always {
        true => "ALWAYS AS IDENTITY",
        false => "BY DEFAULT AS IDENTITY",
    };
    clause(keywords, operands)
}
