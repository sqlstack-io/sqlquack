//! `AlterTableStmt` and the statements that rename, move or re-own a relation (gram.y), as `AlterObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::AlterObject;

use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::{
    DEFERRABLE, ENFORCED, NO_INHERIT, NOT_VALID, check_attributes, clause, name_value, unsupported,
};
use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_col_id, is_col_label, is_non_reserved_word,
};
use crate::parse::_helpers::tree::nodes::{literal, table, tuple};
use crate::parse::errors::ParseError;
use crate::scan::main::digits::digits_value;
use crate::scan::models::TokenKind;

/// Relation kinds `AlterTableStmt` alters, longest spelling first, with their kind text.
const RELATION_KINDS: [(&[&str], &str); 6] = [
    (&["materialized", "view"], "MATERIALIZED VIEW"),
    (&["foreign", "table"], "FOREIGN TABLE"),
    (&["table"], TABLE_KIND),
    (&["index"], INDEX_KIND),
    (&["sequence"], SEQUENCE_KIND),
    (&["view"], "VIEW"),
];

const TABLE_KIND: &str = "TABLE";
const FOREIGN_TABLE_KIND: &str = "FOREIGN TABLE";
const INDEX_KIND: &str = "INDEX";
const SEQUENCE_KIND: &str = "SEQUENCE";
const MATERIALIZED_VIEW_KIND: &str = "MATERIALIZED VIEW";

/// Kinds `ALTER ... ALL IN TABLESPACE` moves.
const MOVE_ALL_KINDS: [&str; 3] = [TABLE_KIND, INDEX_KIND, MATERIALIZED_VIEW_KIND];

/// Kinds `RENAME [COLUMN] name TO name` renames a column of.
const COLUMN_RENAME_KINDS: [&str; 4] = [
    TABLE_KIND,
    FOREIGN_TABLE_KIND,
    "VIEW",
    MATERIALIZED_VIEW_KIND,
];

/// Kinds `SET SCHEMA` moves.
const SCHEMA_KINDS: [&str; 5] = [
    TABLE_KIND,
    FOREIGN_TABLE_KIND,
    SEQUENCE_KIND,
    "VIEW",
    MATERIALIZED_VIEW_KIND,
];

/// Kinds that may depend on an extension.
const DEPENDS_KINDS: [&str; 2] = [INDEX_KIND, MATERIALIZED_VIEW_KIND];

/// Words that begin a `SeqOptElem`, so `ALTER SEQUENCE` takes a `SeqOptList`.
const SEQUENCE_OPTION_WORDS: [&str; 12] = [
    "as",
    "cache",
    "cycle",
    "increment",
    "logged",
    "maxvalue",
    "minvalue",
    "owned",
    "sequence",
    "start",
    "restart",
    "unlogged",
];

/// Words after `SET` that begin an identity column option rather than another column action.
const IDENTITY_SET_WORDS: [&str; 13] = [
    "generated",
    "as",
    "cache",
    "cycle",
    "no",
    "increment",
    "logged",
    "maxvalue",
    "minvalue",
    "owned",
    "sequence",
    "start",
    "unlogged",
];

/// `SeqOptElem`s an identity column's `SET` refuses (42601), with the `defname` the error names.
const IDENTITY_REFUSED: [(&str, &str); 3] =
    [("as", "as"), ("restart", "restart"), ("owned", "owned_by")];

/// Words that begin the object kind of an `ALTER` statement, each with the words that may follow it.
const ALTER_KINDS: [(&str, &[&str]); 36] = [
    ("aggregate", &[]),
    ("collation", &[]),
    ("conversion", &[]),
    ("database", &[]),
    ("default", &["privileges"]),
    ("domain", &[]),
    ("event", &["trigger"]),
    ("extension", &[]),
    ("foreign", &["table", "data"]),
    ("function", &[]),
    ("group", &[]),
    ("index", &[]),
    ("language", &[]),
    ("large", &["object"]),
    ("materialized", &["view"]),
    ("operator", &[]),
    ("policy", &[]),
    ("procedural", &["language"]),
    ("procedure", &[]),
    ("publication", &[]),
    ("role", &[]),
    ("routine", &[]),
    ("rule", &[]),
    ("schema", &[]),
    ("sequence", &[]),
    ("server", &[]),
    ("statistics", &[]),
    ("subscription", &[]),
    ("system", &[]),
    ("table", &[]),
    ("tablespace", &[]),
    ("text", &["search"]),
    ("trigger", &[]),
    ("type", &[]),
    ("user", &[]),
    ("view", &[]),
];

