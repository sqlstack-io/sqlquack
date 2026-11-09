//! The other `CreateStatementVariation`s (create_*.gram) and `ExtensionRepositoryStatement` (load.gram).

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::queries::models::{Ordered, With};
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateSchema, CreateSequence, CreateTrigger, CreateType, SequenceBound, TriggerBody,
    TriggerEvent, TriggerForEach, TriggerReferencing, TriggerTiming, TypeDefinition,
};
use sqlquack_syntax::expressions::tables::models::{CreateIndex, IndexColumn, IndexKeyPart};

use crate::parse::_helpers::commands::options::{CopyOption, CopyValue, RelOption, RelValue};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::{null, table};
use crate::parse::constants::{
    INDEX_COLLATION, INDEX_NAME, INDEX_OPTION_CONSTANT, PARTIAL_INDEX, REPOSITORY_REPLACE,
    SECRET_NO_VALUE, SECRET_TYPE, SECRET_TYPE_CONSTANT, SEQUENCE_CONSTANT, SEQUENCE_MINUS_CHILD,
    SEQUENCE_OWNED, SEQUENCE_ZERO, TRIGGER_ALIASES, TRIGGER_NEW_TWICE, TRIGGER_OLD_TWICE,
};

/// A sequence option's value once the transformer folds parentheses and minus signs.
enum Folded {
    /// A number literal: its text and whether an odd number of minus signs negate it.
    Number(String, bool),
    /// Another constant: its text, and whether it is `NULL`.
    Constant(String, bool),
    /// A minus whose operand is no number.
    Minus,
    /// A call or operator other than minus.
    Call(String),
    /// Anything else.
    Other,
}

/// What a sequence option sets, in the transformer's names.
pub(crate) struct SequenceSetting {
    pub(crate) key: &'static str,
    value: SequenceValue,
}

/// A sequence option's integer as the option rule leaves it; the statement converts it.
#[derive(Clone, Copy)]
enum SequenceValue {
    /// No value: a flag such as `NO MINVALUE`.
    Absent,
    Null,
    Integer(i128),
    /// More digits than an `INT128` holds.
    Huge,
}

