//! The `ALTER` statements of named objects other than relations and routines (gram.y), as `AlterObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};

use crate::parse::_helpers::commands::alters::{alter, name_from};
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::{
    NO_INHERIT, NOT_VALID, check_attributes, clause, name_value, unsupported,
};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{table, tuple};

/// The object kinds this module alters, longest spelling first, with their kind text.
const OBJECT_KINDS: [(&[&str], &str); 26] = [
    (&["text", "search", "configuration"], TS_CONFIGURATION),
    (&["text", "search", "dictionary"], TS_DICTIONARY),
    (&["text", "search", "parser"], TS_PARSER),
    (&["text", "search", "template"], TS_TEMPLATE),
    (&["foreign", "data", "wrapper"], FOREIGN_DATA_WRAPPER),
    (&["procedural", "language"], LANGUAGE),
    (&["event", "trigger"], EVENT_TRIGGER),
    (&["large", "object"], LARGE_OBJECT),
    (&["operator", "class"], OPERATOR_CLASS),
    (&["operator", "family"], OPERATOR_FAMILY),
    (&["operator"], OPERATOR),
    (&["collation"], COLLATION),
    (&["conversion"], CONVERSION),
    (&["database"], DATABASE),
    (&["domain"], DOMAIN),
    (&["extension"], EXTENSION),
    (&["language"], LANGUAGE),
    (&["publication"], PUBLICATION),
    (&["schema"], SCHEMA),
    (&["server"], SERVER),
    (&["statistics"], STATISTICS),
    (&["subscription"], SUBSCRIPTION),
    (&["tablespace"], TABLESPACE),
    (&["trigger"], TRIGGER),
    (&["rule"], RULE),
    (&["type"], TYPE),
];

const TS_CONFIGURATION: &str = "TEXT SEARCH CONFIGURATION";
const TS_DICTIONARY: &str = "TEXT SEARCH DICTIONARY";
const TS_PARSER: &str = "TEXT SEARCH PARSER";
const TS_TEMPLATE: &str = "TEXT SEARCH TEMPLATE";
const FOREIGN_DATA_WRAPPER: &str = "FOREIGN DATA WRAPPER";
const LANGUAGE: &str = "LANGUAGE";
const EVENT_TRIGGER: &str = "EVENT TRIGGER";
const LARGE_OBJECT: &str = "LARGE OBJECT";
const OPERATOR_CLASS: &str = "OPERATOR CLASS";
const OPERATOR_FAMILY: &str = "OPERATOR FAMILY";
const OPERATOR: &str = "OPERATOR";
const COLLATION: &str = "COLLATION";
const CONVERSION: &str = "CONVERSION";
const DATABASE: &str = "DATABASE";
const DOMAIN: &str = "DOMAIN";
const EXTENSION: &str = "EXTENSION";
const PUBLICATION: &str = "PUBLICATION";
const SCHEMA: &str = "SCHEMA";
const SERVER: &str = "SERVER";
const STATISTICS: &str = "STATISTICS";
const SUBSCRIPTION: &str = "SUBSCRIPTION";
const TABLESPACE: &str = "TABLESPACE";
const TRIGGER: &str = "TRIGGER";
const RULE: &str = "RULE";
const TYPE: &str = "TYPE";

/// Kinds named by `any_name`; the others are named by one `name`, or as their grammar names them.
const ANY_NAME_KINDS: [&str; 11] = [
    TS_CONFIGURATION,
    TS_DICTIONARY,
    TS_PARSER,
    TS_TEMPLATE,
    OPERATOR_CLASS,
    OPERATOR_FAMILY,
    COLLATION,
    CONVERSION,
    DOMAIN,
    STATISTICS,
    TYPE,
];

/// Kinds `RENAME TO` does not rename.
const UNRENAMED_KINDS: [&str; 3] = [OPERATOR, EXTENSION, LARGE_OBJECT];

/// Kinds `OWNER TO` does not re-own.
const UNOWNED_KINDS: [&str; 5] = [TS_PARSER, TS_TEMPLATE, EXTENSION, TRIGGER, RULE];