/// The largest column number `SET STATISTICS` takes (`PG_INT16_MAX`).
const MAX_COLUMN_NUMBER: i64 = 32767;

/// `RoleSpec` words that name a role kind rather than a role.
const ROLE_WORDS: [&str; 3] = ["current_role", "current_user", "session_user"];

/// The trigger modes that also take `ROW LEVEL SECURITY`, `ALL` and `USER`.
const PLAIN_MODES: [&str; 2] = ["ENABLE", "DISABLE"];

/// The role name `RoleSpec` refuses (42939).
const RESERVED_ROLE: &str = "none";

/// The role name `RoleSpec` reads as every role.
const PUBLIC_ROLE: &str = "public";

/// An `AlterObject` of `kind` naming `name`.
fn alter(target: (&str, TableRef, bool), action: &str, operands: Vec<Expression>) -> Expression {
    let (kind, name, if_exists) = target;
    Expression::AlterObject(Box::new(AlterObject {
        kind: kind.to_owned(),
        name,
        arguments: Vec::new(),
        if_exists,
        action: action.to_owned(),
        operands,
        properties: Vec::new(),
    }))
}

/// The value of an `ICONST` token's text, in its decimal, hexadecimal, octal or binary spelling.
fn iconst(text: &str) -> i64 {
    let lower: String = text.to_ascii_lowercase();
    let (digits, radix): (&str, u32) = match lower.get(..2) {
        Some("0x") => (&lower[2..], 16),
        Some("0o") => (&lower[2..], 8),
        Some("0b") => (&lower[2..], 2),
        _ => (&lower[..], 10),
    };
    digits_value(digits, radix).map_or(i64::MAX, i64::from)
}