impl Parser<'_> {
    /// `QualifiedName`: `[catalog.][schema.]name`, where a string may only be the last part.
    pub(crate) fn qualified_name(&mut self) -> Parsing<Vec<Identifier>> {
        if self.string_at(0) {
            return Ok(vec![self.col_id_or_string()?]);
        }
        let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
        while self.is_char('.') && (self.is_at(1, is_col_label) || self.string_at(1)) {
            self.at += 1;
            if self.string_at(0) {
                parts.push(self.col_id_or_string()?);
                break;
            }
            parts.push(self.name(is_col_label)?);
        }
        Ok(parts)
    }

    /// `CreateSequenceStmt` after `SEQUENCE`, with the transformer's checks of the options.
    pub(crate) fn create_sequence(
        &mut self,
        or_replace: bool,
        temporary: bool,
        start: usize,
    ) -> Parsing<Expression> {
        let if_not_exists: bool = self.create_if_not_exists(or_replace, start)?;
        let name: TableRef = table(self.qualified_name()?);
        let mut settings: Vec<SequenceSetting> = Vec::new();
        while let Some(setting) = self.sequence_option()? {
            if let Some(earlier) = settings.iter().find(|earlier| earlier.key == setting.key) {
                let mut word: String = earlier.key.to_owned();
                word[..1].make_ascii_uppercase();
                let message: String = format!("{word} should be passed at most once");
                self.refuse_later(&message, start);
            }
            settings.push(setting);
        }
        let find = |key: &str| settings.iter().find(|setting| setting.key == key);
        for (none, bound) in [("nominvalue", "minvalue"), ("nomaxvalue", "maxvalue")] {
            if find(none).is_some() && find(bound).is_some() {
                let mut word: String = bound.to_owned();
                word[..1].make_ascii_uppercase();
                let message: String = format!("{word} should be passed at most once");
                self.refuse_later(&message, start);
            }
        }
        if find("owned").is_some() {
            self.refuse_later(SEQUENCE_OWNED, start);
        }
        let mut values: Vec<(&str, i64)> = Vec::new();
        for setting in &settings {
            let label: &str = match setting.key {
                "increment" => "INCREMENT",
                "minvalue" => "MINVALUE",
                "maxvalue" => "MAXVALUE",
                "start" => "START value",
                _ => continue,
            };
            let refusal: String = match setting.value {
                SequenceValue::Absent => continue,
                SequenceValue::Null => format!("{label} must not be NULL"),
                SequenceValue::Huge => "Unimplemented type \"BIGNUM\" for GetValue()".to_owned(),
                SequenceValue::Integer(value) => match i64::try_from(value) {
                    Ok(value) => {
                        values.push((setting.key, value));
                        continue;
                    }
                    Err(_) => format!(
                        "Type INT128 with value {value} can't be cast because the value is out of range for the destination type INT64"
                    ),
                },
            };
            return Err(self.error_with(&refusal, start, false));
        }
        let value = |key: &str| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| *value)
        };
        let increment: i64 = value("increment").unwrap_or(1);
        if increment == 0 {
            self.refuse_later(SEQUENCE_ZERO, start);
        }
        let (low, high): (i64, i64) = match increment < 0 {
            true => (i64::MIN, -1),
            false => (1, i64::MAX),
        };
        let minimum: i64 = value("minvalue").unwrap_or(low);
        let maximum: i64 = value("maxvalue").unwrap_or(high);
        let first: i64 = value("start").unwrap_or(match increment < 0 {
            true => maximum,
            false => minimum,
        });
        let refusal: Option<String> = if maximum <= minimum {
            Some(format!(
                "MINVALUE ({minimum}) must be less than MAXVALUE ({maximum})"
            ))
        } else if first < minimum {
            Some(format!(
                "START value ({first}) cannot be less than MINVALUE ({minimum})"
            ))
        } else if first > maximum {
            Some(format!(
                "START value ({first}) cannot be greater than MAXVALUE ({maximum})"
            ))
        } else {
            None
        };
        if let Some(message) = refusal {
            self.refuse_later(&message, start);
        }
        let mut node: CreateSequence = CreateSequence::new(String::new());
        node.name = name;
        node.if_not_exists = if_not_exists;
        node.or_replace = or_replace;
        node.temporary = temporary;
        node.increment = value("increment");
        node.start = value("start");
        node.minvalue = bound(
            find("minvalue").is_some(),
            find("nominvalue").is_some(),
            minimum,
        );
        node.maxvalue = bound(
            find("maxvalue").is_some(),
            find("nomaxvalue").is_some(),
            maximum,
        );
        node.cycle =
            find("cycle").is_some_and(|setting| matches!(setting.value, SequenceValue::Integer(1)));
        Ok(Expression::CreateSequence(Box::new(node)))
    }

    /// One `SequenceOption`, or `None` when the next token starts none.
    pub(crate) fn sequence_option(&mut self) -> Parsing<Option<SequenceSetting>> {
        let start: usize = self.offset();
        let setting =
            |key: &'static str, value: SequenceValue| Some(SequenceSetting { key, value });
        if self.eat_kw("CYCLE") {
            return Ok(setting("cycle", SequenceValue::Integer(1)));
        }
        if self.eat_kws(&["NO", "CYCLE"]) {
            return Ok(setting("cycle", SequenceValue::Integer(0)));
        }
        if self.kw("NO") && self.kw_in(1, &["MINVALUE", "MAXVALUE"]) {
            let minimum: bool = self.kw_at(1, "MINVALUE");
            self.at += 2;
            return Ok(setting(
                match minimum {
                    true => "nominvalue",
                    false => "nomaxvalue",
                },
                SequenceValue::Absent,
            ));
        }
        if self.eat_kw("INCREMENT") {
            self.eat_kw("BY");
            let value: SequenceValue = self.sequence_number("INCREMENT", true, start)?;
            return Ok(setting("increment", value));
        }
        if self.kw_in(0, &["MINVALUE", "MAXVALUE"]) {
            let minimum: bool = self.kw("MINVALUE");
            self.at += 1;
            let (key, label) = match minimum {
                true => ("minvalue", "MINVALUE"),
                false => ("maxvalue", "MAXVALUE"),
            };
            let value: SequenceValue = self.sequence_number(label, true, start)?;
            return Ok(setting(key, value));
        }
        if self.eat_kw("START") {
            self.eat_kw("WITH");
            let value: SequenceValue = self.sequence_number("START", false, start)?;
            return Ok(setting("start", value));
        }
        if self.eat_kws(&["OWNED", "BY"]) {
            self.qualified_name()?;
            return Ok(setting("owned", SequenceValue::Absent));
        }
        Ok(None)
    }

    /// A sequence option's integer, refused unless it folds to an integer constant or `NULL`.
    fn sequence_number(
        &mut self,
        label: &str,
        minus_call: bool,
        start: usize,
    ) -> Parsing<SequenceValue> {
        let expression: Expression = self.expression()?;
        if !minus_call && is_binary_minus(&expression) {
            return Err(self.error_with(SEQUENCE_CONSTANT, start, false));
        }
        let folded: Folded = match (fold(&expression), minus_call) {
            (Folded::Call(name), true) => {
                let message: String = format!("Expected a minus function instead of \"{name}\"");
                return Err(self.error_with(&message, start, false));
            }
            (Folded::Minus, true) => {
                return Err(self.error_with(SEQUENCE_MINUS_CHILD, start, false));
            }
            (Folded::Call(_) | Folded::Minus | Folded::Other, _) => {
                return Err(self.error_with(SEQUENCE_CONSTANT, start, false));
            }
            (folded, _) => folded,
        };
        let (text, negative) = match folded {
            Folded::Number(text, negative) => (text, negative),
            Folded::Constant(_, true) => return Ok(SequenceValue::Null),
            Folded::Constant(text, false) => (text, false),
            _ => return Err(self.error_with(SEQUENCE_CONSTANT, start, false)),
        };
        let shown: String = match negative {
            true => format!("-{text}"),
            false => text.clone(),
        };
        let digits: String = text.replace('_', "");
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            let message: String = format!("Expected an integer argument for {label}, got {shown}");
            return Err(self.error_with(&message, start, false));
        }
        let Ok(magnitude) = digits.parse::<i128>() else {
            return Ok(SequenceValue::Huge);
        };
        Ok(SequenceValue::Integer(match negative {
            true => -magnitude,
            false => magnitude,
        }))
    }

    /// `CreateTypeStmt` after `TYPE`: `[IF NOT EXISTS] name AS {ENUM (query) | ENUM (strings) | type}`.
    pub(crate) fn create_type(
        &mut self,
        or_replace: bool,
        temporary: bool,
        start: usize,
    ) -> Parsing<Expression> {
        let if_not_exists: bool = self.create_if_not_exists(or_replace, start)?;
        let name: TableRef = table(self.qualified_name()?);
        self.expect_kw("AS")?;
        let start: usize = self.offset();
        let mut definition: Option<TypeDefinition> = None;
        if self.kw("ENUM") && self.char_at(1, '(') {
            self.at += 1;
            if let Ok(_query) = self.attempt(|parser| parser.parenthesized_query()) {
                self.gap("enum_query", start);
                definition = Some(TypeDefinition::Enum(Vec::new()));
            } else if let Ok(labels) = self.attempt(|parser| parser.enum_labels()) {
                if self.is_char('[') || self.kw("ARRAY") {
                    self.array_bounds(DataType::Unknown)?;
                    self.gap("enum_array", start);
                }
                definition = Some(TypeDefinition::Enum(labels));
            } else {
                self.at -= 1;
            }
        }
        let definition: TypeDefinition = match definition {
            Some(definition) => definition,
            None => TypeDefinition::Alias {
                data_type: self.data_type()?,
                comment: None,
            },
        };
        Ok(Expression::CreateType(Box::new(CreateType {
            name,
            definition,
            if_not_exists,
            or_replace,
            temporary,
        })))
    }

    /// `(string, ...)` or `()`, the labels of an enum.
    fn enum_labels(&mut self) -> Parsing<Vec<String>> {
        self.expect_char('(')?;
        let mut labels: Vec<String> = Vec::new();
        while self.string_at(0) {
            labels.push(self.advance()?.value);
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char(')')?;
        Ok(labels)
    }

    /// `CreateSchemaStmt` after `SCHEMA`: `[IF NOT EXISTS] name [WITH (options)]`.
    pub(crate) fn create_schema(&mut self, or_replace: bool, start: usize) -> Parsing<Expression> {
        let if_not_exists: bool = self.create_if_not_exists(or_replace, start)?;
        let name: Vec<Identifier> = self.qualified_name()?;
        let options: usize = self.offset();
        if self.eat_kw("WITH") {
            self.rel_options()?;
            self.gap("schema_options", options);
        }
        Ok(Expression::CreateSchema(Box::new(CreateSchema {
            name,
            or_replace,
            if_not_exists,
            authorization: None,
            clone_from: None,
            at_clause: None,
            properties: Vec::new(),
            leading_comments: Vec::new(),
        })))
    }

    /// `CreateIndexStmt` from `[UNIQUE] INDEX`, with the transformer's refusals.
    pub(crate) fn create_index(&mut self, or_replace: bool, start: usize) -> Parsing<Expression> {
        let unique: bool = self.eat_kw("UNIQUE");
        self.expect_kw("INDEX")?;
        let if_not_exists: bool = self.create_if_not_exists(or_replace, start)?;
        let name: Option<Identifier> = match self.is(is_identifier) {
            true => Some(self.name(is_identifier)?),
            false => None,
        };
        self.expect_kw("ON")?;
        let target: TableRef = self.base_table_name()?;
        let using: Option<String> = match self.eat_kw("USING") {
            true => Some(self.name(is_identifier)?.name),
            false => None,
        };
        let elements: Vec<Ordered> = self.parenthesized(|parser| parser.sort_item())?;
        let options_start: usize = self.offset();
        let options: Option<Vec<RelOption>> = match self.kw("WITH") {
            true => Some(self.with_list()?),
            false => None,
        };
        let filtered: bool = match self.kw("WHERE") {
            true => {
                self.at += 1;
                self.expression()?;
                true
            }
            false => false,
        };
        let collated: bool = elements
            .iter()
            .any(|element| matches!(element.this, Expression::Collation(_)));
        let constant_options: bool = options
            .iter()
            .flatten()
            .all(|option| option.value != RelValue::NonConstant);
        let refusal: Option<&str> = match (name.is_none(), collated, filtered, constant_options) {
            (true, _, _, _) => Some(INDEX_NAME),
            (_, true, _, _) => Some(INDEX_COLLATION),
            (_, _, true, _) => Some(PARTIAL_INDEX),
            (_, _, _, false) => Some(INDEX_OPTION_CONSTANT),
            _ => None,
        };
        if let Some(message) = refusal {
            self.refuse_later(message, start);
        }
        if options.is_some() {
            self.gap("index_options", options_start);
        }
        let mut node: CreateIndex = CreateIndex::new(String::new(), String::new());
        node.name = name.unwrap_or_else(Identifier::empty);
        node.table = target;
        node.unique = unique;
        node.if_not_exists = if_not_exists;
        node.using = using;
        let plain: bool = elements.iter().all(
            |element| matches!(&element.this, Expression::Column(column) if column.table.is_none()),
        );
        for element in elements {
            match (&element.this, plain) {
                (Expression::Column(column), true) => node.columns.push(IndexColumn {
                    column: column.name.clone(),
                    desc: element.desc,
                    asc: element.explicit_asc,
                    nulls_first: element.nulls_first,
                    opclass: None,
                }),
                _ => node.key_parts.push(IndexKeyPart {
                    expression: Box::new(element.this),
                    prefix_length: None,
                    desc: element.desc,
                    asc: element.explicit_asc,
                    nulls_first: element.nulls_first,
                    opclass: None,
                }),
            }
        }
        Ok(Expression::CreateIndex(Box::new(node)))
    }

    /// `CreateTriggerStmt` after `TRIGGER`; its statement body is a gap.
    pub(crate) fn create_trigger(&mut self, or_replace: bool, start: usize) -> Parsing<Expression> {
        self.create_if_not_exists(or_replace, start)?;
        let name: Identifier = self.name(is_identifier)?;
        let timing: TriggerTiming = if self.eat_kw("BEFORE") {
            TriggerTiming::Before
        } else if self.eat_kw("AFTER") {
            TriggerTiming::After
        } else {
            self.expect_kw("INSTEAD")?;
            self.expect_kw("OF")?;
            TriggerTiming::InsteadOf
        };
        let event: TriggerEvent = if self.eat_kw("INSERT") {
            TriggerEvent::Insert
        } else if self.eat_kw("DELETE") {
            TriggerEvent::Delete
        } else {
            self.expect_kw("UPDATE")?;
            match self.eat_kw("OF") {
                true => TriggerEvent::Update(Some(
                    self.comma_list(|parser| parser.name(is_identifier))?,
                )),
                false => TriggerEvent::Update(None),
            }
        };
        self.expect_kw("ON")?;
        let target: TableRef = self.base_table_name()?;
        let referencing: Option<TriggerReferencing> = self.trigger_referencing(start)?;
        let for_each: Option<TriggerForEach> = match self.eat_kws(&["FOR", "EACH"]) {
            true if self.eat_kw("ROW") => Some(TriggerForEach::Row),
            true => {
                self.expect_kw("STATEMENT")?;
                Some(TriggerForEach::Statement)
            }
            false => None,
        };
        let body_start: usize = self.offset();
        let with: Option<With> = match self.kw("WITH") {
            true => Some(self.ctes()?),
            false => None,
        };
        match self.word_at(0).as_deref() {
            Some("INSERT") => self.insert_statement(with)?,
            Some("UPDATE") => self.update_statement(with)?,
            Some("DELETE") if self.kw_at(1, "FROM") => self.delete_statement(with)?,
            Some("MERGE") => {
                self.skip_statement_rest()?;
                null()
            }
            _ => return Err(self.error()),
        };
        let text: String = self.sql[body_start..self.offset().max(body_start)]
            .trim_end()
            .to_owned();
        self.gap("trigger_body", body_start);
        let mut node: CreateTrigger = CreateTrigger::new(String::new(), String::new());
        node.name = name;
        node.table = target;
        node.timing = timing;
        node.events = vec![event];
        node.for_each = for_each;
        node.referencing = referencing;
        node.or_replace = or_replace;
        node.body = TriggerBody::Block(text);
        Ok(Expression::CreateTrigger(Box::new(node)))
    }

    /// `ReferencingClause`: `REFERENCING {NEW | OLD} TABLE AS name [...]`, each once and distinct.
    fn trigger_referencing(&mut self, start: usize) -> Parsing<Option<TriggerReferencing>> {
        if !self.eat_kw("REFERENCING") {
            return Ok(None);
        }
        let mut node: TriggerReferencing = TriggerReferencing {
            old_table: None,
            new_table: None,
            old_row: None,
            new_row: None,
        };
        for index in 0..2 {
            if index == 1 && !self.kw_in(0, &["NEW", "OLD"]) {
                break;
            }
            let new: bool = self.eat_kw("NEW");
            if !new {
                self.expect_kw("OLD")?;
            }
            self.expect_kw("TABLE")?;
            self.expect_kw("AS")?;
            let alias: Identifier = self.name(is_identifier)?;
            let (slot, message) = match new {
                true => (&mut node.new_table, TRIGGER_NEW_TWICE),
                false => (&mut node.old_table, TRIGGER_OLD_TWICE),
            };
            if slot.replace(alias).is_some() {
                self.refuse_later(message, start);
            }
        }
        if let (Some(new), Some(old)) = (&node.new_table, &node.old_table)
            && new.name.eq_ignore_ascii_case(&old.name)
        {
            return Err(self.error_with(TRIGGER_ALIASES, start, false));
        }
        Ok(Some(node))
    }

    /// `CreateSecretStmt` after `SECRET`: `[IF NOT EXISTS] [name] [IN storage] (options)`, a gap.
    pub(crate) fn create_secret(&mut self, or_replace: bool, start: usize) -> Parsing<Expression> {
        self.create_if_not_exists(or_replace, start)?;
        let named: bool = self.is(is_identifier);
        if named {
            self.at += 1;
        }
        if self.eat_kw("IN") {
            self.name(is_identifier)?;
        }
        let options: Vec<CopyOption> = self.generic_copy_options()?;
        if options.iter().any(|option| option.value.is_none()) {
            self.refuse_later(SECRET_NO_VALUE, start);
        }
        let kind: Option<&CopyOption> = options
            .iter()
            .find(|option| option.name.name.eq_ignore_ascii_case("TYPE"));
        if !named {
            match kind.and_then(|option| option.value.as_ref()) {
                None => return Err(self.error_with(SECRET_TYPE, start, false)),
                Some(CopyValue::Constant { null: false }) => {}
                Some(_) => return Err(self.error_with(SECRET_TYPE_CONSTANT, start, false)),
            }
        }
        self.gap("create_secret", start);
        Ok(null())
    }

    /// `CreateExtensionRepositoryStmt` after `EXTENSION`, a gap.
    pub(crate) fn create_extension_repository(
        &mut self,
        or_replace: bool,
        start: usize,
    ) -> Parsing<Expression> {
        self.expect_kw("REPOSITORY")?;
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        self.col_id_or_string()?;
        self.eat_kw("WITH");
        self.expect_kw("PREFIX")?;
        self.expect_string()?;
        if self.kw("USING") || self.kw("PUBLIC") {
            self.eat_kw("USING");
            self.expect_kw("PUBLIC")?;
            if !self.eat_kw("KEYS") {
                self.expect_kw("KEY")?;
            }
            self.comma_list(|parser| parser.expect_string())?;
        }
        if or_replace && if_not_exists {
            self.refuse_later(REPOSITORY_REPLACE, start);
        }
        self.gap("extension_repository", start);
        Ok(null())
    }

    /// `CreateExternalResourceStmt` after `EXTERNAL`: `RESOURCE 'name' [AS alias] [(options)]`, a gap.
    pub(crate) fn create_external_resource(&mut self, start: usize) -> Parsing<Expression> {
        self.expect_kw("RESOURCE")?;
        self.expect_string()?;
        if self.eat_kw("AS") {
            self.name(is_identifier)?;
        }
        if self.is_char('(') {
            self.generic_copy_options()?;
        }
        self.gap("external_resource", start);
        Ok(null())
    }

    /// A string literal.
    pub(crate) fn expect_string(&mut self) -> Parsing<String> {
        match self.string_at(0) {
            true => Ok(self.advance()?.value),
            false => Err(self.error()),
        }
    }
}

