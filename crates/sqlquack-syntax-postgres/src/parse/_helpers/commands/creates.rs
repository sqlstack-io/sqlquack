//! `ViewStmt`, `CreateMatViewStmt`, `IndexStmt`, `CreateSeqStmt`, `CreateTrigStmt`, `CREATE TYPE` and `CreateForeignTableStmt` (gram.y).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, CreateView, ViewColumn};
use sqlquack_syntax::expressions::tables::models::CreateTable;

use crate::parse::_helpers::commands::alters::{iconst, name_from};
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::{
    DEFERRABLE, INITIALLY_DEFERRED, SEQUENCE_OPTIONS, check_attributes, clause, name_value,
    unsupported,
};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{function, literal, null, table, tuple};
use crate::scan::models::TokenKind;

/// The kinds this module creates, longest spelling first, with their kind text.
const CREATED_KINDS: [(&[&str], &str); 7] = [
    (&["materialized", "view"], MATERIALIZED_VIEW),
    (&["foreign", "table"], FOREIGN_TABLE),
    (&["view"], VIEW),
    (&["index"], INDEX),
    (&["sequence"], SEQUENCE),
    (&["trigger"], TRIGGER),
    (&["type"], TYPE),
];

const MATERIALIZED_VIEW: &str = "MATERIALIZED VIEW";
const FOREIGN_TABLE: &str = "FOREIGN TABLE";
const VIEW: &str = "VIEW";
const INDEX: &str = "INDEX";
const SEQUENCE: &str = "SEQUENCE";
const TRIGGER: &str = "TRIGGER";
const TYPE: &str = "TYPE";

/// `OptTemp` spellings, longest first, with the persistence each gives.
pub(crate) const PERSISTENCES: [(&[&str], &str); 7] = [
    (&["local", "temporary"], TEMPORARY),
    (&["local", "temp"], TEMPORARY),
    (&["global", "temporary"], TEMPORARY),
    (&["global", "temp"], TEMPORARY),
    (&["temporary"], TEMPORARY),
    (&["temp"], TEMPORARY),
    (&["unlogged"], UNLOGGED),
];

const TEMPORARY: &str = "TEMPORARY";
const UNLOGGED: &str = "UNLOGGED";

/// Words between `OptTemp` and the kind: `RECURSIVE` (views), `UNIQUE` (indexes), `CONSTRAINT` (triggers).
const KIND_PREFIXES: [(&str, &str); 3] = [
    ("recursive", RECURSIVE),
    ("unique", UNIQUE),
    ("constraint", CONSTRAINT),
];

const RECURSIVE: &str = "RECURSIVE";
const UNIQUE: &str = "UNIQUE";
const CONSTRAINT: &str = "CONSTRAINT";

/// The access method an index without `USING` gets, as gram.y's `DEFAULT_INDEX_TYPE`.
const DEFAULT_INDEX_METHOD: &str = "btree";

/// Trigger events in `TRIGGER_TYPE_*` bit order, so the tree lists them as the raw tree's bitmask does.
const TRIGGER_EVENTS: [&str; 4] = ["insert", "delete", "update", "truncate"];

/// `TriggerEvents`' error when an event repeats (42601).
const DUPLICATE_EVENTS: &str = "duplicate trigger events specified";

/// Words that may begin what follows `CREATE [OR REPLACE] [OptTemp]`, and the words that must follow each.
const CREATE_WORDS: [(&str, &[&str]); 38] = [
    ("access", &["method"]),
    ("aggregate", &[]),
    ("assertion", &[]),
    ("cast", &[]),
    ("collation", &[]),
    ("constraint", &["trigger"]),
    ("conversion", &[]),
    ("database", &[]),
    ("default", &["conversion"]),
    ("domain", &[]),
    ("event", &["trigger"]),
    ("extension", &[]),
    ("foreign", &["data", "table"]),
    ("function", &[]),
    ("group", &[]),
    ("index", &[]),
    ("language", &[]),
    ("materialized", &["view"]),
    ("operator", &[]),
    ("policy", &[]),
    ("procedure", &[]),
    ("procedural", &["language"]),
    ("publication", &[]),
    ("recursive", &["view"]),
    ("role", &[]),
    ("rule", &[]),
    ("schema", &[]),
    ("sequence", &[]),
    ("server", &[]),
    ("statistics", &[]),
    ("subscription", &[]),
    ("table", &[]),
    ("tablespace", &[]),
    ("text", &["search"]),
    ("transform", &[]),
    ("trigger", &[]),
    ("trusted", &["procedural", "language"]),
    ("type", &[]),
];

