//! `CreateStmt` and `CreateAsStmt` (gram.y): `CREATE [TEMP | UNLOGGED] TABLE`, clauses kept as keyword tuples.

use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::tables::models::CreateTable;
use sqlquack_syntax::{DataType, Expression};

use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, is_col_id, is_col_label, is_non_reserved_word, is_reserved,
    is_type_function_name,
};
use crate::parse::_helpers::tree::nodes::{
    literal, name_text, paren, table, tuple, type_name_text,
};
use crate::parse::_helpers::values::expressions::{is_operator_token, operator_text};
use crate::parse::constants::{JSON_CALLS, VALUE_FUNCTIONS, XML_CALLS};
use crate::parse::errors::ParseError;
use crate::scan::models::TokenKind;

/// `CONSTRAINT name` before a constraint, as the first item of its tuple.
const CONSTRAINT: &str = "CONSTRAINT";

/// The tuple tag of a table element other than a column, followed by the number of columns before it.
const ELEMENT: &str = "ELEMENT";

/// The marker `UNLOGGED` sets in `table_modifier`.
const UNLOGGED: &str = "UNLOGGED";

/// `ConstraintAttributeElem` bits, as gram.y's `CAS_` flags.
pub(crate) const NOT_DEFERRABLE: u8 = 1;
pub(crate) const DEFERRABLE: u8 = 2;
pub(crate) const INITIALLY_IMMEDIATE: u8 = 4;
pub(crate) const INITIALLY_DEFERRED: u8 = 8;
pub(crate) const NOT_VALID: u8 = 16;
pub(crate) const NO_INHERIT: u8 = 32;
pub(crate) const NOT_ENFORCED: u8 = 64;
pub(crate) const ENFORCED: u8 = 128;

/// The `processCASbits` checks in order: the bits, what they mark, and the bit a constraint must allow.
const CAS_CHECKS: [(u8, &str, u8); 6] = [
    (DEFERRABLE | INITIALLY_DEFERRED, "DEFERRABLE", DEFERRABLE),
    (INITIALLY_DEFERRED, "DEFERRABLE", DEFERRABLE),
    (NOT_VALID, "NOT VALID", NOT_VALID),
    (NO_INHERIT, "NO INHERIT", NO_INHERIT),
    (NOT_ENFORCED, "NOT ENFORCED", ENFORCED),
    (ENFORCED, "ENFORCED", ENFORCED),
];

/// Words that begin an `OptTemp`; a second one after the first is a syntax error.
const TEMP_WORDS: [&str; 5] = ["temporary", "temp", "unlogged", "local", "global"];

/// `PARTITION BY` strategies `parsePartitionStrategy` knows.
const PARTITION_STRATEGIES: [&str; 3] = ["list", "range", "hash"];

/// Keywords that begin a `TableLikeOption`.
const LIKE_OPTIONS: [&str; 10] = [
    "comments",
    "compression",
    "constraints",
    "defaults",
    "identity",
    "generated",
    "indexes",
    "statistics",
    "storage",
    "all",
];

/// Keywords that begin a `SeqOptElem`.
pub(crate) const SEQUENCE_OPTIONS: [&str; 13] = [
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
    "restart",
    "unlogged",
];

/// Keywords that begin a `func_expr_common_subexpr` an index or partition key may use bare.
const KEY_CALLS: [&str; 2] = ["cast", "collation"];

/// A keyword tuple: the keyword, then its values.
pub(crate) fn clause(keyword: &str, values: Vec<Expression>) -> Expression {
    let mut parts: Vec<Expression> = vec![var(keyword)];
    parts.extend(values);
    tuple(parts)
}

/// A dotted name as one identifier, a part holding `.` or `"` quoted.
pub(crate) fn name_value(parts: &[Identifier]) -> Expression {
    Expression::Identifier(Identifier::new(name_text(parts)))
}

fn number(text: String) -> Expression {
    literal(Literal::Number(text))
}

/// An error PostgreSQL's grammar actions raise with a SQLSTATE other than 42601.
pub(crate) fn unsupported(message: String, position: usize) -> ParseError {
    ParseError {
        message,
        position,
        syntax: false,
    }
}

/// `processCASbits`: refuses the attributes a `kind` constraint cannot take (0A000).
pub(crate) fn check_attributes(kind: &str, allowed: u8, bits: u8, start: usize) -> Parsing<()> {
    match CAS_CHECKS
        .iter()
        .find(|(mask, _, needed)| bits & mask != 0 && allowed & needed == 0)
    {
        Some((_, marked, _)) => Err(unsupported(
            format!("{kind} constraints cannot be marked {marked}"),
            start,
        )),
        None => Ok(()),
    }
}