/// A sequence bound from whether it was given, refused with `NO`, and its resolved value.
fn bound(given: bool, none: bool, value: i64) -> Option<SequenceBound> {
    match (given, none) {
        (true, _) => Some(SequenceBound::Value(value)),
        (_, true) => Some(SequenceBound::None),
        _ => None,
    }
}

/// Whether `expression` is `a - b`, a call `START WITH` refuses, once parentheses fold away.
fn is_binary_minus(expression: &Expression) -> bool {
    match expression {
        Expression::Paren(paren) => is_binary_minus(&paren.this),
        Expression::Sub(_) => true,
        _ => false,
    }
}

/// How the transformer sees a sequence option's expression.
fn fold(expression: &Expression) -> Folded {
    match expression {
        Expression::Paren(paren) => fold(&paren.this),
        Expression::Literal(literal) => match &**literal {
            Literal::Number(text) => Folded::Number(text.clone(), false),
            Literal::String(text) => Folded::Constant(format!("'{text}'"), false),
            other => Folded::Constant(format!("{other:?}"), false),
        },
        Expression::Boolean(boolean) => Folded::Constant(boolean.value.to_string(), false),
        Expression::Null(_) => Folded::Constant("NULL".to_owned(), true),
        Expression::Neg(unary) => match fold(&unary.this) {
            Folded::Number(text, negative) => Folded::Number(text, !negative),
            _ => Folded::Minus,
        },
        Expression::Sub(binary) => match fold(&binary.left) {
            Folded::Number(text, negative) => Folded::Number(text, !negative),
            _ => Folded::Minus,
        },
        Expression::Add(_) | Expression::Pos(_) => Folded::Call("+".to_owned()),
        Expression::Mul(_) => Folded::Call("*".to_owned()),
        Expression::Div(_) => Folded::Call("/".to_owned()),
        Expression::Mod(_) => Folded::Call("%".to_owned()),
        Expression::Concat(_) => Folded::Call("||".to_owned()),
        Expression::Function(function) => Folded::Call(function.name.to_lowercase()),
        Expression::Operator(_)
        | Expression::BitwiseAnd(_)
        | Expression::BitwiseOr(_)
        | Expression::BitwiseNot(_)
        | Expression::BitwiseLeftShift(_)
        | Expression::BitwiseRightShift(_)
        | Expression::Like(_)
        | Expression::ILike(_)
        | Expression::Glob(_)
        | Expression::Struct(_)
        | Expression::Array(_)
        | Expression::MethodCall(_) => Folded::Call("function".to_owned()),
        _ => Folded::Other,
    }
}