/// More `CREATE` words: `UNIQUE INDEX`, `USER`, `VIEW`.
const CREATE_WORDS_MORE: [(&str, &[&str]); 3] =
    [("unique", &["index"]), ("user", &[]), ("view", &[])];

/// Two-word `CREATE` kinds and the words that must follow them.
const CREATE_THIRD_WORDS: [(&str, &str, &[&str]); 2] = [
    (
        "text",
        "search",
        &["parser", "dictionary", "template", "configuration"],
    ),
    ("foreign", "data", &["wrapper"]),
];

/// gram.y's error for `CREATE ASSERTION`, raised after the whole statement parses (0A000).
const ASSERTION: &str = "CREATE ASSERTION is not yet implemented";

/// gram.y's error for `CREATE OR REPLACE CONSTRAINT TRIGGER` (0A000).
const REPLACE_CONSTRAINT_TRIGGER: &str = "CREATE OR REPLACE CONSTRAINT TRIGGER is not supported";

/// gram.y's error for a check option on a recursive view (0A000).
const RECURSIVE_CHECK: &str = "WITH CHECK OPTION not supported on recursive views";

/// The words before a created kind: `OR REPLACE`, a persistence, and a kind prefix.
#[derive(Clone, Copy)]
struct CreatePrefix {
    or_replace: bool,
    persistence: Option<&'static str>,
    word: Option<&'static str>,
}

/// A `CreateObject` of `kind` named `name`, with its prefix's `OR REPLACE` and modifier.
fn created(kind: &str, name: TableRef, prefix: CreatePrefix) -> CreateObject {
    CreateObject {
        kind: kind.to_owned(),
        modifier: prefix.persistence.or(prefix.word).map(str::to_owned),
        or_replace: prefix.or_replace,
        or_alter: false,
        if_not_exists: false,
        name,
        source: String::new(),
        operands: Vec::new(),
        properties: Vec::new(),
    }
}