/// Kinds `SET SCHEMA` moves.
const SCHEMA_KINDS: [&str; 13] = [
    TS_CONFIGURATION,
    TS_DICTIONARY,
    TS_PARSER,
    TS_TEMPLATE,
    OPERATOR_CLASS,
    OPERATOR_FAMILY,
    OPERATOR,
    COLLATION,
    CONVERSION,
    DOMAIN,
    EXTENSION,
    STATISTICS,
    TYPE,
];

/// Kinds named by `name ON qualified_name`.
const ON_TABLE_KINDS: [&str; 2] = [TRIGGER, RULE];

/// Kinds `COMMENT ON` names that `ALTER EXTENSION ADD | DROP` does not.
const UNMEMBER_KINDS: [&str; 6] = ["column", "constraint", "policy", "rule", "trigger", "large"];

/// `enable_trigger` spellings.
const TRIGGER_MODES: [(&[&str], &str); 4] = [
    (&["enable", "replica"], "ENABLE REPLICA"),
    (&["enable", "always"], "ENABLE ALWAYS"),
    (&["enable"], "ENABLE"),
    (&["disable"], "DISABLE"),
];

/// The `ALTER TYPE` commands on attributes, which name the type as a relation of at most three parts.
const ATTRIBUTE_COMMANDS: [&[&str]; 4] = [
    &["add", "attribute"],
    &["drop", "attribute"],
    &["alter", "attribute"],
    &["rename", "attribute"],
];

/// `DROP VALUE`'s error (0A000), which gram.y raises for every enum.
const DROP_VALUE: &str = "dropping an enum value is not implemented";

/// The action of `ALTER DATABASE`'s `createdb_opt_list`, with or without `WITH`.
const DATABASE_OPTIONS_ACTION: &str = "OPTIONS";