impl Parser<'_> {
    /// The relation kind `ALTER` alters here, if it is one `AlterTableStmt` names.
    pub(crate) fn altered_relation(&self) -> Option<&'static str> {
        if !self.kw("alter") {
            return None;
        }
        RELATION_KINDS
            .iter()
            .find(|(words, _)| self.kws_after_alter(words))
            .map(|(_, kind)| *kind)
    }

    /// Whether `words` follow `ALTER` here.
    fn kws_after_alter(&self, words: &[&str]) -> bool {
        for (index, word) in words.iter().enumerate() {
            if !self.kw_at(index + 1, word) {
                return false;
            }
        }
        true
    }

    /// The offset of the token after `ALTER` that no `ALTER` statement kind allows, if there is one.
    pub(crate) fn unknown_alter_kind(&self) -> Option<usize> {
        if !self.kw("alter") {
            return None;
        }
        let Some((_, follows)) = ALTER_KINDS.iter().find(|(word, _)| self.kw_at(1, word)) else {
            return Some(1);
        };
        match follows.is_empty() || follows.iter().any(|word| self.kw_at(2, word)) {
            true => None,
            false => Some(2),
        }
    }

    /// `ALTER kind [IF EXISTS] name` then commands, a rename, a new schema, a partition or a move.
    pub(crate) fn alter_relation(&mut self, kind: &'static str) -> Parsing<Expression> {
        self.at += 1 + kind.split(' ').count();
        if self.kw("all") && MOVE_ALL_KINDS.contains(&kind) {
            return self.move_all(kind);
        }
        let if_exists: bool = self.kw("if") && self.kw_at(1, "exists");
        if if_exists {
            self.at += 2;
        }
        let name: TableRef = match kind {
            TABLE_KIND | FOREIGN_TABLE_KIND => self.relation()?,
            _ => table(self.table_name()?),
        };
        if self.eat_kw("rename") {
            let (action, operands) = self.relation_rename(kind)?;
            return Ok(alter((kind, name, if_exists), action, operands));
        }
        if self.kw("set") && self.kw_at(1, "schema") && SCHEMA_KINDS.contains(&kind) {
            self.at += 2;
            let schema: Expression = name_from(self.name(is_col_id)?);
            return Ok(alter((kind, name, if_exists), "SET SCHEMA", vec![schema]));
        }
        let depends: bool = (self.kw("depends") || (self.kw("no") && self.kw_at(1, "depends")))
            && DEPENDS_KINDS.contains(&kind)
            && !if_exists;
        if depends {
            let action: &str = match self.eat_kw("no") {
                true => "NO DEPENDS ON EXTENSION",
                false => "DEPENDS ON EXTENSION",
            };
            self.expect_kw("depends")?;
            self.expect_kw("on")?;
            self.expect_kw("extension")?;
            let extension: Expression = name_from(self.name(is_col_id)?);
            return Ok(alter((kind, name, if_exists), action, vec![extension]));
        }
        let partition: bool = match kind {
            TABLE_KIND => self.kw("attach") || self.kw("detach"),
            INDEX_KIND => self.kw("attach") && !if_exists,
            _ => false,
        };
        if partition {
            let (action, operands) = self.partition_command(kind)?;
            return Ok(alter((kind, name, if_exists), action, operands));
        }
        let sequence_options: bool = kind == SEQUENCE_KIND
            && match self.keyword_at(0) {
                Some("no") => matches!(self.keyword_at(1), Some("cycle" | "maxvalue" | "minvalue")),
                Some(word) => SEQUENCE_OPTION_WORDS.contains(&word),
                None => false,
            };
        if sequence_options {
            let mut options: Vec<Expression> = vec![self.sequence_option()?];
            while self
                .keyword_at(0)
                .is_some_and(|word| SEQUENCE_OPTION_WORDS.contains(&word))
                || self.kw("no")
            {
                options.push(self.sequence_option()?);
            }
            return Ok(alter(
                (kind, name, if_exists),
                "SET SEQUENCE OPTIONS",
                options,
            ));
        }
        let commands: Vec<Expression> = self.comma_list(|parser| parser.alter_table_command())?;
        Ok(alter((kind, name, if_exists), "", commands))
    }

    /// `ALTER kind ALL IN TABLESPACE name [OWNED BY role, ...] SET TABLESPACE name [NOWAIT]`.
    fn move_all(&mut self, kind: &'static str) -> Parsing<Expression> {
        self.expect_kw("all")?;
        self.expect_kw("in")?;
        self.expect_kw("tablespace")?;
        let from: Identifier = self.name(is_col_id)?;
        let mut operands: Vec<Expression> = Vec::new();
        if self.eat_kw("owned") {
            self.expect_kw("by")?;
            let roles: Vec<Expression> = self.comma_list(|parser| parser.role_spec())?;
            operands.push(clause("OWNED BY", roles));
        }
        self.expect_kw("set")?;
        self.expect_kw("tablespace")?;
        operands.push(clause(
            "SET TABLESPACE",
            vec![name_from(self.name(is_col_id)?)],
        ));
        if self.eat_kw("nowait") {
            operands.push(var("NOWAIT"));
        }
        Ok(alter(
            (kind, table(vec![from]), false),
            "ALL IN TABLESPACE",
            operands,
        ))
    }

    /// After `RENAME`: `TO name`, `[COLUMN] name TO name` or `CONSTRAINT name TO name`.
    fn relation_rename(&mut self, kind: &str) -> Parsing<(&'static str, Vec<Expression>)> {
        if self.eat_kw("to") {
            return Ok(("RENAME", vec![name_from(self.name(is_col_id)?)]));
        }
        let action: &'static str = match self.kw("constraint") && kind == TABLE_KIND {
            true => {
                self.at += 1;
                "RENAME CONSTRAINT"
            }
            false if COLUMN_RENAME_KINDS.contains(&kind) => {
                self.eat_kw("column");
                "RENAME COLUMN"
            }
            false => return Err(self.error()),
        };
        let old: Expression = name_from(self.name(is_col_id)?);
        self.expect_kw("to")?;
        Ok((action, vec![old, name_from(self.name(is_col_id)?)]))
    }

    /// `partition_cmd` or `index_partition_cmd`.
    fn partition_command(&mut self, kind: &str) -> Parsing<(&'static str, Vec<Expression>)> {
        let attach: bool = self.eat_kw("attach");
        if !attach {
            self.expect_kw("detach")?;
        }
        self.expect_kw("partition")?;
        let partition: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        if attach {
            let mut operands: Vec<Expression> = vec![partition];
            if kind == TABLE_KIND {
                operands.push(self.partition_bound()?);
            }
            return Ok(("ATTACH PARTITION", operands));
        }
        let mut operands: Vec<Expression> = vec![partition];
        if self.eat_kw("concurrently") {
            operands.push(var("CONCURRENTLY"));
        } else if self.eat_kw("finalize") {
            operands.push(var("FINALIZE"));
        }
        Ok(("DETACH PARTITION", operands))
    }

    /// One `alter_table_cmd`, as its keywords and values.
    fn alter_table_command(&mut self) -> Parsing<Expression> {
        let Some(word) = self.keyword_at(0) else {
            return Err(self.error());
        };
        match word {
            "add" => self.add_command(),
            "alter" if self.kw_at(1, "constraint") => self.alter_constraint(),
            "alter" => {
                self.at += 1;
                self.eat_kw("column");
                self.alter_column()
            }
            "drop" => self.drop_command(),
            "validate" => {
                self.at += 1;
                self.expect_kw("constraint")?;
                Ok(clause(
                    "VALIDATE CONSTRAINT",
                    vec![name_from(self.name(is_col_id)?)],
                ))
            }
            "set" => {
                self.at += 1;
                self.table_set()
            }
            "reset" => {
                self.at += 1;
                Ok(clause("RESET", self.options(true)?))
            }
            "cluster" => {
                self.at += 1;
                self.expect_kw("on")?;
                Ok(clause("CLUSTER ON", vec![name_from(self.name(is_col_id)?)]))
            }
            "enable" | "disable" => self.enable_command(),
            "force" => {
                self.at += 1;
                self.row_level_security("FORCE ROW LEVEL SECURITY")
            }
            "inherit" => {
                self.at += 1;
                Ok(clause("INHERIT", vec![self.relation_name()?]))
            }
            "no" if self.kw_at(1, "inherit") => {
                self.at += 2;
                Ok(clause("NO INHERIT", vec![self.relation_name()?]))
            }
            "no" if self.kw_at(1, "force") => {
                self.at += 2;
                self.row_level_security("NO FORCE ROW LEVEL SECURITY")
            }
            "of" => {
                self.at += 1;
                let type_name = self.qualified_name()?;
                Ok(clause("OF", vec![name_value(&type_name)]))
            }
            "not" if self.kw_at(1, "of") => {
                self.at += 2;
                Ok(clause("NOT OF", Vec::new()))
            }
            "owner" => {
                self.at += 1;
                self.expect_kw("to")?;
                Ok(clause("OWNER TO", vec![self.role_spec()?]))
            }
            "replica" => self.replica_identity(),
            "options" => Ok(clause("OPTIONS", self.generic_options()?)),
            _ => Err(self.error()),
        }
    }

    /// `ADD [COLUMN] [IF NOT EXISTS] columnDef` or `ADD TableConstraint`.
    fn add_command(&mut self) -> Parsing<Expression> {
        self.at += 1;
        if self.starts_table_constraint() {
            return Ok(clause("ADD", vec![self.table_constraint()?]));
        }
        self.eat_kw("column");
        let if_not_exists: bool = self.kw("if") && self.kw_at(1, "not");
        if if_not_exists {
            self.at += 2;
            self.expect_kw("exists")?;
        }
        let column = self.column_definition(false)?;
        let keyword: &str = match if_not_exists {
            true => "ADD COLUMN IF NOT EXISTS",
            false => "ADD COLUMN",
        };
        Ok(clause(
            keyword,
            vec![Expression::ColumnDef(Box::new(column))],
        ))
    }

    /// `ALTER CONSTRAINT name` then `INHERIT` or constraint attributes.
    fn alter_constraint(&mut self) -> Parsing<Expression> {
        self.at += 2;
        let constraint: Expression = name_from(self.name(is_col_id)?);
        if self.eat_kw("inherit") {
            return Ok(clause("ALTER CONSTRAINT", vec![constraint, var("INHERIT")]));
        }
        let start: usize = self.offset();
        let (bits, attributes) = self.constraint_attributes()?;
        if bits & NOT_VALID != 0 {
            return Err(unsupported(
                "constraints cannot be altered to be NOT VALID".to_owned(),
                start,
            ));
        }
        check_attributes(
            "FOREIGN KEY",
            DEFERRABLE | ENFORCED | NO_INHERIT,
            bits,
            start,
        )?;
        let mut values: Vec<Expression> = vec![constraint];
        values.extend(attributes);
        Ok(clause("ALTER CONSTRAINT", values))
    }

    /// `DROP [COLUMN] [IF EXISTS] name` or `DROP CONSTRAINT [IF EXISTS] name`, then a drop behaviour.
    fn drop_command(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let constraint: bool = self.eat_kw("constraint");
        if !constraint {
            self.eat_kw("column");
        }
        let if_exists: bool = self.kw("if") && self.kw_at(1, "exists");
        if if_exists {
            self.at += 2;
        }
        let keyword: &str = match (constraint, if_exists) {
            (true, true) => "DROP CONSTRAINT IF EXISTS",
            (true, false) => "DROP CONSTRAINT",
            (false, true) => "DROP COLUMN IF EXISTS",
            (false, false) => "DROP COLUMN",
        };
        let mut values: Vec<Expression> = vec![name_from(self.name(is_col_id)?)];
        values.extend(self.drop_behavior());
        Ok(clause(keyword, values))
    }

    /// `REPLICA IDENTITY` then `NOTHING`, `FULL`, `DEFAULT` or `USING INDEX name`.
    fn replica_identity(&mut self) -> Parsing<Expression> {
        self.at += 1;
        self.expect_kw("identity")?;
        let identity: Expression = match self.keyword_at(0) {
            Some("nothing") => var("NOTHING"),
            Some("full") => var("FULL"),
            Some("default") => var("DEFAULT"),
            Some("using") => {
                self.at += 1;
                self.expect_kw("index")?;
                let index: Expression = name_from(self.name(is_col_id)?);
                return Ok(clause(
                    "REPLICA IDENTITY",
                    vec![clause("USING INDEX", vec![index])],
                ));
            }
            _ => return Err(self.error()),
        };
        self.at += 1;
        Ok(clause("REPLICA IDENTITY", vec![identity]))
    }

    /// `ROW LEVEL SECURITY` after `ENABLE`, `DISABLE`, `FORCE` or `NO FORCE`.
    fn row_level_security(&mut self, keyword: &str) -> Parsing<Expression> {
        self.expect_kw("row")?;
        self.expect_kw("level")?;
        self.expect_kw("security")?;
        Ok(clause(keyword, Vec::new()))
    }

    /// The `SET` commands of `alter_table_cmd`, after `SET`.
    fn table_set(&mut self) -> Parsing<Expression> {
        if self.is_char('(') {
            return Ok(clause("SET", self.options(true)?));
        }
        match self.keyword_at(0) {
            Some("without") => {
                self.at += 1;
                match self.eat_kw("oids") {
                    true => Ok(clause("SET WITHOUT OIDS", Vec::new())),
                    false => {
                        self.expect_kw("cluster")?;
                        Ok(clause("SET WITHOUT CLUSTER", Vec::new()))
                    }
                }
            }
            Some("logged") => {
                self.at += 1;
                Ok(clause("SET LOGGED", Vec::new()))
            }
            Some("unlogged") => {
                self.at += 1;
                Ok(clause("SET UNLOGGED", Vec::new()))
            }
            Some("access") => {
                self.at += 1;
                self.expect_kw("method")?;
                let method: Expression = match self.eat_kw("default") {
                    true => var("DEFAULT"),
                    false => name_from(self.name(is_col_id)?),
                };
                Ok(clause("SET ACCESS METHOD", vec![method]))
            }
            Some("tablespace") => {
                self.at += 1;
                Ok(clause(
                    "SET TABLESPACE",
                    vec![name_from(self.name(is_col_id)?)],
                ))
            }
            _ => Err(self.error()),
        }
    }

    /// `ENABLE` or `DISABLE` a trigger, a rule or row level security.
    fn enable_command(&mut self) -> Parsing<Expression> {
        let enable: bool = self.eat_kw("enable");
        if !enable {
            self.expect_kw("disable")?;
        }
        let mode: &str = match enable {
            true if self.eat_kw("always") => "ENABLE ALWAYS",
            true if self.eat_kw("replica") => "ENABLE REPLICA",
            true => "ENABLE",
            false => "DISABLE",
        };
        if self.kw("row") && PLAIN_MODES.contains(&mode) {
            return self.row_level_security(&format!("{mode} ROW LEVEL SECURITY"));
        }
        if self.eat_kw("rule") {
            return Ok(clause(
                &format!("{mode} RULE"),
                vec![name_from(self.name(is_col_id)?)],
            ));
        }
        self.expect_kw("trigger")?;
        let plain: bool = PLAIN_MODES.contains(&mode);
        let target: Expression = match self.keyword_at(0) {
            Some("all") if plain => {
                self.at += 1;
                var("ALL")
            }
            Some("user") if plain => {
                self.at += 1;
                var("USER")
            }
            _ => name_from(self.name(is_col_id)?),
        };
        Ok(clause(&format!("{mode} TRIGGER"), vec![target]))
    }

    /// `ALTER [COLUMN] column action`, after `COLUMN`: the column, then the action's tuple.
    fn alter_column(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let number: Option<i64> = match self.peek().map(|token| token.kind) {
            Some(TokenKind::Integer) => Some(iconst(&self.advance()?.value)),
            _ => None,
        };
        let column: Expression = match number {
            Some(value) => literal(Literal::Number(value.to_string())),
            None => name_from(self.name(is_col_id)?),
        };
        if let Some(value) = number {
            self.expect_kw("set")?;
            self.expect_kw("statistics")?;
            let statistics: Expression = self.statistics_value()?;
            if !(1..=MAX_COLUMN_NUMBER).contains(&value) {
                return Err(unsupported(
                    format!("column number must be in range from 1 to {MAX_COLUMN_NUMBER}"),
                    start,
                ));
            }
            return Ok(clause(
                "ALTER COLUMN",
                vec![column, clause("SET STATISTICS", vec![statistics])],
            ));
        }
        let mut values: Vec<Expression> = vec![column];
        values.extend(self.column_action()?);
        Ok(clause("ALTER COLUMN", values))
    }

    /// The action of `ALTER [COLUMN] column`, as one tuple or a list of identity options.
    fn column_action(&mut self) -> Parsing<Vec<Expression>> {
        let Some(word) = self.keyword_at(0) else {
            return Err(self.error());
        };
        let action: Expression = match word {
            "set" if self.kw_at(1, "data") || self.char_at(1, '(') => {
                self.at += 1;
                match self.is_char('(') {
                    true => clause("SET", self.options(true)?),
                    false => {
                        self.expect_kw("data")?;
                        self.column_type()?
                    }
                }
            }
            "type" => self.column_type()?,
            "set"
                if self
                    .keyword_at(1)
                    .is_some_and(|next| IDENTITY_SET_WORDS.contains(&next)) =>
            {
                return self.identity_options();
            }
            "restart" => return self.identity_options(),
            "set" => {
                self.at += 1;
                let Some(next) = self.keyword_at(0) else {
                    return Err(self.error());
                };
                self.at += 1;
                match next {
                    "default" => clause("SET DEFAULT", vec![self.a_expr()?]),
                    "not" => {
                        self.expect_kw("null")?;
                        clause("SET NOT NULL", Vec::new())
                    }
                    "expression" => {
                        self.expect_kw("as")?;
                        clause("SET EXPRESSION AS", vec![self.parenthesized_condition()?])
                    }
                    "statistics" => clause("SET STATISTICS", vec![self.statistics_value()?]),
                    "storage" | "compression" => {
                        let value: Expression = match self.eat_kw("default") {
                            true => var("DEFAULT"),
                            false => name_from(self.name(is_col_id)?),
                        };
                        clause(&format!("SET {}", next.to_uppercase()), vec![value])
                    }
                    _ => {
                        self.at -= 1;
                        return Err(self.error());
                    }
                }
            }
            "reset" => {
                self.at += 1;
                clause("RESET", self.options(true)?)
            }
            "drop" => {
                self.at += 1;
                match self.keyword_at(0) {
                    Some("default") => {
                        self.at += 1;
                        clause("DROP DEFAULT", Vec::new())
                    }
                    Some("not") => {
                        self.at += 1;
                        self.expect_kw("null")?;
                        clause("DROP NOT NULL", Vec::new())
                    }
                    Some("expression" | "identity") => {
                        let keyword: String =
                            format!("DROP {}", self.advance()?.value.to_uppercase());
                        let if_exists: bool = self.kw("if");
                        if if_exists {
                            self.at += 1;
                            self.expect_kw("exists")?;
                        }
                        match if_exists {
                            true => clause(&format!("{keyword} IF EXISTS"), Vec::new()),
                            false => clause(&keyword, Vec::new()),
                        }
                    }
                    _ => return Err(self.error()),
                }
            }
            "add" => {
                self.at += 1;
                self.expect_kw("generated")?;
                let always: bool = self.eat_kw("always");
                if !always {
                    self.expect_kw("by")?;
                    self.expect_kw("default")?;
                }
                self.expect_kw("as")?;
                self.expect_kw("identity")?;
                let mut options: Vec<Expression> = Vec::new();
                if self.eat_char('(') {
                    options.push(self.sequence_option()?);
                    while !self.is_char(')') {
                        options.push(self.sequence_option()?);
                    }
                    self.at += 1;
                }
                let keyword: &str = match always {
                    true => "ADD GENERATED ALWAYS AS IDENTITY",
                    false => "ADD GENERATED BY DEFAULT AS IDENTITY",
                };
                clause(keyword, options)
            }
            "options" => clause("OPTIONS", self.generic_options()?),
            _ => return Err(self.error()),
        };
        Ok(vec![action])
    }

    /// `TYPE Typename [COLLATE any_name] [USING a_expr]`, after any `SET DATA`.
    fn column_type(&mut self) -> Parsing<Expression> {
        self.expect_kw("type")?;
        let mut values: Vec<Expression> = vec![Expression::DataType(self.typename()?)];
        if self.eat_kw("collate") {
            let collation = self.qualified_name()?;
            values.push(clause("COLLATE", vec![name_value(&collation)]));
        }
        if self.eat_kw("using") {
            values.push(clause("USING", vec![self.a_expr()?]));
        }
        Ok(clause("SET DATA TYPE", values))
    }

    /// `alter_identity_column_option_list`: `RESTART [[WITH] n]`, `SET SeqOptElem`, `SET GENERATED ...`.
    fn identity_options(&mut self) -> Parsing<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();
        loop {
            if self.eat_kw("restart") {
                let with: bool = self.eat_kw("with");
                let numbered: bool = with
                    || self.peek().is_some_and(|token| {
                        matches!(
                            token.kind,
                            TokenKind::Integer | TokenKind::Numeric | TokenKind::Char('+' | '-')
                        )
                    });
                options.push(match numbered {
                    true => clause("RESTART", vec![self.numeric_only()?]),
                    false => clause("RESTART", Vec::new()),
                });
                continue;
            }
            let identity_set: bool = self.kw("set")
                && self
                    .keyword_at(1)
                    .is_some_and(|next| IDENTITY_SET_WORDS.contains(&next))
                || (self.kw("set") && self.kw_at(1, "restart"));
            if !identity_set {
                break;
            }
            self.at += 1;
            if self.eat_kw("generated") {
                let keyword: &str = match self.eat_kw("always") {
                    true => "SET GENERATED ALWAYS",
                    false => {
                        self.expect_kw("by")?;
                        self.expect_kw("default")?;
                        "SET GENERATED BY DEFAULT"
                    }
                };
                options.push(clause(keyword, Vec::new()));
                continue;
            }
            let start: usize = self.offset();
            let refused: Option<&str> = IDENTITY_REFUSED
                .iter()
                .find(|(word, _)| self.kw(word))
                .map(|(_, defname)| *defname);
            let option: Expression = self.sequence_option()?;
            if let Some(defname) = refused {
                return Err(self.error_with(
                    &format!("sequence option \"{defname}\" not supported here"),
                    start,
                ));
            }
            options.push(clause("SET", vec![option]));
        }
        match options.is_empty() {
            true => Err(self.error()),
            false => Ok(options),
        }
    }

    /// `set_statistics_value`: `SignedIconst` or `DEFAULT`.
    fn statistics_value(&mut self) -> Parsing<Expression> {
        if self.eat_kw("default") {
            return Ok(var("DEFAULT"));
        }
        let sign: &str = match self.peek().map(|token| token.kind) {
            Some(TokenKind::Char('-')) => "-",
            _ => "",
        };
        if self.is_char('-') || self.is_char('+') {
            self.at += 1;
        }
        let token = self.advance()?;
        if token.kind != TokenKind::Integer {
            self.at -= 1;
            return Err(self.error());
        }
        let value: i64 = iconst(&token.value);
        let signed: i64 = match sign {
            "-" => -value,
            _ => value,
        };
        Ok(literal(Literal::Number(signed.to_string())))
    }

    /// `opt_drop_behavior`; `RESTRICT` is the default, so only `CASCADE` is kept.
    pub(crate) fn drop_behavior(&mut self) -> Option<Expression> {
        if self.eat_kw("cascade") {
            return Some(var("CASCADE"));
        }
        self.eat_kw("restrict");
        None
    }

    /// A `qualified_name` as a table.
    fn relation_name(&mut self) -> Parsing<Expression> {
        Ok(Expression::Table(Box::new(table(self.table_name()?))))
    }

    /// `alter_generic_options`: `OPTIONS ( [SET | ADD | DROP] name ['value'], ... )`.
    pub(crate) fn generic_options(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_kw("options")?;
        self.parenthesized(|parser| {
            let action: Option<&str> = match parser.keyword_at(0) {
                Some("set") if parser.peek_at(1).is_some_and(is_col_label) => Some("SET"),
                Some("add") if parser.peek_at(1).is_some_and(is_col_label) => Some("ADD"),
                Some("drop") if parser.peek_at(1).is_some_and(is_col_label) => Some("DROP"),
                _ => None,
            };
            if action.is_some() {
                parser.at += 1;
            }
            let option: Expression = name_from(parser.name(is_col_label)?);
            let mut values: Vec<Expression> = vec![option];
            if action != Some("DROP") {
                values.push(parser.sconst()?);
            }
            Ok(match action {
                Some(action) => clause(action, values),
                None => tuple(values),
            })
        })
    }

    /// `RoleSpec`: a role name, `PUBLIC`, or `CURRENT_ROLE`, `CURRENT_USER` or `SESSION_USER`.
    pub(crate) fn role_spec(&mut self) -> Parsing<Expression> {
        if let Some(word) = self.keyword_at(0).filter(|word| ROLE_WORDS.contains(word)) {
            self.at += 1;
            return Ok(var(&word.to_uppercase()));
        }
        let start: usize = self.offset();
        let token = self.expect_token(is_non_reserved_word)?;
        let role: Identifier = identifier(&token);
        match role.name.as_str() {
            PUBLIC_ROLE => Ok(var(&PUBLIC_ROLE.to_uppercase())),
            RESERVED_ROLE => Err(ParseError {
                message: format!("role name \"{RESERVED_ROLE}\" is reserved"),
                position: start,
                syntax: false,
            }),
            _ => Ok(name_from(role)),
        }
    }
}

/// A single name as an expression.
fn name_from(identifier: Identifier) -> Expression {
    Expression::Identifier(identifier)
}