impl Parser<'_> {
    /// Whether `words` follow at `offset`.
    pub(crate) fn words_from(&self, offset: usize, words: &[&str]) -> bool {
        for (index, word) in words.iter().enumerate() {
            if !self.kw_at(offset + index, word) {
                return false;
            }
        }
        true
    }

    /// The offset of the token after `CREATE` that no `CREATE` statement allows, if there is one.
    pub(crate) fn truncated_create(&self) -> Option<usize> {
        if !self.kw("create") {
            return None;
        }
        let mut offset: usize = 1;
        if self.kw_at(offset, "or") && self.kw_at(offset + 1, "replace") {
            offset += 2;
        }
        if let Some((words, _)) = PERSISTENCES
            .iter()
            .find(|(words, _)| self.words_from(offset, words))
        {
            offset += words.len();
        }
        let Some((word, follows)) = CREATE_WORDS
            .iter()
            .chain(CREATE_WORDS_MORE.iter())
            .find(|(word, _)| self.kw_at(offset, word))
        else {
            return Some(offset);
        };
        if !follows.is_empty() && !follows.iter().any(|next| self.kw_at(offset + 1, next)) {
            return Some(offset + 1);
        }
        let third = CREATE_THIRD_WORDS
            .iter()
            .find(|(first, second, _)| first == word && self.kw_at(offset + 1, second));
        match third {
            Some((_, _, words)) if !words.iter().any(|next| self.kw_at(offset + 2, next)) => {
                Some(offset + 2)
            }
            _ => None,
        }
    }

    /// `CreateAssertionStmt`: `CREATE ASSERTION any_name CHECK (a_expr) ConstraintAttributeSpec`, always refused.
    pub(crate) fn create_assertion(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("create")?;
        self.expect_kw("assertion")?;
        self.qualified_name()?;
        self.expect_kw("check")?;
        self.parenthesized_condition()?;
        self.constraint_attributes()?;
        Err(unsupported(ASSERTION.to_owned(), start))
    }

    /// The kind `CREATE` creates here, if this module creates it.
    pub(crate) fn created_kind(&self) -> Option<&'static str> {
        if !self.kw("create") {
            return None;
        }
        let mut offset: usize = 1;
        if self.kw_at(offset, "or") && self.kw_at(offset + 1, "replace") {
            offset += 2;
        }
        if let Some((words, _)) = PERSISTENCES
            .iter()
            .find(|(words, _)| self.words_from(offset, words))
        {
            offset += words.len();
        }
        if KIND_PREFIXES
            .iter()
            .any(|(word, _)| self.kw_at(offset, word))
        {
            offset += 1;
        }
        CREATED_KINDS
            .iter()
            .find(|(words, _)| self.words_from(offset, words))
            .map(|(_, kind)| *kind)
    }

    /// `CREATE [OR REPLACE] [OptTemp] [RECURSIVE | UNIQUE | CONSTRAINT] kind ...`.
    pub(crate) fn create_kind(&mut self, kind: &'static str) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("create")?;
        let or_replace: bool = self.kw("or") && self.kw_at(1, "replace");
        if or_replace {
            self.at += 2;
        }
        let persistence: Option<&'static str> = match PERSISTENCES
            .iter()
            .find(|(words, _)| self.words_from(0, words))
        {
            Some((words, persistence)) => {
                self.at += words.len();
                Some(*persistence)
            }
            None => None,
        };
        let word: Option<&'static str> = match KIND_PREFIXES.iter().find(|(word, _)| self.kw(word))
        {
            Some((_, word)) => {
                self.at += 1;
                Some(*word)
            }
            None => None,
        };
        let prefix = CreatePrefix {
            or_replace,
            persistence,
            word,
        };
        if !prefix_allowed(kind, prefix) {
            return Err(self.error());
        }
        self.at += kind.split(' ').count();
        match kind {
            VIEW => self.create_view(prefix),
            MATERIALIZED_VIEW => self.create_materialized_view(prefix),
            INDEX => self.create_index(prefix),
            SEQUENCE => self.create_sequence(prefix),
            TRIGGER => {
                let trigger: Expression = self.create_trigger(prefix)?;
                match prefix.or_replace && prefix.word == Some(CONSTRAINT) {
                    true => Err(unsupported(REPLACE_CONSTRAINT_TRIGGER.to_owned(), start)),
                    false => Ok(trigger),
                }
            }
            TYPE => self.create_type(),
            _ => self.create_foreign_table(),
        }
    }

    /// `[IF NOT EXISTS]`, consumed.
    pub(crate) fn if_not_exists(&mut self) -> Parsing<bool> {
        if !(self.kw("if") && self.kw_at(1, "not")) {
            return Ok(false);
        }
        self.at += 2;
        self.expect_kw("exists")?;
        Ok(true)
    }

    /// `WITH reloptions`, if written, as a `WITH` clause.
    pub(crate) fn reloptions(&mut self) -> Parsing<Option<Expression>> {
        match self.kw("with") && self.char_at(1, '(') {
            true => {
                self.at += 1;
                Ok(Some(clause("WITH", self.options(true)?)))
            }
            false => Ok(None),
        }
    }

    /// `OptTableSpace`: `TABLESPACE name`, if written.
    fn table_space(&mut self) -> Parsing<Option<Expression>> {
        match self.eat_kw("tablespace") {
            true => Ok(Some(clause(
                "TABLESPACE",
                vec![name_from(self.name(is_col_id)?)],
            ))),
            false => Ok(None),
        }
    }

    /// `AS SelectStmt`.
    fn view_query(&mut self) -> Parsing<Expression> {
        self.expect_kw("as")?;
        match self.is_char('(') || self.starts_query_at(0) {
            true => self.query(),
            false => Err(self.error()),
        }
    }

    /// `ViewStmt` after `VIEW`.
    fn create_view(&mut self, prefix: CreatePrefix) -> Parsing<Expression> {
        let mut node: CreateView = CreateView::new(String::new(), null());
        node.name = table(self.table_name()?);
        node.or_replace = prefix.or_replace;
        node.temporary = prefix.persistence == Some(TEMPORARY);
        let recursive: bool = prefix.word == Some(RECURSIVE);
        if prefix.persistence == Some(UNLOGGED) {
            node.options.push(var(UNLOGGED));
        }
        if recursive {
            node.options.push(var(RECURSIVE));
        }
        if recursive || self.is_char('(') {
            node.columns = view_columns(self.name_list()?);
        }
        node.options.extend(self.reloptions()?);
        node.query = self.view_query()?;
        let start: usize = self.offset();
        if self.kw("with") {
            self.at += 1;
            let level: &str = match self.eat_kw("local") {
                true => "LOCAL",
                false => {
                    self.eat_kw("cascaded");
                    "CASCADED"
                }
            };
            self.expect_kw("check")?;
            self.expect_kw("option")?;
            if recursive {
                return Err(unsupported(RECURSIVE_CHECK.to_owned(), start));
            }
            node.options.push(clause("CHECK OPTION", vec![var(level)]));
        }
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// `CreateMatViewStmt` after `MATERIALIZED VIEW`.
    fn create_materialized_view(&mut self, prefix: CreatePrefix) -> Parsing<Expression> {
        let mut node: CreateView = CreateView::new(String::new(), null());
        node.materialized = true;
        node.if_not_exists = self.if_not_exists()?;
        node.name = table(self.table_name()?);
        if prefix.persistence.is_some() {
            node.options.push(var(UNLOGGED));
        }
        if self.is_char('(') {
            node.columns = view_columns(self.name_list()?);
        }
        if self.eat_kw("using") {
            node.options
                .push(clause("USING", vec![name_from(self.name(is_col_id)?)]));
        }
        node.options.extend(self.reloptions()?);
        node.options.extend(self.table_space()?);
        node.query = self.view_query()?;
        if self.eat_kw("with") {
            if self.eat_kw("no") {
                node.options.push(var("WITH NO DATA"));
            }
            self.expect_kw("data")?;
        }
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// `IndexStmt` after `INDEX`.
    fn create_index(&mut self, prefix: CreatePrefix) -> Parsing<Expression> {
        let concurrently: bool = self.eat_kw("concurrently");
        let if_not_exists: bool = self.if_not_exists()?;
        let name: Vec<Identifier> = match if_not_exists || !self.kw("on") {
            true => vec![self.name(is_col_id)?],
            false => Vec::new(),
        };
        let mut node: CreateObject = created(INDEX, table(name), prefix);
        node.if_not_exists = if_not_exists;
        if concurrently {
            node.operands.push(var("CONCURRENTLY"));
        }
        self.expect_kw("on")?;
        let relation: Expression = Expression::Table(Box::new(self.relation()?));
        node.operands.push(clause("ON", vec![relation]));
        let method: Identifier = match self.eat_kw("using") {
            true => self.name(is_col_id)?,
            false => Identifier::new(DEFAULT_INDEX_METHOD),
        };
        node.operands
            .push(clause("USING", vec![Expression::Identifier(method)]));
        let keys: Vec<Expression> = self.parenthesized(|parser| parser.key_element(true))?;
        node.operands.push(clause("KEYS", keys));
        if self.eat_kw("include") {
            let included: Vec<Expression> =
                self.parenthesized(|parser| parser.key_element(true))?;
            node.operands.push(clause("INCLUDE", included));
        }
        if self.kw("nulls") {
            self.at += 1;
            let not: bool = self.eat_kw("not");
            self.expect_kw("distinct")?;
            if not {
                node.operands.push(var("NULLS NOT DISTINCT"));
            }
        }
        node.operands.extend(self.reloptions()?);
        node.operands.extend(self.table_space()?);
        if self.eat_kw("where") {
            node.operands.push(clause("WHERE", vec![self.a_expr()?]));
        }
        Ok(Expression::CreateObject(Box::new(node)))
    }

    /// `CreateSeqStmt` after `SEQUENCE`.
    fn create_sequence(&mut self, prefix: CreatePrefix) -> Parsing<Expression> {
        let if_not_exists: bool = self.if_not_exists()?;
        let mut node: CreateObject = created(SEQUENCE, table(self.table_name()?), prefix);
        node.if_not_exists = if_not_exists;
        while self
            .keyword_at(0)
            .is_some_and(|word| SEQUENCE_OPTIONS.contains(&word))
        {
            node.operands.push(self.sequence_option()?);
        }
        Ok(Expression::CreateObject(Box::new(node)))
    }

    /// `CreateTrigStmt` after `TRIGGER`.
    fn create_trigger(&mut self, prefix: CreatePrefix) -> Parsing<Expression> {
        let constraint: bool = prefix.word == Some(CONSTRAINT);
        let mut node: CreateObject = created(TRIGGER, table(vec![self.name(is_col_id)?]), prefix);
        let timing: &str = match self.keyword_at(0) {
            Some("before") if !constraint => "BEFORE",
            Some("instead") if !constraint && self.kw_at(1, "of") => "INSTEAD OF",
            Some("after") => "AFTER",
            _ => return Err(self.error()),
        };
        self.at += timing.split(' ').count();
        node.operands.push(var(timing));
        node.operands.extend(self.trigger_events()?);
        self.expect_kw("on")?;
        let relation: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        node.operands.push(clause("ON", vec![relation]));
        match constraint {
            true => node.operands.extend(self.constraint_trigger_options()?),
            false => node.operands.extend(self.trigger_options()?),
        }
        if self.eat_kw("when") {
            node.operands
                .push(clause("WHEN", vec![self.parenthesized_condition()?]));
        }
        node.operands.push(self.trigger_call()?);
        Ok(Expression::CreateObject(Box::new(node)))
    }

    /// `TriggerEvents`: the events in bit order, then `UPDATE OF`'s columns in source order.
    fn trigger_events(&mut self) -> Parsing<Vec<Expression>> {
        let mut seen: [bool; 4] = [false; 4];
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let Some(index) = TRIGGER_EVENTS.iter().position(|word| self.kw(word)) else {
                return Err(self.error());
            };
            self.at += 1;
            if seen[index] {
                return Err(self.error_with(DUPLICATE_EVENTS, self.offset()));
            }
            seen[index] = true;
            if TRIGGER_EVENTS[index] == TRIGGER_EVENTS[2] && self.eat_kw("of") {
                columns.extend(self.comma_list(|parser| Ok(name_from(parser.name(is_col_id)?)))?);
            }
            if !self.eat_kw("or") {
                break;
            }
        }
        let mut events: Vec<Expression> = TRIGGER_EVENTS
            .iter()
            .zip(seen)
            .filter(|(_, seen)| *seen)
            .map(|(word, _)| var(&word.to_uppercase()))
            .collect();
        if !columns.is_empty() {
            events.push(clause("OF", columns));
        }
        Ok(vec![clause("EVENTS", events)])
    }

    /// `TriggerReferencing TriggerForSpec`: transition tables in order, and `FOR EACH ROW` when it is.
    fn trigger_options(&mut self) -> Parsing<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();
        if self.eat_kw("referencing") {
            let mut transitions: Vec<Expression> = vec![self.transition()?];
            while self.kw("new") || self.kw("old") {
                transitions.push(self.transition()?);
            }
            options.push(clause("REFERENCING", transitions));
        }
        if self.eat_kw("for") {
            self.eat_kw("each");
            if self.eat_kw("row") {
                options.push(var("FOR EACH ROW"));
            } else {
                self.expect_kw("statement")?;
            }
        }
        Ok(options)
    }

    /// `TriggerTransition`: `NEW | OLD`, `TABLE | ROW`, `[AS] name`.
    fn transition(&mut self) -> Parsing<Expression> {
        let age: &str = match self.eat_kw("new") {
            true => "NEW",
            false => {
                self.expect_kw("old")?;
                "OLD"
            }
        };
        let shape: &str = match self.eat_kw("table") {
            true => "TABLE",
            false => {
                self.expect_kw("row")?;
                "ROW"
            }
        };
        self.eat_kw("as");
        Ok(tuple(vec![
            var(age),
            var(shape),
            name_from(self.name(is_col_id)?),
        ]))
    }

    /// `OptConstrFromTable ConstraintAttributeSpec FOR EACH ROW`, the attributes as `processCASbits` keeps them.
    fn constraint_trigger_options(&mut self) -> Parsing<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();
        if self.eat_kw("from") {
            let referenced: Expression = Expression::Table(Box::new(table(self.table_name()?)));
            options.push(clause("FROM", vec![referenced]));
        }
        let start: usize = self.offset();
        let (bits, _) = self.constraint_attributes()?;
        check_attributes(TRIGGER, DEFERRABLE, bits, start)?;
        if bits & (DEFERRABLE | INITIALLY_DEFERRED) != 0 {
            options.push(var("DEFERRABLE"));
        }
        if bits & INITIALLY_DEFERRED != 0 {
            options.push(var("INITIALLY DEFERRED"));
        }
        self.expect_kw("for")?;
        self.expect_kw("each")?;
        self.expect_kw("row")?;
        options.push(var("FOR EACH ROW"));
        Ok(options)
    }

    /// `EXECUTE FUNCTION | PROCEDURE func_name ( TriggerFuncArgs )`, each argument the string gram.y makes.
    fn trigger_call(&mut self) -> Parsing<Expression> {
        self.expect_kw("execute")?;
        if !self.eat_kw("function") {
            self.expect_kw("procedure")?;
        }
        let name: Vec<Identifier> = self.func_name()?;
        self.expect_char('(')?;
        let arguments: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.trigger_argument())?,
        };
        self.expect_char(')')?;
        Ok(clause(
            "EXECUTE",
            vec![Expression::Function(Box::new(function(name, arguments)))],
        ))
    }

    /// One `TriggerFuncArg`: an integer's decimal text, a number's text, a string, or a folded label.
    fn trigger_argument(&mut self) -> Parsing<Expression> {
        if self.sconst_at(0) {
            return self.sconst();
        }
        let token = self.advance()?;
        let text: String = match token.kind {
            TokenKind::Integer => iconst(&token.value).to_string(),
            TokenKind::Numeric => token.value.clone(),
            _ if is_col_label(&token) => identifier(&token).name,
            _ => {
                self.at -= 1;
                return Err(self.error());
            }
        };
        Ok(literal(Literal::String(text)))
    }

    /// `CREATE TYPE any_name` then a definition, a composite (named by at most three parts), an enum, a range, or nothing.
    fn create_type(&mut self) -> Parsing<Expression> {
        let name_at: usize = self.at;
        let mut node: CreateObject = created(
            TYPE,
            table(self.qualified_name()?),
            CreatePrefix {
                or_replace: false,
                persistence: None,
                word: None,
            },
        );
        if self.is_char('(') {
            node.properties = self.definition_list()?;
            return Ok(Expression::CreateObject(Box::new(node)));
        }
        if !self.eat_kw("as") {
            return Ok(Expression::CreateObject(Box::new(node)));
        }
        let (source, operands, properties) = match self.keyword_at(0) {
            Some("enum") => {
                self.at += 1;
                self.expect_char('(')?;
                let labels: Vec<Expression> = match self.is_char(')') {
                    true => Vec::new(),
                    false => self.comma_list(|parser| parser.sconst())?,
                };
                self.expect_char(')')?;
                ("AS ENUM", labels, Vec::new())
            }
            Some("range") => {
                self.at += 1;
                ("AS RANGE", Vec::new(), self.definition_list()?)
            }
            _ => {
                let end: usize = self.at;
                self.at = name_at;
                self.table_name()?;
                self.at = end;
                ("AS", self.composite_attributes()?, Vec::new())
            }
        };
        node.source = source.to_owned();
        node.operands = operands;
        node.properties = properties;
        Ok(Expression::CreateObject(Box::new(node)))
    }

    /// `'(' OptTableFuncElementList ')'`: each attribute as `(name, type [, (COLLATE, name)])`.
    fn composite_attributes(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_char('(')?;
        if self.eat_char(')') {
            return Ok(Vec::new());
        }
        let attributes: Vec<Expression> = self.comma_list(|parser| {
            let mut parts: Vec<Expression> = vec![
                name_from(parser.name(is_col_id)?),
                Expression::DataType(parser.typename()?),
            ];
            if parser.eat_kw("collate") {
                parts.push(clause(
                    "COLLATE",
                    vec![name_value(&parser.qualified_name()?)],
                ));
            }
            Ok(tuple(parts))
        })?;
        self.expect_char(')')?;
        Ok(attributes)
    }

    /// `CreateForeignTableStmt` after `FOREIGN TABLE`, as a `CreateTable` with the `FOREIGN` modifier.
    fn create_foreign_table(&mut self) -> Parsing<Expression> {
        let mut node: CreateTable = CreateTable::new(String::new());
        node.table_modifier = Some("FOREIGN".to_owned());
        node.if_not_exists = self.if_not_exists()?;
        node.name = table(self.table_name()?);
        if self.kw("partition") && self.kw_at(1, "of") {
            self.at += 2;
            node.partition_of = Some(Expression::Table(Box::new(table(self.table_name()?))));
            if self.eat_char('(') {
                node = self.table_elements(node, true)?;
                self.expect_char(')')?;
            }
            let bound: Expression = self.partition_bound()?;
            node.properties.push(bound);
        } else {
            self.expect_char('(')?;
            if !self.is_char(')') {
                node = self.table_elements(node, false)?;
            }
            self.expect_char(')')?;
            if self.eat_kw("inherits") {
                let parents: Vec<Vec<Identifier>> =
                    self.parenthesized(|parser| parser.table_name())?;
                node.inherits = parents.into_iter().map(table).collect();
            }
        }
        self.expect_kw("server")?;
        node.properties
            .push(clause("SERVER", vec![name_from(self.name(is_col_id)?)]));
        node.properties.extend(self.create_generic_options()?);
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// `create_generic_options`: `OPTIONS ( name 'value', ... )`, if written.
    pub(crate) fn create_generic_options(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("options") {
            return Ok(None);
        }
        let options: Vec<Expression> = self.parenthesized(|parser| {
            let name: Identifier = parser.name(is_col_label)?;
            Ok(tuple(vec![Expression::Identifier(name), parser.sconst()?]))
        })?;
        Ok(Some(clause("OPTIONS", options)))
    }
}

/// Whether gram.y allows `prefix` before `kind`.
fn prefix_allowed(kind: &str, prefix: CreatePrefix) -> bool {
    let (replace, persistences, word): (bool, &[&str], Option<&str>) = match kind {
        VIEW => (true, &[TEMPORARY, UNLOGGED], Some(RECURSIVE)),
        MATERIALIZED_VIEW => (false, &[UNLOGGED], None),
        INDEX => (false, &[], Some(UNIQUE)),
        SEQUENCE => (false, &[TEMPORARY, UNLOGGED], None),
        TRIGGER => (true, &[], Some(CONSTRAINT)),
        _ => (false, &[], None),
    };
    (replace || !prefix.or_replace)
        && prefix
            .persistence
            .is_none_or(|persistence| persistences.contains(&persistence))
        && (prefix.word.is_none() || prefix.word == word)
}

/// A view's column names.
fn view_columns(names: Vec<Identifier>) -> Vec<ViewColumn> {
    names
        .into_iter()
        .map(|name| {
            let mut column: ViewColumn = ViewColumn::new(String::new());
            column.name = name;
            column
        })
        .collect()
}