impl Parser<'_> {
    /// The object kind `ALTER` alters here and its word count, if this module alters it.
    pub(crate) fn altered_object(&self) -> Option<(&'static str, usize)> {
        if !self.kw("alter") {
            return None;
        }
        OBJECT_KINDS
            .iter()
            .find(|(words, _)| self.kws_after_alter(words) && !self.char_at(words.len() + 1, '.'))
            .map(|(words, kind)| (*kind, words.len()))
    }

    /// `ALTER kind name` then a rename, an owner, a schema, or the kind's own action.
    pub(crate) fn alter_object(&mut self, kind: &'static str, words: usize) -> Parsing<Expression> {
        self.at += 1 + words;
        let if_exists: bool = kind == STATISTICS && self.kw("if") && self.kw_at(1, "exists");
        if if_exists {
            self.at += 2;
        }
        let name_at: usize = self.at;
        let (name, mut operands) = self.altered_object_name(kind)?;
        if kind == TYPE && self.starts_attribute_command() {
            let end: usize = self.at;
            self.at = name_at;
            self.table_name()?;
            self.at = end;
        }
        let common = match if_exists {
            true => None,
            false => self.common_object_action(kind)?,
        };
        let (action, values) = match common {
            Some(common) => common,
            None => self.object_action(kind)?,
        };
        operands.extend(values);
        Ok(alter((kind, name, if_exists), action, operands))
    }

    /// Whether an attribute command follows, whose type name gram.y reads with `makeRangeVarFromAnyName`.
    fn starts_attribute_command(&self) -> bool {
        ATTRIBUTE_COMMANDS.iter().any(|words| self.words_at(words))
    }

    /// The altered object's name, and the operands that identify it beyond its name.
    fn altered_object_name(&mut self, kind: &str) -> Parsing<(TableRef, Vec<Expression>)> {
        if kind == OPERATOR {
            return Ok((table(Vec::new()), vec![self.operator_with_argtypes()?]));
        }
        if kind == LARGE_OBJECT {
            return Ok((table(Vec::new()), vec![self.numeric_only()?]));
        }
        if !ANY_NAME_KINDS.contains(&kind) {
            let name: Identifier = self.name(is_col_id)?;
            if !ON_TABLE_KINDS.contains(&kind) {
                return Ok((table(vec![name]), Vec::new()));
            }
            self.expect_kw("on")?;
            let relation: Expression = Expression::Table(Box::new(table(self.table_name()?)));
            return Ok((table(vec![name]), vec![clause("ON", vec![relation])]));
        }
        let name: TableRef = table(self.qualified_name()?);
        if kind != OPERATOR_CLASS && kind != OPERATOR_FAMILY {
            return Ok((name, Vec::new()));
        }
        self.expect_kw("using")?;
        let method: Expression = name_from(self.name(is_col_id)?);
        Ok((name, vec![clause("USING", vec![method])]))
    }

    /// `RENAME TO name`, `OWNER TO RoleSpec` or `SET SCHEMA name`, where the kind takes it.
    fn common_object_action(
        &mut self,
        kind: &str,
    ) -> Parsing<Option<(&'static str, Vec<Expression>)>> {
        if self.kw("rename") && self.kw_at(1, "to") && !UNRENAMED_KINDS.contains(&kind) {
            self.at += 2;
            return Ok(Some(("RENAME", vec![name_from(self.name(is_col_id)?)])));
        }
        if self.kw("owner") && self.kw_at(1, "to") && !UNOWNED_KINDS.contains(&kind) {
            self.at += 2;
            return Ok(Some(("OWNER TO", vec![self.role_spec()?])));
        }
        if self.kw("set") && self.kw_at(1, "schema") && SCHEMA_KINDS.contains(&kind) {
            self.at += 2;
            return Ok(Some(("SET SCHEMA", vec![name_from(self.name(is_col_id)?)])));
        }
        Ok(None)
    }

    /// The action only `kind` takes.
    fn object_action(&mut self, kind: &str) -> Parsing<(&'static str, Vec<Expression>)> {
        match kind {
            STATISTICS => {
                self.expect_kw("set")?;
                self.expect_kw("statistics")?;
                Ok(("SET STATISTICS", vec![self.statistics_value()?]))
            }
            COLLATION => {
                self.expect_kw("refresh")?;
                self.expect_kw("version")?;
                Ok(("REFRESH VERSION", Vec::new()))
            }
            DATABASE => self.database_action(),
            DOMAIN => self.domain_action(),
            TYPE => self.type_action(),
            EXTENSION => self.extension_action(),
            TABLESPACE => {
                let action: &str = match self.eat_kw("set") {
                    true => "SET",
                    false => {
                        self.expect_kw("reset")?;
                        "RESET"
                    }
                };
                Ok((action, self.options(true)?))
            }
            EVENT_TRIGGER => {
                let Some((words, mode)) =
                    TRIGGER_MODES.iter().find(|(words, _)| self.words_at(words))
                else {
                    return Err(self.error());
                };
                self.at += words.len();
                Ok((mode, Vec::new()))
            }
            TRIGGER => self.depends_action(),
            OPERATOR => {
                self.expect_kw("set")?;
                Ok(("SET", self.operator_definitions()?))
            }
            OPERATOR_FAMILY => self.operator_family_action(),
            TS_DICTIONARY => Ok(("OPTIONS", self.options(false)?)),
            TS_CONFIGURATION => self.mapping_action(),
            PUBLICATION => self.publication_action(),
            SUBSCRIPTION => self.subscription_action(),
            FOREIGN_DATA_WRAPPER => self.foreign_action(true),
            SERVER => self.foreign_action(false),
            _ => Err(self.error()),
        }
    }

    /// `[NO] DEPENDS ON EXTENSION name`.
    fn depends_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        let action: &str = match self.eat_kw("no") {
            true => "NO DEPENDS ON EXTENSION",
            false => "DEPENDS ON EXTENSION",
        };
        self.expect_kw("depends")?;
        self.expect_kw("on")?;
        self.expect_kw("extension")?;
        Ok((action, vec![name_from(self.name(is_col_id)?)]))
    }

    /// `ALTER DATABASE name` then a tablespace, a collation refresh, a setting, or options.
    fn database_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        let moved: bool =
            self.kw("set") && self.kw_at(1, "tablespace") && self.peek_at(2).is_some_and(is_col_id);
        if moved {
            self.at += 2;
            return Ok(("SET TABLESPACE", vec![name_from(self.name(is_col_id)?)]));
        }
        if self.kw("refresh") && self.kw_at(1, "collation") {
            self.at += 2;
            self.expect_kw("version")?;
            return Ok(("REFRESH COLLATION VERSION", Vec::new()));
        }
        if self.kw("set") || self.kw("reset") {
            return Ok(("", vec![self.set_reset_clause()?]));
        }
        self.eat_kw("with");
        Ok((DATABASE_OPTIONS_ACTION, self.database_options()?))
    }

    /// `AlterDomainStmt`: a default, `NOT NULL`, or a constraint added, dropped, validated or renamed.
    fn domain_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        let Some(word) = self.keyword_at(0) else {
            return Err(self.error());
        };
        self.at += 1;
        match word {
            "set" if self.eat_kw("default") => Ok(("SET DEFAULT", vec![self.a_expr()?])),
            "drop" if self.eat_kw("default") => Ok(("DROP DEFAULT", Vec::new())),
            "set" | "drop" if self.kw("not") => {
                self.at += 1;
                self.expect_kw("null")?;
                Ok(match word {
                    "set" => ("SET NOT NULL", Vec::new()),
                    _ => ("DROP NOT NULL", Vec::new()),
                })
            }
            "drop" => {
                self.expect_kw("constraint")?;
                let if_exists: bool = self.kw("if") && self.kw_at(1, "exists");
                if if_exists {
                    self.at += 2;
                }
                let mut values: Vec<Expression> = vec![name_from(self.name(is_col_id)?)];
                values.extend(self.drop_behavior());
                Ok(match if_exists {
                    true => ("DROP CONSTRAINT IF EXISTS", values),
                    false => ("DROP CONSTRAINT", values),
                })
            }
            "add" => Ok(("ADD", vec![self.domain_constraint()?])),
            "validate" => {
                self.expect_kw("constraint")?;
                Ok((
                    "VALIDATE CONSTRAINT",
                    vec![name_from(self.name(is_col_id)?)],
                ))
            }
            "rename" => {
                self.expect_kw("constraint")?;
                let old: Expression = name_from(self.name(is_col_id)?);
                self.expect_kw("to")?;
                Ok((
                    "RENAME CONSTRAINT",
                    vec![old, name_from(self.name(is_col_id)?)],
                ))
            }
            _ => {
                self.at -= 1;
                Err(self.error())
            }
        }
    }

    /// `DomainConstraint`: `[CONSTRAINT name]` then `CHECK (a_expr)` or `NOT NULL`, with attributes.
    fn domain_constraint(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Expression> = Vec::new();
        if self.eat_kw("constraint") {
            parts.push(clause("CONSTRAINT", vec![name_from(self.name(is_col_id)?)]));
        }
        let check: bool = self.eat_kw("check");
        let mut values: Vec<Expression> = Vec::new();
        if check {
            values.push(self.parenthesized_condition()?);
        } else {
            self.expect_kw("not")?;
            self.expect_kw("null")?;
        }
        let start: usize = self.offset();
        let (bits, attributes) = self.constraint_attributes()?;
        let (keyword, allowed): (&str, u8) = match check {
            true => ("CHECK", NOT_VALID | NO_INHERIT),
            false => ("NOT NULL", 0),
        };
        check_attributes(keyword, allowed, bits, start)?;
        values.extend(attributes);
        parts.push(clause(keyword, values));
        Ok(tuple(parts))
    }

    /// `ALTER TYPE name` then enum values, attribute commands, or `SET ( operator_def_list )`.
    fn type_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        if self.kw("add") && self.kw_at(1, "value") {
            self.at += 2;
            let if_not_exists: bool = self.kw("if") && self.kw_at(1, "not");
            if if_not_exists {
                self.at += 2;
                self.expect_kw("exists")?;
            }
            let mut values: Vec<Expression> = vec![self.sconst()?];
            let neighbour: Option<&str> = match self.eat_kw("before") {
                true => Some("BEFORE"),
                false if self.eat_kw("after") => Some("AFTER"),
                false => None,
            };
            if let Some(neighbour) = neighbour {
                values.push(clause(neighbour, vec![self.sconst()?]));
            }
            return Ok(match if_not_exists {
                true => ("ADD VALUE IF NOT EXISTS", values),
                false => ("ADD VALUE", values),
            });
        }
        if self.kw("rename") && self.kw_at(1, "value") {
            self.at += 2;
            let old: Expression = self.sconst()?;
            self.expect_kw("to")?;
            return Ok(("RENAME VALUE", vec![old, self.sconst()?]));
        }
        if self.kw("drop") && self.kw_at(1, "value") {
            let start: usize = self.offset();
            self.at += 2;
            self.sconst()?;
            return Err(unsupported(DROP_VALUE.to_owned(), start));
        }
        if self.kw("rename") && self.kw_at(1, "attribute") {
            self.at += 2;
            let old: Expression = name_from(self.name(is_col_id)?);
            self.expect_kw("to")?;
            let mut values: Vec<Expression> = vec![old, name_from(self.name(is_col_id)?)];
            values.extend(self.drop_behavior());
            return Ok(("RENAME ATTRIBUTE", values));
        }
        if self.kw("set") && self.char_at(1, '(') {
            self.at += 1;
            return Ok(("SET", self.operator_definitions()?));
        }
        Ok(("", self.comma_list(|parser| parser.type_command())?))
    }

    /// One `alter_type_cmd`: `ADD`, `DROP` or `ALTER ATTRIBUTE`.
    fn type_command(&mut self) -> Parsing<Expression> {
        let Some(word) = self.keyword_at(0) else {
            return Err(self.error());
        };
        self.at += 1;
        self.expect_kw("attribute")?;
        let mut values: Vec<Expression> = Vec::new();
        let keyword: &str = match word {
            "add" => {
                values.push(name_from(self.name(is_col_id)?));
                values.push(Expression::DataType(self.typename()?));
                values.extend(self.collate_clause()?);
                "ADD ATTRIBUTE"
            }
            "drop" => {
                let if_exists: bool = self.kw("if") && self.kw_at(1, "exists");
                if if_exists {
                    self.at += 2;
                }
                values.push(name_from(self.name(is_col_id)?));
                match if_exists {
                    true => "DROP ATTRIBUTE IF EXISTS",
                    false => "DROP ATTRIBUTE",
                }
            }
            "alter" => {
                values.push(name_from(self.name(is_col_id)?));
                if self.eat_kw("set") {
                    self.expect_kw("data")?;
                }
                self.expect_kw("type")?;
                values.push(Expression::DataType(self.typename()?));
                values.extend(self.collate_clause()?);
                "ALTER ATTRIBUTE"
            }
            _ => {
                self.at -= 2;
                return Err(self.error());
            }
        };
        values.extend(self.drop_behavior());
        Ok(clause(keyword, values))
    }

    /// `operator_def_list`: `( name [= arg | = NONE], ... )`, where `NONE` and a bare name have no argument.
    fn operator_definitions(&mut self) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| {
            let mut values: Vec<Expression> = vec![name_value(&[parser.name(is_col_label)?])];
            if parser.eat_char('=') && !parser.eat_kw("none") {
                values.push(parser.definition_value()?);
            }
            Ok(tuple(values))
        })
    }

    /// `opt_collate_clause`: `COLLATE any_name`, if written.
    fn collate_clause(&mut self) -> Parsing<Option<Expression>> {
        match self.eat_kw("collate") {
            true => Ok(Some(clause(
                "COLLATE",
                vec![name_value(&self.qualified_name()?)],
            ))),
            false => Ok(None),
        }
    }

    /// `ALTER EXTENSION name` then `UPDATE [TO version]...`, or `ADD` or `DROP` a member object.
    fn extension_action(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        if self.eat_kw("update") {
            let mut values: Vec<Expression> = Vec::new();
            while self.eat_kw("to") {
                values.push(clause("TO", vec![self.word_or_string()?]));
            }
            return Ok(("UPDATE", values));
        }
        let action: &str = match self.eat_kw("add") {
            true => "ADD",
            false => {
                self.expect_kw("drop")?;
                "DROP"
            }
        };
        if self
            .keyword_at(0)
            .is_some_and(|word| UNMEMBER_KINDS.contains(&word))
        {
            return Err(self.error());
        }
        let (kind, this) = self.commented_object()?;
        Ok((action, vec![var(&kind), this]))
    }
}