impl Parser<'_> {
    /// Whether `CREATE [OptTemp] TABLE` comes next.
    pub(crate) fn creates_table(&self) -> bool {
        let skip: usize = match self.keyword_at(1) {
            Some("temporary" | "temp" | "unlogged") => 2,
            Some("local" | "global") if !self.kw_at(2, "table") => 3,
            Some("local" | "global") => 2,
            _ => 1,
        };
        let repeated: bool = skip > 1
            && self
                .keyword_at(skip)
                .is_some_and(|word| TEMP_WORDS.contains(&word));
        self.kw("create") && (self.kw_at(skip, "table") || repeated)
    }

    /// `CREATE OptTemp TABLE [IF NOT EXISTS] name ...`, as `CreateStmt` or `CreateAsStmt`.
    pub(crate) fn create_table(&mut self) -> Parsing<Expression> {
        self.expect_kw("create")?;
        let mut node: CreateTable = CreateTable::new(String::new());
        match self.keyword_at(0) {
            Some("unlogged") => {
                self.at += 1;
                node.table_modifier = Some(UNLOGGED.to_owned());
            }
            Some("local" | "global") => {
                self.at += 1;
                if !(self.eat_kw("temporary") || self.eat_kw("temp")) {
                    return Err(self.error());
                }
                node.temporary = true;
            }
            Some("temporary" | "temp") => {
                self.at += 1;
                node.temporary = true;
            }
            _ => {}
        }
        self.expect_kw("table")?;
        if self.kw("if") && self.kw_at(1, "not") {
            self.at += 2;
            self.expect_kw("exists")?;
            node.if_not_exists = true;
        }
        node.name = table(self.table_name()?);
        let as_query = match self.attempt(|parser| parser.create_table_as(node.clone())) {
            Ok(created) => return Ok(created),
            Err(failed) => failed,
        };
        match self.attempt(|parser| parser.create_table_body(node)) {
            Ok(created) => Ok(created),
            Err((error, reached)) if reached >= as_query.1 => Err(error),
            Err(_) => Err(as_query.0),
        }
    }

    /// `create_as_target AS SelectStmt | EXECUTE name [(expr_list)] opt_with_data`, after the name.
    fn create_table_as(&mut self, mut node: CreateTable) -> Parsing<Expression> {
        if self.is_char('(') {
            for name in self.name_list()? {
                let mut column: ColumnDef = ColumnDef::new(name.name.clone(), DataType::Unknown);
                column.name = name;
                column.no_type = true;
                node.columns.push(column);
            }
        }
        node = self.table_options(node)?;
        self.expect_kw("as")?;
        node.as_select = Some(match self.eat_kw("execute") {
            true => {
                let mut values: Vec<Expression> =
                    vec![Expression::Identifier(self.name(is_col_id)?)];
                if self.is_char('(') {
                    values.extend(self.parenthesized(|parser| parser.a_expr())?);
                }
                clause("EXECUTE", values)
            }
            false => match self.is_char('(') || self.starts_query_at(0) {
                true => self.query()?,
                false => return Err(self.error()),
            },
        });
        if self.eat_kw("with") {
            node.with_data = Some(!self.eat_kw("no"));
            self.expect_kw("data")?;
        }
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// The `CreateStmt` forms after the name: a column list, `OF type` or `PARTITION OF parent`.
    fn create_table_body(&mut self, mut node: CreateTable) -> Parsing<Expression> {
        let typed: bool = match self.keyword_at(0) {
            Some("of") => {
                self.at += 1;
                let name: Vec<Identifier> = self.qualified_name()?;
                node.properties.push(clause(
                    "OF",
                    vec![Expression::DataType(DataType::Custom {
                        name: type_name_text(&name),
                    })],
                ));
                true
            }
            Some("partition") if self.kw_at(1, "of") => {
                self.at += 2;
                node.partition_of = Some(Expression::Table(Box::new(table(self.table_name()?))));
                true
            }
            _ => false,
        };
        match typed {
            true if self.is_char('(') => {
                self.at += 1;
                node = self.table_elements(node, true)?;
                self.expect_char(')')?;
            }
            true => {}
            false => {
                self.expect_char('(')?;
                if !self.is_char(')') {
                    node = self.table_elements(node, false)?;
                }
                self.expect_char(')')?;
            }
        }
        if node.partition_of.is_some() {
            let bound: Expression = self.partition_bound()?;
            node.properties.push(bound);
        }
        if !typed && self.eat_kw("inherits") {
            let parents: Vec<Vec<Identifier>> = self.parenthesized(|parser| parser.table_name())?;
            node.inherits = parents.into_iter().map(table).collect();
        }
        if self.kw("partition") {
            let spec: Expression = self.partition_spec()?;
            node.properties.push(spec);
        }
        node = self.table_options(node)?;
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// `table_access_method_clause OptWith OnCommitOption OptTableSpace`.
    fn table_options(&mut self, mut node: CreateTable) -> Parsing<CreateTable> {
        if self.eat_kw("using") {
            node.properties.push(clause(
                "USING",
                vec![Expression::Identifier(self.name(is_col_id)?)],
            ));
        }
        if self.eat_kw("with") {
            let options: Vec<Expression> = self.options(true)?;
            node.properties.push(clause("WITH", options));
        } else if self.eat_kw("without") {
            self.expect_kw("oids")?;
            node.properties.push(clause("WITHOUT OIDS", Vec::new()));
        }
        if self.kw("on") && self.kw_at(1, "commit") {
            self.at += 2;
            let action: &str = match self.keyword_at(0) {
                Some("drop") => "ON COMMIT DROP",
                Some("delete") => "ON COMMIT DELETE ROWS",
                Some("preserve") => "ON COMMIT PRESERVE ROWS",
                _ => return Err(self.error()),
            };
            if !self.eat_kw("drop") {
                self.at += 1;
                self.expect_kw("rows")?;
            }
            node.properties.push(clause(action, Vec::new()));
        }
        if self.eat_kw("tablespace") {
            node.properties.push(clause(
                "TABLESPACE",
                vec![Expression::Identifier(self.name(is_col_id)?)],
            ));
        }
        Ok(node)
    }

    /// `TableElementList` or `TypedTableElementList`: columns into `columns`, the rest into `properties`.
    pub(crate) fn table_elements(
        &mut self,
        mut node: CreateTable,
        typed: bool,
    ) -> Parsing<CreateTable> {
        loop {
            let constraint: bool = self.starts_table_constraint();
            let element: Option<Expression> = match self.keyword_at(0) {
                _ if constraint => Some(self.table_constraint()?),
                Some("like") if !typed => {
                    self.at += 1;
                    let mut values: Vec<Expression> =
                        vec![Expression::Table(Box::new(table(self.table_name()?)))];
                    while self.kw("including") || self.kw("excluding") {
                        let action: &str = match self.eat_kw("including") {
                            true => "INCLUDING",
                            false => {
                                self.at += 1;
                                "EXCLUDING"
                            }
                        };
                        let option: &str = match self.keyword_at(0) {
                            Some(word) if LIKE_OPTIONS.contains(&word) => word,
                            _ => return Err(self.error()),
                        };
                        self.at += 1;
                        values.push(var(&format!("{action} {}", option.to_uppercase())));
                    }
                    Some(clause("LIKE", values))
                }
                _ => {
                    let column: ColumnDef = self.column_definition(typed)?;
                    node.columns.push(column);
                    None
                }
            };
            if let Some(element) = element {
                let position: Expression = number(node.columns.len().to_string());
                node.properties
                    .push(tuple(vec![var(ELEMENT), position, element]));
            }
            if !self.eat_char(',') {
                return Ok(node);
            }
        }
    }

    /// `columnDef`, or `columnOptions` in a typed table: name, type, then options and constraints.
    pub(crate) fn column_definition(&mut self, typed: bool) -> Parsing<ColumnDef> {
        let name: Identifier = self.name(is_col_id)?;
        let data_type: DataType = match typed {
            true => DataType::Unknown,
            false => self.typename()?,
        };
        let mut column: ColumnDef = ColumnDef::new(name.name.clone(), data_type);
        column.name = name;
        column.no_type = typed;
        if typed && self.eat_kw("with") {
            self.expect_kw("options")?;
        }
        if !typed {
            for keyword in ["storage", "compression"] {
                if self.eat_kw(keyword) {
                    let value: Expression = match self.eat_kw("default") {
                        true => var("DEFAULT"),
                        false => Expression::Identifier(self.name(is_col_id)?),
                    };
                    column
                        .options
                        .push(clause(&keyword.to_uppercase(), vec![value]));
                }
            }
            if self.eat_kw("options") {
                let options: Vec<Expression> = self.parenthesized(|parser| {
                    let name: Identifier = parser.name(is_col_label)?;
                    Ok(tuple(vec![Expression::Identifier(name), parser.sconst()?]))
                })?;
                column.options.push(clause("OPTIONS", options));
            }
        }
        let mut collated: bool = false;
        loop {
            let start: usize = self.offset();
            let Some((constraint, collation)) = self.column_constraint()? else {
                break;
            };
            if collation && collated {
                return Err(self.error_with("multiple COLLATE clauses not allowed", start));
            }
            collated |= collation;
            column.options.push(constraint);
        }
        Ok(column)
    }

    /// One `ColConstraint`, and whether it is a `COLLATE` clause; `None` when none comes next.
    pub(crate) fn column_constraint(&mut self) -> Parsing<Option<(Expression, bool)>> {
        let attribute: Option<&str> = match self.keyword_at(0) {
            Some("deferrable") => Some("DEFERRABLE"),
            Some("not") if self.kw_at(1, "deferrable") => Some("NOT DEFERRABLE"),
            Some("not") if self.kw_at(1, "enforced") => Some("NOT ENFORCED"),
            Some("initially") if self.kw_at(1, "deferred") => Some("INITIALLY DEFERRED"),
            Some("initially") if self.kw_at(1, "immediate") => Some("INITIALLY IMMEDIATE"),
            Some("enforced") => Some("ENFORCED"),
            _ => None,
        };
        if let Some(attribute) = attribute {
            self.at += attribute.split(' ').count();
            return Ok(Some((clause(attribute, Vec::new()), false)));
        }
        if self.kw("initially") {
            self.at += 1;
            return Err(self.error());
        }
        if self.eat_kw("collate") {
            let name: Vec<Identifier> = self.qualified_name()?;
            return Ok(Some((clause("COLLATE", vec![name_value(&name)]), true)));
        }
        if self.eat_kw("constraint") {
            let name: Identifier = self.name(is_col_id)?;
            let Some(mut parts) = self.column_constraint_element()? else {
                return Err(self.error());
            };
            parts.splice(0..0, [var(CONSTRAINT), Expression::Identifier(name)]);
            return Ok(Some((tuple(parts), false)));
        }
        Ok(self
            .column_constraint_element()?
            .map(|parts| (tuple(parts), false)))
    }

    /// `ColConstraintElem` as its keyword and values, or `None` when none comes next.
    fn column_constraint_element(&mut self) -> Parsing<Option<Vec<Expression>>> {
        let mut parts: Vec<Expression> = Vec::new();
        match self.keyword_at(0) {
            Some("not") if self.kw_at(1, "null") => {
                self.at += 2;
                parts.push(var("NOT NULL"));
                parts.extend(self.no_inherit());
            }
            Some("null") => {
                self.at += 1;
                parts.push(var("NULL"));
            }
            Some("unique") => {
                self.at += 1;
                parts.push(var("UNIQUE"));
                parts.extend(self.null_treatment()?);
                parts.extend(self.index_parameters()?);
            }
            Some("primary") => {
                self.at += 1;
                self.expect_kw("key")?;
                parts.push(var("PRIMARY KEY"));
                parts.extend(self.index_parameters()?);
            }
            Some("check") => {
                self.at += 1;
                parts.push(var("CHECK"));
                parts.push(self.parenthesized_condition()?);
                parts.extend(self.no_inherit());
            }
            Some("default") => {
                self.at += 1;
                parts.push(var("DEFAULT"));
                parts.push(self.b_expr()?);
            }
            Some("generated") => {
                self.at += 1;
                let when_start: usize = self.offset();
                let always: bool = self.eat_kw("always");
                if !always {
                    self.expect_kw("by")?;
                    self.expect_kw("default")?;
                }
                self.expect_kw("as")?;
                if self.eat_kw("identity") {
                    parts.push(var(match always {
                        true => "GENERATED ALWAYS AS IDENTITY",
                        false => "GENERATED BY DEFAULT AS IDENTITY",
                    }));
                    if self.is_char('(') {
                        self.at += 1;
                        parts.push(self.sequence_option()?);
                        while !self.is_char(')') {
                            parts.push(self.sequence_option()?);
                        }
                        self.at += 1;
                    }
                } else {
                    let expression: Expression = self.parenthesized_condition()?;
                    let kind: &str = match self.keyword_at(0) {
                        Some("stored") => "STORED",
                        Some("virtual") => "VIRTUAL",
                        _ => "",
                    };
                    if !kind.is_empty() {
                        self.at += 1;
                    }
                    if !always {
                        return Err(self.error_with(
                            "for a generated column, GENERATED ALWAYS must be specified",
                            when_start,
                        ));
                    }
                    parts.push(var("GENERATED ALWAYS AS"));
                    parts.push(expression);
                    if !kind.is_empty() {
                        parts.push(var(kind));
                    }
                }
            }
            Some("references") => {
                self.at += 1;
                parts.push(var("REFERENCES"));
                parts.extend(self.references(false)?);
            }
            _ => return Ok(None),
        }
        Ok(Some(parts))
    }

    /// `'(' a_expr ')'`.
    pub(crate) fn parenthesized_condition(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let expression: Expression = self.a_expr()?;
        self.expect_char(')')?;
        Ok(expression)
    }

    /// `opt_no_inherit`.
    fn no_inherit(&mut self) -> Option<Expression> {
        match self.kw("no") && self.kw_at(1, "inherit") {
            true => {
                self.at += 2;
                Some(var("NO INHERIT"))
            }
            false => None,
        }
    }

    /// `opt_unique_null_treatment`.
    fn null_treatment(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("nulls") {
            return Ok(None);
        }
        let not: bool = self.eat_kw("not");
        self.expect_kw("distinct")?;
        Ok(Some(var(match not {
            true => "NULLS NOT DISTINCT",
            false => "NULLS DISTINCT",
        })))
    }

    /// `opt_definition OptConsTableSpace`.
    fn index_parameters(&mut self) -> Parsing<Vec<Expression>> {
        let mut parts: Vec<Expression> = Vec::new();
        if self.eat_kw("with") {
            let options: Vec<Expression> = self.options(false)?;
            parts.push(clause("WITH", options));
        }
        if self.kw("using") && self.kw_at(1, "index") && self.kw_at(2, "tablespace") {
            self.at += 3;
            parts.push(clause(
                "USING INDEX TABLESPACE",
                vec![Expression::Identifier(self.name(is_col_id)?)],
            ));
        }
        Ok(parts)
    }

    /// `REFERENCES qualified_name opt_column_list key_match key_actions`, after the keyword.
    fn references(&mut self, period: bool) -> Parsing<Vec<Expression>> {
        let mut parts: Vec<Expression> = Vec::new();
        parts.push(Expression::Table(Box::new(table(self.table_name()?))));
        if self.is_char('(') {
            parts.push(self.column_list(period)?);
        }
        let match_start: usize = self.offset();
        if self.eat_kw("match") {
            match self.keyword_at(0) {
                Some("full") => parts.push(var("MATCH FULL")),
                Some("simple") => parts.push(var("MATCH SIMPLE")),
                Some("partial") => {
                    return Err(unsupported(
                        "MATCH PARTIAL not yet implemented".to_owned(),
                        match_start,
                    ));
                }
                _ => return Err(self.error()),
            }
            self.at += 1;
        }
        let (mut update, mut delete) = (false, false);
        while self.kw("on") {
            let on_update: bool = self.kw_at(1, "update") && !update;
            if !(on_update || (self.kw_at(1, "delete") && !delete)) {
                break;
            }
            let start: usize = self.offset();
            self.at += 2;
            let action: &str = match self.keyword_at(0) {
                Some("no") if self.kw_at(1, "action") => "NO ACTION",
                Some("restrict") => "RESTRICT",
                Some("cascade") => "CASCADE",
                Some("set") if self.kw_at(1, "null") => "SET NULL",
                Some("set") if self.kw_at(1, "default") => "SET DEFAULT",
                _ => return Err(self.error()),
            };
            self.at += action.split(' ').count();
            let mut values: Vec<Expression> = vec![var(action)];
            if action.starts_with("SET") && self.is_char('(') {
                if on_update {
                    return Err(unsupported(
                        format!(
                            "a column list with {action} is only supported for ON DELETE actions"
                        ),
                        start,
                    ));
                }
                values.push(self.column_list(false)?);
            }
            update |= on_update;
            delete |= !on_update;
            parts.push(clause(
                match on_update {
                    true => "ON UPDATE",
                    false => "ON DELETE",
                },
                values,
            ));
        }
        Ok(parts)
    }

    /// `'(' columnList ')'`, with `optionalPeriodName` when `period`, as a tuple of names.
    pub(crate) fn column_list(&mut self, period: bool) -> Parsing<Expression> {
        self.expect_char('(')?;
        let mut names: Vec<Expression> = Vec::new();
        loop {
            if period
                && self.kw("period")
                && self.peek_at(1).is_some_and(is_col_id)
                && !names.is_empty()
            {
                self.at += 1;
                names.push(clause(
                    "PERIOD",
                    vec![Expression::Identifier(self.name(is_col_id)?)],
                ));
                break;
            }
            names.push(Expression::Identifier(self.name(is_col_id)?));
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char(')')?;
        Ok(tuple(names))
    }

    /// `TableConstraint`: `[CONSTRAINT name] ConstraintElem ConstraintAttributeSpec`, checked as `processCASbits`.
    pub(crate) fn table_constraint(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Expression> = Vec::new();
        if self.eat_kw("constraint") {
            parts.push(var(CONSTRAINT));
            parts.push(Expression::Identifier(self.name(is_col_id)?));
        }
        let (kind, allowed): (&str, u8) = match self.keyword_at(0) {
            Some("check") => {
                self.at += 1;
                parts.push(var("CHECK"));
                parts.push(self.parenthesized_condition()?);
                ("CHECK", NOT_VALID | NO_INHERIT | ENFORCED)
            }
            Some("not") => {
                self.at += 1;
                self.expect_kw("null")?;
                parts.push(var("NOT NULL"));
                parts.push(Expression::Identifier(self.name(is_col_id)?));
                ("NOT NULL", NOT_VALID | NO_INHERIT)
            }
            Some("unique") => {
                self.at += 1;
                parts.push(var("UNIQUE"));
                parts.extend(self.null_treatment()?);
                parts.extend(self.key_columns()?);
                ("UNIQUE", DEFERRABLE)
            }
            Some("primary") => {
                self.at += 1;
                self.expect_kw("key")?;
                parts.push(var("PRIMARY KEY"));
                parts.extend(self.key_columns()?);
                ("PRIMARY KEY", DEFERRABLE)
            }
            Some("exclude") => {
                self.at += 1;
                parts.push(var("EXCLUDE"));
                if self.eat_kw("using") {
                    parts.push(clause(
                        "USING",
                        vec![Expression::Identifier(self.name(is_col_id)?)],
                    ));
                }
                let elements: Vec<Expression> = self.parenthesized(|parser| {
                    let element: Expression = parser.key_element(true)?;
                    parser.expect_kw("with")?;
                    let operator: Expression = match parser.kw("operator") && parser.char_at(1, '(')
                    {
                        true => {
                            parser.at += 1;
                            parser.expect_char('(')?;
                            let operator: Expression = parser.any_operator()?;
                            parser.expect_char(')')?;
                            clause("OPERATOR", vec![operator])
                        }
                        false => parser.any_operator()?,
                    };
                    Ok(tuple(vec![element, var("WITH"), operator]))
                })?;
                parts.push(tuple(elements));
                parts.extend(self.include()?);
                parts.extend(self.index_parameters()?);
                if self.eat_kw("where") {
                    parts.push(clause("WHERE", vec![self.parenthesized_condition()?]));
                }
                ("EXCLUDE", DEFERRABLE)
            }
            Some("foreign") => {
                self.at += 1;
                self.expect_kw("key")?;
                parts.push(var("FOREIGN KEY"));
                parts.push(self.column_list(true)?);
                self.expect_kw("references")?;
                parts.extend(self.references(true)?);
                ("FOREIGN KEY", DEFERRABLE | ENFORCED | NOT_VALID)
            }
            _ => return Err(self.error()),
        };
        let start: usize = self.offset();
        let (bits, attributes): (u8, Vec<Expression>) = self.constraint_attributes()?;
        parts.extend(attributes);
        check_attributes(kind, allowed, bits, start)?;
        Ok(tuple(parts))
    }

    /// Whether a `TableConstraint` begins here rather than a column definition.
    pub(crate) fn starts_table_constraint(&self) -> bool {
        match self.keyword_at(0) {
            Some("constraint" | "check" | "unique" | "primary" | "foreign") => true,
            Some("not") => self.kw_at(1, "null"),
            Some("exclude") => self.kw_at(1, "using") || self.char_at(1, '('),
            _ => false,
        }
    }

    /// `'(' columnList opt_without_overlaps ')' opt_c_include opt_definition OptConsTableSpace`, or `ExistingIndex`.
    fn key_columns(&mut self) -> Parsing<Vec<Expression>> {
        let mut parts: Vec<Expression> = Vec::new();
        if self.eat_kw("using") {
            self.expect_kw("index")?;
            parts.push(clause(
                "USING INDEX",
                vec![Expression::Identifier(self.name(is_col_id)?)],
            ));
            return Ok(parts);
        }
        self.expect_char('(')?;
        let mut names: Vec<Expression> =
            self.comma_list(|parser| Ok(Expression::Identifier(parser.name(is_col_id)?)))?;
        if self.eat_kw("without") {
            self.expect_kw("overlaps")?;
            names.push(var("WITHOUT OVERLAPS"));
        }
        self.expect_char(')')?;
        parts.push(tuple(names));
        parts.extend(self.include()?);
        parts.extend(self.index_parameters()?);
        Ok(parts)
    }

    /// `opt_c_include`.
    fn include(&mut self) -> Parsing<Option<Expression>> {
        match self.eat_kw("include") {
            true => Ok(Some(clause("INCLUDE", vec![self.column_list(false)?]))),
            false => Ok(None),
        }
    }

    /// `ConstraintAttributeSpec`: its `CAS_` bits and each element as written.
    pub(crate) fn constraint_attributes(&mut self) -> Parsing<(u8, Vec<Expression>)> {
        let (mut bits, mut parts): (u8, Vec<Expression>) = (0, Vec::new());
        loop {
            let start: usize = self.offset();
            let (bit, text): (u8, &str) = match self.keyword_at(0) {
                Some("not") if self.kw_at(1, "deferrable") => (NOT_DEFERRABLE, "NOT DEFERRABLE"),
                Some("deferrable") => (DEFERRABLE, "DEFERRABLE"),
                Some("initially") if self.kw_at(1, "immediate") => {
                    (INITIALLY_IMMEDIATE, "INITIALLY IMMEDIATE")
                }
                Some("initially") if self.kw_at(1, "deferred") => {
                    (INITIALLY_DEFERRED, "INITIALLY DEFERRED")
                }
                Some("not") if self.kw_at(1, "valid") => (NOT_VALID, "NOT VALID"),
                Some("no") if self.kw_at(1, "inherit") => (NO_INHERIT, "NO INHERIT"),
                Some("not") if self.kw_at(1, "enforced") => (NOT_ENFORCED, "NOT ENFORCED"),
                Some("enforced") => (ENFORCED, "ENFORCED"),
                Some("initially") => {
                    self.at += 1;
                    return Err(self.error());
                }
                _ => return Ok((bits, parts)),
            };
            self.at += text.split(' ').count();
            bits |= bit;
            let both = |pair: u8| bits & pair == pair;
            if both(NOT_DEFERRABLE | INITIALLY_DEFERRED) {
                return Err(self.error_with(
                    "constraint declared INITIALLY DEFERRED must be DEFERRABLE",
                    start,
                ));
            }
            if both(NOT_DEFERRABLE | DEFERRABLE)
                || both(INITIALLY_IMMEDIATE | INITIALLY_DEFERRED)
                || both(NOT_ENFORCED | ENFORCED)
            {
                return Err(self.error_with("conflicting constraint properties", start));
            }
            parts.push(var(text));
        }
    }

    /// `any_operator`: `[ColId .]... all_Op`, a dotted schema quoted as for calls.
    pub(crate) fn any_operator(&mut self) -> Parsing<Expression> {
        let mut parts: Vec<Identifier> = Vec::new();
        while self.is(is_col_id) && self.char_at(1, '.') {
            parts.push(self.name(is_col_id)?);
            self.at += 1;
        }
        let operator = self.expect_token(is_operator_token)?;
        parts.push(Identifier::new(operator_text(&operator)));
        Ok(name_value(&parts))
    }

    /// `index_elem` (`ordered`) or `part_elem`: a column, a call or `( a_expr )`, then its options.
    pub(crate) fn key_element(&mut self, ordered: bool) -> Parsing<Expression> {
        let head: Expression = match self.peek().map(|token| token.kind) {
            Some(TokenKind::Char('(')) => paren(self.parenthesized_condition()?),
            _ if self.is(is_col_id) && !self.char_at(1, '(') && !self.char_at(1, '.') => {
                Expression::Identifier(self.name(is_col_id)?)
            }
            _ => self.windowless_call()?,
        };
        let mut parts: Vec<Expression> = vec![head];
        if self.eat_kw("collate") {
            let name: Vec<Identifier> = self.qualified_name()?;
            parts.push(clause("COLLATE", vec![name_value(&name)]));
        }
        let ordering: bool = self.kw("nulls") && (self.kw_at(1, "first") || self.kw_at(1, "last"));
        if self.is(is_col_id) && !ordering {
            let name: Vec<Identifier> = self.qualified_name()?;
            let mut values: Vec<Expression> = vec![name_value(&name)];
            if ordered && self.is_char('(') {
                values.push(tuple(self.options(true)?));
            }
            parts.push(clause("OPCLASS", values));
        }
        if ordered {
            if self.eat_kw("asc") {
                parts.push(var("ASC"));
            } else if self.eat_kw("desc") {
                parts.push(var("DESC"));
            }
            if self.kw("nulls") && (self.kw_at(1, "first") || self.kw_at(1, "last")) {
                self.at += 1;
                parts.push(var(match self.eat_kw("first") {
                    true => "NULLS FIRST",
                    false => {
                        self.at += 1;
                        "NULLS LAST"
                    }
                }));
            }
        }
        Ok(tuple(parts))
    }

    /// `func_expr_windowless`: a call by name, or a special call such as `CAST` or `COALESCE`.
    pub(crate) fn windowless_call(&mut self) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        let special: bool = match token.kind {
            TokenKind::Keyword(keyword) => {
                let name: &str = keyword.name;
                (KEY_CALLS.contains(&name) && (self.char_at(1, '(') || self.kw_at(1, "for")))
                    || VALUE_FUNCTIONS.iter().any(|(value, _)| *value == name)
                    || (self.char_at(1, '(')
                        && (JSON_CALLS.contains(&name)
                            || XML_CALLS.contains(&name)
                            || self.is_special_call(name)))
            }
            _ => false,
        };
        if special {
            return self.c_expr(crate::parse::_helpers::values::expressions::Flavor::B);
        }
        let start: usize = self.at;
        let first = self.advance()?;
        let mut parts: Vec<Identifier> =
            vec![crate::parse::_helpers::tree::cursor::identifier(&first)];
        while self.eat_char('.') {
            parts.push(self.name(is_col_label)?);
        }
        let named: bool = match parts.len() {
            1 => is_type_function_name(&first),
            _ => is_col_id(&first),
        };
        if !named || !self.is_char('(') {
            self.at = start;
            return Err(self.error());
        }
        self.function_call(parts, true)
    }

    /// `reloptions` (`dotted`) or `definition`: `( name [= def_arg], ... )`.
    pub(crate) fn options(&mut self, dotted: bool) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| {
            let mut name: Vec<Identifier> = vec![parser.name(is_col_label)?];
            if dotted && parser.eat_char('.') {
                name.push(parser.name(is_col_label)?);
            }
            let mut values: Vec<Expression> = vec![name_value(&name)];
            if parser.eat_char('=') {
                values.push(parser.definition_value()?);
            }
            Ok(tuple(values))
        })
    }

    /// `def_arg`: a type, a reserved word or `NONE`, an operator, a number or a string.
    pub(crate) fn definition_value(&mut self) -> Parsing<Expression> {
        if self.sconst_at(0) {
            return self.sconst();
        }
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        let signed: bool = matches!(token.kind, TokenKind::Char('+' | '-'))
            && self
                .peek_at(1)
                .is_some_and(|next| matches!(next.kind, TokenKind::Integer | TokenKind::Numeric));
        match token.kind {
            TokenKind::Integer | TokenKind::Numeric => return self.numeric_only(),
            _ if signed => return self.numeric_only(),
            _ if is_operator_token(&token) => {
                self.at += 1;
                return Ok(clause(
                    "OPERATOR",
                    vec![Expression::Identifier(Identifier::new(operator_text(
                        &token,
                    )))],
                ));
            }
            TokenKind::Keyword(_) if self.kw("operator") && self.char_at(1, '(') => {
                self.at += 1;
                let name: String = self.operator_name()?;
                return Ok(clause(
                    "OPERATOR",
                    vec![Expression::Identifier(Identifier::new(name))],
                ));
            }
            TokenKind::Keyword(keyword) if is_reserved(&token) || self.kw("none") => {
                self.at += 1;
                return Ok(literal(Literal::String(keyword.name.to_owned())));
            }
            _ => {}
        }
        Ok(Expression::DataType(self.func_type()?))
    }

    /// `PartitionBoundSpec`: `FOR VALUES WITH | IN | FROM ... TO ...`, or `DEFAULT`.
    pub(crate) fn partition_bound(&mut self) -> Parsing<Expression> {
        if self.eat_kw("default") {
            return Ok(clause("DEFAULT", Vec::new()));
        }
        self.expect_kw("for")?;
        self.expect_kw("values")?;
        match self.keyword_at(0) {
            Some("in") => {
                self.at += 1;
                let values: Vec<Expression> = self.parenthesized(|parser| parser.a_expr())?;
                Ok(clause("FOR VALUES IN", values))
            }
            Some("from") => {
                self.at += 1;
                let from: Vec<Expression> = self.parenthesized(|parser| parser.a_expr())?;
                self.expect_kw("to")?;
                let to: Vec<Expression> = self.parenthesized(|parser| parser.a_expr())?;
                Ok(clause("FOR VALUES FROM", vec![tuple(from), tuple(to)]))
            }
            Some("with") => {
                self.at += 1;
                let start: usize = self.offset();
                let elements: Vec<(Identifier, String, usize)> = self.parenthesized(|parser| {
                    let position: usize = parser.offset();
                    let name: Identifier = parser.name(is_non_reserved_word)?;
                    let value = parser.advance()?;
                    if value.kind != TokenKind::Integer {
                        parser.at -= 1;
                        return Err(parser.error());
                    }
                    Ok((name, value.value, position))
                })?;
                let (mut modulus, mut remainder): (Option<String>, Option<String>) = (None, None);
                for (name, value, position) in elements {
                    let slot: &mut Option<String> = match name.name.as_str() {
                        "modulus" => &mut modulus,
                        "remainder" => &mut remainder,
                        other => {
                            return Err(self.error_with(
                                &format!(
                                    "unrecognized hash partition bound specification \"{other}\""
                                ),
                                position,
                            ));
                        }
                    };
                    if slot.is_some() {
                        return Err(unsupported(
                            format!("{} for hash partition provided more than once", name.name),
                            position,
                        ));
                    }
                    *slot = Some(value);
                }
                let Some(modulus) = modulus else {
                    return Err(
                        self.error_with("modulus for hash partition must be specified", start)
                    );
                };
                let Some(remainder) = remainder else {
                    return Err(
                        self.error_with("remainder for hash partition must be specified", start)
                    );
                };
                Ok(clause(
                    "FOR VALUES WITH",
                    vec![number(modulus), number(remainder)],
                ))
            }
            _ => Err(self.error()),
        }
    }

    /// `PartitionSpec`: `PARTITION BY strategy ( part_elem, ... )`.
    fn partition_spec(&mut self) -> Parsing<Expression> {
        self.expect_kw("partition")?;
        self.expect_kw("by")?;
        let start: usize = self.offset();
        let strategy: Identifier = self.name(is_col_id)?;
        let elements: Vec<Expression> = self.parenthesized(|parser| parser.key_element(false))?;
        let folded: String = strategy.name.to_lowercase();
        if !PARTITION_STRATEGIES.contains(&folded.as_str()) {
            return Err(unsupported(
                format!("unrecognized partitioning strategy \"{}\"", strategy.name),
                start,
            ));
        }
        let mut values: Vec<Expression> = vec![var(&folded.to_uppercase())];
        values.extend(elements);
        Ok(clause("PARTITION BY", values))
    }

    /// One `SeqOptElem`, as its keyword and value.
    pub(crate) fn sequence_option(&mut self) -> Parsing<Expression> {
        let Some(word) = self
            .keyword_at(0)
            .filter(|word| SEQUENCE_OPTIONS.contains(word))
        else {
            return Err(self.error());
        };
        self.at += 1;
        let option: Expression = match word {
            "as" => clause("AS", vec![Expression::DataType(self.simple_typename()?)]),
            "cache" | "maxvalue" | "minvalue" => {
                clause(&word.to_uppercase(), vec![self.numeric_only()?])
            }
            "cycle" | "logged" | "unlogged" => clause(&word.to_uppercase(), Vec::new()),
            "no" => match self.keyword_at(0) {
                Some(next @ ("cycle" | "maxvalue" | "minvalue")) => {
                    self.at += 1;
                    clause(&format!("NO {}", next.to_uppercase()), Vec::new())
                }
                _ => return Err(self.error()),
            },
            "increment" => {
                self.eat_kw("by");
                clause("INCREMENT", vec![self.numeric_only()?])
            }
            "owned" => {
                self.expect_kw("by")?;
                let name: Vec<Identifier> = self.qualified_name()?;
                clause("OWNED BY", vec![name_value(&name)])
            }
            "sequence" => {
                self.expect_kw("name")?;
                let name: Vec<Identifier> = self.qualified_name()?;
                clause("SEQUENCE NAME", vec![name_value(&name)])
            }
            "start" => {
                self.eat_kw("with");
                clause("START", vec![self.numeric_only()?])
            }
            _ => {
                let with: bool = self.eat_kw("with");
                let numbered: bool = with
                    || self.peek().is_some_and(|token| {
                        matches!(
                            token.kind,
                            TokenKind::Integer | TokenKind::Numeric | TokenKind::Char('+' | '-')
                        )
                    });
                match numbered {
                    true => clause("RESTART", vec![self.numeric_only()?]),
                    false => clause("RESTART", Vec::new()),
                }
            }
        };
        Ok(option)
    }
}
