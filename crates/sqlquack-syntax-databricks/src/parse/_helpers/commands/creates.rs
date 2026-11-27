//! `createTable`, `replaceTable`, `createView`, `dropTable` and `dropView` (SqlBaseParser.g4).

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, ComputedColumn, ConstraintModifiers, ForeignKeyRef,
    GeneratedAsIdentity, ReferentialAction, TableConstraint,
};
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{CreateView, DropView, ViewColumn};
use sqlquack_syntax::expressions::tables::models::{
    ClusterByColumnsProperty, ConstraintType, CreateTable, DropTable, FileFormatProperty,
    InputOutputFormat, LocationProperty, OptionEntry, OptionsProperty, PartitionedByProperty,
    Property, RowFormatDelimitedProperty, RowFormatSerdeProperty, SchemaCommentProperty,
    SerdeProperties, StorageHandlerProperty,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    boolean, collate, column_from, joined, literal, null, table, too_deep_to_keep, tuple, var,
};
use crate::parse::constants::{
    CREATE_KINDS, DROP_KINDS, DROP_TABLE_FORCE, DROP_TEMPORARY_TABLE, TEMPORARY_VIEW_USING,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// What `CREATE` heads: a table (`OR REPLACE`, `TEMPORARY`, `EXTERNAL`) or a view.
pub(crate) struct Head {
    pub(crate) or_replace: bool,
    pub(crate) replace_only: bool,
    pub(crate) temporary: bool,
    pub(crate) temporary_spelling: Option<String>,
    pub(crate) global: bool,
    pub(crate) external: bool,
    /// The token `CREATE` or `REPLACE` is at.
    pub(crate) at: usize,
}

pub(crate) fn property(name: &str, value: Expression) -> Expression {
    Expression::Property(Box::new(Property {
        this: Box::new(var(name)),
        value: Some(Box::new(value)),
    }))
}

/// `ENABLE` or `DISABLE`, then `VALIDATE` or `NOVALIDATE`, as modifier values, when both read.
fn validation_words(
    enable: Option<&str>,
    validate: Option<&str>,
) -> Option<(Option<bool>, Option<bool>)> {
    let enabled: Option<bool> = match enable {
        None => None,
        Some("ENABLE") => Some(true),
        Some("DISABLE") => Some(false),
        Some(_) => return None,
    };
    let validated: Option<bool> = match validate {
        None => None,
        Some("VALIDATE") => Some(true),
        Some("NOVALIDATE") => Some(false),
        Some(_) => return None,
    };
    Some((enabled, validated))
}

/// The modifiers `constraint` holds, if its kind has them.
fn modifiers_of(constraint: &mut TableConstraint) -> Option<&mut ConstraintModifiers> {
    match constraint {
        TableConstraint::PrimaryKey { modifiers, .. }
        | TableConstraint::ForeignKey { modifiers, .. }
        | TableConstraint::Unique { modifiers, .. }
        | TableConstraint::Check { modifiers, .. } => Some(modifiers),
        _ => None,
    }
}

/// A comment that was given, with its written spelling, as the optional pair a node stores.
fn given((text, written): (String, Option<Literal>)) -> (Option<String>, Option<Literal>) {
    (Some(text), written)
}

/// Whether the written foreign key options repeat one or put MATCH between the two actions.
fn order_unsaid(written: &[String]) -> bool {
    let distinct: std::collections::BTreeSet<&String> = written.iter().collect();
    let between: bool = written
        .iter()
        .position(|option| option.starts_with("MATCH"))
        .is_some_and(|at| at > 0 && at + 1 < written.len());
    distinct.len() < written.len() || between
}

/// The deferral options a constraint reads: both on a primary or foreign key, `DEFERRABLE` on `UNIQUE`, none on `CHECK`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Deferral {
    Both,
    Deferrable,
    Neither,
}

impl Parser<'_> {
    /// `CREATE ...` or `REPLACE TABLE ...`; any other `CREATE` is an opaque gap.
    pub(crate) fn create_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        let mut head = Head {
            or_replace: false,
            replace_only: false,
            temporary: false,
            temporary_spelling: None,
            global: false,
            external: false,
            at: start_at,
        };
        if self.eat_kw("REPLACE") {
            head.or_replace = true;
            head.replace_only = true;
            let temporary: bool = self.kw_in(0, &["TEMP", "TEMPORARY"]) && self.kw_at(1, "TABLE");
            if !self.kw("TABLE") && !temporary {
                return Err(self.error());
            }
        } else {
            self.expect_kw("CREATE")?;
            if self.kw("OR") && self.kw_at(1, "REPLACE") {
                self.at += 2;
                head.or_replace = true;
            }
        }
        let global: bool = self.eat_kw("GLOBAL");
        head.global = global;
        let temp: bool = self.kw("TEMP");
        head.temporary = self.eat_kw("TEMPORARY") || self.eat_kw("TEMP");
        head.temporary_spelling = match (global, temp) {
            (true, true) => Some("GLOBAL TEMP".to_owned()),
            (true, false) => Some("GLOBAL TEMPORARY".to_owned()),
            (false, true) => Some("TEMP".to_owned()),
            (false, false) => None,
        };
        if !global {
            head.external = self.eat_kw("EXTERNAL");
        }
        match self.word_at(0).as_deref() {
            Some("TABLE") if !global => self.create_table(head),
            Some("VIEW") => self.create_view(head),
            Some("SCHEMA" | "DATABASE" | "NAMESPACE") => {
                match head.or_replace || head.temporary || global || head.external {
                    true => Err(self.error()),
                    false => self.create_schema(),
                }
            }
            Some("FUNCTION") if !global && !head.external => self.create_function(head),
            Some(kind) if CREATE_KINDS.contains(&kind) => {
                self.at = start_at;
                self.opaque_statement()
            }
            _ => Err(self.error()),
        }
    }

    /// `IF NOT EXISTS`, when written.
    fn if_not_exists(&mut self) -> bool {
        self.eat_kws(&["IF", "NOT", "EXISTS"])
    }

    fn create_table(&mut self, head: Head) -> Parsing<Expression> {
        self.expect_kw("TABLE")?;
        let if_not_exists: bool = self.if_not_exists();
        let name: TableRef = self.table_reference()?;
        let mut node: CreateTable = CreateTable::new(String::new());
        node.name = name;
        node.if_not_exists = if_not_exists;
        node.or_replace = head.or_replace;
        node.replace_only = head.replace_only;
        node.temporary = head.temporary;
        node.temporary_spelling = head.temporary_spelling.clone();
        if head.external {
            node.table_modifier = Some("EXTERNAL".to_owned());
        }
        if self.kw("LIKE") {
            if head.or_replace || head.temporary || head.external {
                return Err(self.error());
            }
            return self.create_table_like(node);
        }
        if self.kw_in(0, &["SHALLOW", "DEEP", "CLONE"]) {
            if head.temporary || head.external {
                return Err(self.error());
            }
            return self.create_table_clone(node);
        }
        if self.is_char('(') && !self.starts_query_at(1) {
            self.at += 1;
            loop {
                if self.kw_in(0, &["CONSTRAINT", "PRIMARY", "UNIQUE", "FOREIGN", "CHECK"]) {
                    let constraint: TableConstraint = self.table_constraint()?;
                    let keyed: bool = matches!(
                        constraint,
                        TableConstraint::PrimaryKey { .. } | TableConstraint::ForeignKey { .. }
                    );
                    let constraint: TableConstraint = match keyed {
                        true => self.constraint_validation(constraint, &["ENABLE", "NOVALIDATE"]),
                        false => constraint,
                    };
                    node.constraints.push(constraint);
                } else {
                    node.columns.push(self.column_definition()?);
                }
                if !self.eat_char(',') {
                    break;
                }
            }
            self.expect_char(')')?;
        }
        if self.eat_kw("USING") {
            let provider: Identifier = joined(self.multipart_identifier()?);
            node.properties
                .push(Expression::FileFormatProperty(Box::new(
                    FileFormatProperty {
                        this: Some(Box::new(Expression::Identifier(provider))),
                        expressions: Vec::new(),
                        hive_format: None,
                    },
                )));
        }
        node.properties.extend(self.create_table_clauses()?);
        let as_query: bool = self.eat_kw("AS");
        if (as_query || self.starts_query_at(0)) && !node.columns.is_empty() {
            return Err(self.error_with(
                "Operation not allowed: Schema may not be specified in a Create Table As Select (CTAS) statement",
                self.offset(),
                true,
            ));
        }
        let typed_partitions: bool = node.properties.iter().any(|property| {
            matches!(property, Expression::PartitionedByProperty(partitioned)
                if matches!(partitioned.this.as_ref(), Expression::Tuple(fields)
                    if fields.expressions.iter().any(|field| matches!(field, Expression::ColumnDef(_)))))
        });
        if (as_query || self.starts_query_at(0)) && typed_partitions {
            return Err(self.error_with(
                "Operation not allowed: Partition column types may not be specified in Create Table As Select (CTAS)",
                self.offset(),
                true,
            ));
        }
        if as_query || self.starts_query_at(0) {
            let (query, parenthesized) = self.flagged_query()?;
            node.as_select = Some(query);
            node.as_select_parenthesized = parenthesized;
            node.as_select_without_as = !as_query;
        }
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// `createTableClauses`, each at most once, as properties; Hive clauses are gaps.
    fn create_table_clauses(&mut self) -> Parsing<Vec<Expression>> {
        let mut properties: Vec<Expression> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        loop {
            let start: usize = self.offset();
            let Some((clause, property)) = self.create_table_clause(start)? else {
                return Ok(properties);
            };
            properties.extend(property);
            if seen.iter().any(|earlier| earlier == clause) {
                return Err(self.error_with(
                    &format!("Found duplicate clauses: {clause}"),
                    start,
                    true,
                ));
            }
            seen.push(clause.to_owned());
        }
    }

    /// One of `createTableClauses` from `start`: its name and its property; none if none is here.
    fn create_table_clause(
        &mut self,
        start: usize,
    ) -> Parsing<Option<(&'static str, Option<Expression>)>> {
        let Some(word) = self.word_at(0) else {
            return Ok(None);
        };
        let clause: (&'static str, Option<Expression>) = match word.as_str() {
            "OPTIONS" if self.char_at(1, '(') => {
                self.at += 1;
                let spelled: Vec<(OptionEntry, OptionSpelling)> =
                    self.parenthesized(|parser| parser.table_option())?;
                let mut entries: Vec<OptionEntry> = Vec::new();
                let mut string_keys: Vec<usize> = Vec::new();
                let mut without_equals: Vec<usize> = Vec::new();
                let mut written_keys: Vec<(usize, Literal)> = Vec::new();
                for (at, (entry, spelling)) in spelled.into_iter().enumerate() {
                    if let Some(written) = spelling.written {
                        written_keys.push((at, written));
                    }
                    if spelling.string_key {
                        string_keys.push(at);
                    }
                    if spelling.without_equals {
                        without_equals.push(at);
                    }
                    entries.push(entry);
                }
                let options = Expression::OptionsProperty(Box::new(OptionsProperty {
                    written_keys,
                    entries,
                    string_keys,
                    without_equals,
                }));
                ("OPTIONS", Some(options))
            }
            "PARTITIONED" if self.kw_at(1, "BY") => {
                self.at += 2;
                let fields: Vec<Expression> =
                    self.parenthesized(|parser| parser.partition_field())?;
                let partitioned =
                    Expression::PartitionedByProperty(Box::new(PartitionedByProperty {
                        this: Box::new(tuple(fields)),
                    }));
                ("PARTITIONED", Some(partitioned))
            }
            "CLUSTER" if self.kw_at(1, "BY") => {
                self.at += 2;
                let clustering: Option<Expression> = if self.kw("AUTO") && !self.char_at(1, '(') {
                    self.at += 1;
                    Some(property("CLUSTER BY", var("AUTO")))
                } else if self.char_at(0, '(') && self.string_at(1) {
                    let expressions: Vec<Expression> =
                        self.parenthesized(|parser| parser.expression())?;
                    Some(property("CLUSTER BY", tuple(expressions)))
                } else {
                    let columns: Vec<Identifier> =
                        self.parenthesized(|parser| parser.multipart_identifier().map(joined))?;
                    Some(Expression::ClusterByColumnsProperty(Box::new(
                        ClusterByColumnsProperty { columns },
                    )))
                };
                ("CLUSTER", clustering)
            }
            "LOCATION" => {
                self.at += 1;
                let path: Expression = self.string_literal()?;
                let location = Expression::LocationProperty(Box::new(LocationProperty {
                    this: Box::new(path),
                }));
                ("LOCATION", Some(location))
            }
            "COMMENT" => {
                self.at += 1;
                let text: Expression = self.string_literal()?;
                let comment = Expression::SchemaCommentProperty(Box::new(SchemaCommentProperty {
                    this: Box::new(text),
                }));
                ("COMMENT", Some(comment))
            }
            "TBLPROPERTIES" => {
                self.at += 1;
                ("TBLPROPERTIES", Some(self.properties_node(true)?))
            }
            "WITH" if self.kw_at(1, "ROW") && self.kw_at(2, "FILTER") => {
                self.at += 3;
                let filter = Expression::Table(Box::new(table(self.multipart_identifier()?)));
                let columns: Vec<Expression> = self.row_filter_columns()?;
                let row_filter = property("WITH ROW FILTER", tuple(vec![filter, tuple(columns)]));
                if too_deep_to_keep(&row_filter) {
                    self.gap("row_filter", start);
                    return Ok(Some(("WITH", None)));
                }
                self.treed("row_filter", start);
                ("WITH", Some(row_filter))
            }
            "DEFAULT" if self.kw_at(1, "COLLATION") => {
                self.at += 2;
                let name: Identifier = self.ident()?;
                let collation = property("DEFAULT COLLATION", Expression::Identifier(name));
                ("COLLATION", Some(collation))
            }
            "ROW" if self.kw_at(1, "FORMAT") => ("ROW", Some(self.row_format()?)),
            "STORED" if self.kw_in(1, &["AS", "BY"]) => {
                let format: Option<Expression> = self.create_file_format()?;
                if format.is_none() {
                    self.gap("stored_as", start);
                }
                ("STORED", format)
            }
            "CLUSTERED" if self.kw_at(1, "BY") => {
                self.bucket_spec()?;
                self.gap("bucket_spec", start);
                ("CLUSTERED", None)
            }
            "SKEWED" if self.kw_at(1, "BY") => {
                self.skew_spec()?;
                self.gap("skew_spec", start);
                ("SKEWED", None)
            }
            _ => return Ok(None),
        };
        Ok(Some(clause))
    }

    /// One `OPTIONS` entry and how it was written.
    fn table_option(&mut self) -> Parsing<(OptionEntry, OptionSpelling)> {
        let mut strings: usize = 0;
        while self.string_at(strings) {
            strings += 1;
        }
        let string_key: bool = self.string_at(0);
        let (key, written): (Identifier, Option<Literal>) = match string_key {
            true if !self.char_at(strings, '=') => {
                let piece: Literal = self.one_string()?;
                let text: String = piece.value_str().to_string();
                (
                    Identifier::new(text),
                    (!matches!(piece, Literal::String(_))).then_some(piece),
                )
            }
            true => {
                let (text, written) = self.written_string()?;
                (Identifier::new(text), written)
            }
            false => (joined(self.multipart_identifier()?), None),
        };
        let equals: bool = self.eat_char('=');
        if !equals && self.terminal_at(0) {
            return Err(self.error_with(
                &format!(
                    "Operation not allowed: A value must be specified for the key: {}.",
                    key.name
                ),
                self.offset(),
                true,
            ));
        }
        let value: Expression = self.expression()?;
        let spelling: OptionSpelling = OptionSpelling {
            string_key,
            without_equals: !equals,
            written,
        };
        Ok((OptionEntry { key, value }, spelling))
    }

    /// `bucketSpec`: `CLUSTERED BY (columns) [SORTED BY (columns)] INTO n BUCKETS`.
    fn bucket_spec(&mut self) -> Parsing<()> {
        self.at += 2;
        self.identifier_list()?;
        if self.kw("SORTED") && self.kw_at(1, "BY") {
            self.at += 2;
            self.parenthesized(|parser| {
                parser.ident()?;
                if !parser.eat_kw("ASC") {
                    parser.eat_kw("DESC");
                }
                Ok(())
            })?;
        }
        self.expect_kw("INTO")?;
        self.expect_token(|token| token.kind == TokenKind::Number(NumberKind::Integer))?;
        self.expect_kw("BUCKETS")?;
        Ok(())
    }

    /// `skewSpec`: `SKEWED BY (columns) ON (values) [STORED AS DIRECTORIES]`.
    fn skew_spec(&mut self) -> Parsing<()> {
        self.at += 2;
        self.identifier_list()?;
        self.expect_kw("ON")?;
        self.parenthesized(|parser| match parser.is_char('(') {
            true => parser
                .parenthesized(|parser| parser.primary_expression())
                .map(|_| ()),
            false => parser.primary_expression().map(|_| ()),
        })?;
        self.eat_kws(&["STORED", "AS", "DIRECTORIES"]);
        Ok(())
    }

    /// `createFileFormat`: `STORED AS format` or `STORED BY handler`.
    pub(crate) fn create_file_format(&mut self) -> Parsing<Option<Expression>> {
        let start: usize = self.offset();
        self.expect_kw("STORED")?;
        if self.eat_kw("BY") {
            let handler: Expression = self.string_literal()?;
            if self.eat_kws(&["WITH", "SERDEPROPERTIES"]) {
                self.property_list()?;
                return Ok(None);
            }
            self.treed("stored_as", start);
            return Ok(Some(Expression::StorageHandlerProperty(Box::new(
                StorageHandlerProperty {
                    this: Box::new(handler),
                },
            ))));
        }
        self.expect_kw("AS")?;
        let format: Expression = match self.eat_kw("INPUTFORMAT") {
            true => {
                let input: Expression = self.string_literal()?;
                self.expect_kw("OUTPUTFORMAT")?;
                Expression::InputOutputFormat(Box::new(InputOutputFormat {
                    input_format: Some(Box::new(input)),
                    output_format: Some(Box::new(self.string_literal()?)),
                }))
            }
            false => Expression::Identifier(self.ident()?),
        };
        self.treed("stored_as", start);
        Ok(Some(Expression::FileFormatProperty(Box::new(
            FileFormatProperty {
                this: Some(Box::new(format)),
                expressions: Vec::new(),
                hive_format: Some(Box::new(boolean(true))),
            },
        ))))
    }

    /// `createTableLike` after the new table's name: `LIKE source` and its clauses.
    fn create_table_like(&mut self, mut node: CreateTable) -> Parsing<Expression> {
        self.expect_kw("LIKE")?;
        let source: TableRef = self.table_reference()?;
        node.constraints.push(TableConstraint::Like {
            source,
            options: Vec::new(),
        });
        loop {
            let start: usize = self.offset();
            if self.eat_kw("USING") {
                let provider: Identifier = joined(self.multipart_identifier()?);
                node.properties
                    .push(Expression::FileFormatProperty(Box::new(
                        FileFormatProperty {
                            this: Some(Box::new(Expression::Identifier(provider))),
                            expressions: Vec::new(),
                            hive_format: None,
                        },
                    )));
            } else if self.eat_kw("LOCATION") {
                let path: Expression = self.string_literal()?;
                node.properties
                    .push(Expression::LocationProperty(Box::new(LocationProperty {
                        this: Box::new(path),
                    })));
            } else if self.eat_kw("TBLPROPERTIES") {
                let properties: Expression = self.properties_node(false)?;
                node.properties.push(properties);
            } else if self.kw("ROW") && self.kw_at(1, "FORMAT") {
                let format: Expression = self.row_format()?;
                node.properties.push(format);
            } else if self.kw("STORED") && self.kw_in(1, &["AS", "BY"]) {
                match self.create_file_format()? {
                    Some(format) => node.properties.push(format),
                    None => self.gap("stored_as", start),
                }
            } else {
                return Ok(Expression::CreateTable(Box::new(node)));
            }
        }
    }

    /// `[SHALLOW | DEEP] CLONE source [temporal clause] [TBLPROPERTIES (...)] [LOCATION 'path']`.
    fn create_table_clone(&mut self, mut node: CreateTable) -> Parsing<Expression> {
        node.shallow_clone = self.eat_kw("SHALLOW");
        node.deep_clone = !node.shallow_clone && self.eat_kw("DEEP");
        self.expect_kw("CLONE")?;
        node.clone_source = Some(self.table_reference()?);
        node.clone_at_clause = match self.at_version()? {
            Some(version) => Some(Expression::Version(version)),
            None => self.temporal_clause()?.map(Expression::Version),
        };
        let mut seen: Vec<&str> = Vec::new();
        loop {
            let start: usize = self.offset();
            let clause: &str = if self.eat_kw("TBLPROPERTIES") {
                let properties: Expression = self.properties_node(false)?;
                node.properties.push(properties);
                "TBLPROPERTIES"
            } else if self.eat_kw("LOCATION") {
                let path: Expression = self.string_literal()?;
                node.properties
                    .push(Expression::LocationProperty(Box::new(LocationProperty {
                        this: Box::new(path),
                    })));
                "LOCATION"
            } else {
                return Ok(Expression::CreateTable(Box::new(node)));
            };
            if seen.contains(&clause) {
                return Err(self.error_with(
                    &format!("Found duplicate clauses: {clause}"),
                    start,
                    true,
                ));
            }
            seen.push(clause);
        }
    }

    /// `rowFormat` in a table's definition, marked as a tree not yet reviewed for layout.
    fn row_format(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let format: Expression = self.read_row_format()?;
        self.treed("row_format", start);
        Ok(format)
    }

    /// `rowFormat`.
    pub(crate) fn read_row_format(&mut self) -> Parsing<Expression> {
        self.expect_kw("ROW")?;
        self.expect_kw("FORMAT")?;
        if self.eat_kw("SERDE") {
            let serde: Expression = self.string_literal()?;
            let serde_properties: Option<Box<Expression>> =
                match self.eat_kws(&["WITH", "SERDEPROPERTIES"]) {
                    true => Some(Box::new(Expression::SerdeProperties(Box::new(
                        SerdeProperties {
                            expressions: self.property_list()?,
                            with_: Some(Box::new(boolean(true))),
                        },
                    )))),
                    false => None,
                };
            return Ok(Expression::RowFormatSerdeProperty(Box::new(
                RowFormatSerdeProperty {
                    this: Box::new(serde),
                    serde_properties,
                },
            )));
        }
        self.expect_kw("DELIMITED")?;
        let mut node: RowFormatDelimitedProperty = RowFormatDelimitedProperty {
            fields: None,
            escaped: None,
            collection_items: None,
            map_keys: None,
            lines: None,
            null: None,
            serde: None,
        };
        if self.eat_kws(&["FIELDS", "TERMINATED", "BY"]) {
            node.fields = Some(Box::new(self.string_literal()?));
            if self.eat_kws(&["ESCAPED", "BY"]) {
                node.escaped = Some(Box::new(self.string_literal()?));
            }
        }
        if self.eat_kws(&["COLLECTION", "ITEMS", "TERMINATED", "BY"]) {
            node.collection_items = Some(Box::new(self.string_literal()?));
        }
        if self.eat_kws(&["MAP", "KEYS", "TERMINATED", "BY"]) {
            node.map_keys = Some(Box::new(self.string_literal()?));
        }
        if self.eat_kws(&["LINES", "TERMINATED", "BY"]) {
            node.lines = Some(Box::new(self.string_literal()?));
        }
        if self.eat_kws(&["NULL", "DEFINED", "AS"]) {
            node.null = Some(Box::new(self.string_literal()?));
        }
        Ok(Expression::RowFormatDelimitedProperty(Box::new(node)))
    }

    /// `partitionField`: a column (`name type`), a column name, or a transform call.
    fn partition_field(&mut self) -> Parsing<Expression> {
        let name: Vec<Identifier> = self.multipart_identifier()?;
        if self.is_char('(') {
            return self.function_call(name);
        }
        if self.terminal_at(0) {
            return Ok(column_from(name));
        }
        let data_type: DataType = self.data_type()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = joined(name);
        if self.kw("NOT") && self.kw_at(1, "NULL") {
            self.at += 2;
            column.nullable = Some(false);
        }
        if self.eat_kw("COMMENT") {
            let (comment, written) = self.written_string()?;
            column.comment = Some(comment);
            column.comment_written = written;
        }
        Ok(Expression::ColumnDef(Box::new(column)))
    }

    /// `colDefinition`: `name dataType colDefinitionOption*`.
    pub(crate) fn column_definition(&mut self) -> Parsing<ColumnDef> {
        let name: Identifier = self.ident()?;
        let (data_type, collation) = self.collated_type()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        column = collate(column, collation);
        loop {
            if self.kw("NOT") && self.kw_at(1, "NULL") {
                self.at += 2;
                column.nullable = Some(false);
            } else if self.eat_kw("DEFAULT") {
                column.default = Some(self.expression()?);
            } else if self.eat_kw("COMMENT") {
                let (comment, written) = self.written_string()?;
                column.comment = Some(comment);
                column.comment_written = written;
            } else if self.eat_kw("GENERATED") {
                column.constraints.push(self.generation_expression()?);
            } else if self.kw("MASK") {
                let start: usize = self.offset();
                self.at += 1;
                let mask: Expression = self.column_mask()?;
                match too_deep_to_keep(&mask) {
                    true => self.gap("column_mask", start),
                    false => {
                        column.options.push(mask);
                        self.treed("column_mask", start);
                    }
                }
            } else if self.kw_in(
                0,
                &[
                    "CONSTRAINT",
                    "PRIMARY",
                    "UNIQUE",
                    "CHECK",
                    "REFERENCES",
                    "FOREIGN",
                ],
            ) {
                let name: Option<Identifier> = match self.eat_kw("CONSTRAINT") {
                    true => Some(self.ident()?),
                    false => None,
                };
                let kind: ConstraintType = if self.kw("PRIMARY") && self.kw_at(1, "KEY") {
                    self.at += 2;
                    column.primary_key = true;
                    ConstraintType::PrimaryKey
                } else if self.eat_kw("UNIQUE") {
                    column.unique = true;
                    ConstraintType::Unique
                } else if self.kw("CHECK") {
                    column
                        .constraints
                        .push(ColumnConstraint::Check(self.check_constraint()?));
                    ConstraintType::Check
                } else {
                    let has_foreign_key_keywords: bool = self.eat_kws(&["FOREIGN", "KEY"]);
                    let mut reference: ForeignKeyRef = self.reference_spec()?;
                    reference.has_foreign_key_keywords = has_foreign_key_keywords;
                    column
                        .constraints
                        .push(ColumnConstraint::References(Box::new(reference)));
                    ConstraintType::References
                };
                let deferral: Deferral = match &kind {
                    ConstraintType::PrimaryKey | ConstraintType::References => Deferral::Both,
                    ConstraintType::Unique => Deferral::Deferrable,
                    _ => Deferral::Neither,
                };
                let index: usize = column.constraint_order.len();
                column.constraint_order.push(kind);
                column
                    .constraint_names
                    .extend(name.map(|name| (index, name)));
                let modifiers: ConstraintModifiers = self.constraint_characteristics(deferral)?;
                if modifiers != ConstraintModifiers::default() {
                    column.constraint_modifiers.push((index, modifiers));
                }
            } else {
                return Ok(column);
            }
        }
    }

    /// `generationExpression` after `GENERATED`.
    fn generation_expression(&mut self) -> Parsing<ColumnConstraint> {
        if self.kw("ALWAYS") && self.kw_at(1, "AS") && self.char_at(2, '(') {
            self.at += 3;
            let expression: Expression = self.expression()?;
            self.expect_char(')')?;
            return Ok(ColumnConstraint::ComputedColumn(ComputedColumn {
                expression: Box::new(expression),
                persisted: true,
                not_null: false,
                persistence_kind: None,
                data_type: None,
                generated_always: false,
            }));
        }
        let always: bool = self.eat_kw("ALWAYS");
        if !always && !self.eat_kws(&["BY", "DEFAULT"]) {
            return Err(self.error());
        }
        self.expect_kw("AS")?;
        self.expect_kw("IDENTITY")?;
        let mut identity = GeneratedAsIdentity {
            always,
            on_null: false,
            start: None,
            increment: None,
            minvalue: None,
            maxvalue: None,
            cycle: None,
        };
        if self.eat_char('(') {
            loop {
                if self.eat_kws(&["START", "WITH"]) {
                    identity.start = Some(Box::new(self.sequence_value()?));
                } else if self.eat_kws(&["INCREMENT", "BY"]) {
                    identity.increment = Some(Box::new(self.sequence_value()?));
                } else {
                    break;
                }
            }
            self.expect_char(')')?;
        }
        Ok(ColumnConstraint::GeneratedAsIdentity(identity))
    }

    /// `sequenceGeneratorStartOrStep`: `[-] integer`.
    fn sequence_value(&mut self) -> Parsing<Expression> {
        let negative: bool = self.eat_char('-');
        let token = self.expect_token(|token| {
            matches!(
                token.kind,
                TokenKind::Number(NumberKind::Integer | NumberKind::BigInt)
            )
        })?;
        let digits: String = token.value.trim_end_matches(['L', 'l']).to_owned();
        let text: String = match negative {
            true => format!("-{digits}"),
            false => digits,
        };
        Ok(crate::parse::_helpers::tree::nodes::literal(
            Literal::Number(text),
        ))
    }

    /// `checkConstraint`: `CHECK ( booleanExpression )`.
    fn check_constraint(&mut self) -> Parsing<Expression> {
        self.expect_kw("CHECK")?;
        self.expect_char('(')?;
        let condition: Expression = self.expression()?;
        self.expect_char(')')?;
        Ok(condition)
    }

    /// `referenceSpec`: `REFERENCES table [(columns)]`.
    fn reference_spec(&mut self) -> Parsing<ForeignKeyRef> {
        self.expect_kw("REFERENCES")?;
        let table: TableRef =
            crate::parse::_helpers::tree::nodes::table(self.multipart_identifier()?);
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.identifier_list()?,
            false => Vec::new(),
        };
        let mut match_full: bool = false;
        let mut on_delete: Option<ReferentialAction> = None;
        let mut on_update: Option<ReferentialAction> = None;
        let mut on_update_first: bool = false;
        let mut match_after_actions: bool = false;
        let mut written: Vec<String> = Vec::new();
        loop {
            if self.eat_kws(&["MATCH", "FULL"]) {
                match_full = true;
                match_after_actions = on_delete.is_some() || on_update.is_some();
                written.push("MATCH FULL".to_owned());
            } else if self.kw("ON")
                && self.kw_in(1, &["DELETE", "UPDATE"])
                && self.kw_at(2, "NO")
                && self.kw_at(3, "ACTION")
            {
                match self.kw_at(1, "UPDATE") {
                    true => {
                        on_update_first = on_delete.is_none();
                        on_update = Some(ReferentialAction::NoAction);
                        written.push("ON UPDATE NO ACTION".to_owned());
                    }
                    false => {
                        on_delete = Some(ReferentialAction::NoAction);
                        written.push("ON DELETE NO ACTION".to_owned());
                    }
                }
                self.at += 4;
            } else {
                break;
            }
        }
        Ok(ForeignKeyRef {
            table,
            columns,
            on_delete,
            on_update,
            on_update_first,
            match_type: match_full
                .then_some(sqlquack_syntax::expressions::columns::models::MatchType::Full),
            match_after_actions,
            constraint_name: None,
            deferrable: None,
            has_foreign_key_keywords: false,
            written_order: match order_unsaid(&written) {
                true => written,
                false => Vec::new(),
            },
        })
    }

    /// `constraintCharacteristic*`: `[NOT] ENFORCED`, `RELY`, `NORELY`, and the deferral options `deferral` allows.
    fn constraint_characteristics(&mut self, deferral: Deferral) -> Parsing<ConstraintModifiers> {
        let mut modifiers: ConstraintModifiers = ConstraintModifiers::default();
        let mut written: Vec<String> = Vec::new();
        loop {
            if deferral != Deferral::Neither && self.eat_kw("DEFERRABLE") {
                modifiers.deferrable = Some(true);
                written.push("DEFERRABLE".to_owned());
            } else if deferral == Deferral::Both && self.eat_kws(&["INITIALLY", "DEFERRED"]) {
                modifiers.initially_deferred = Some(true);
                written.push("INITIALLY DEFERRED".to_owned());
            } else if self.eat_kw("ENFORCED") {
                modifiers.enforced = Some(true);
                written.push("ENFORCED".to_owned());
            } else if self.kw("NOT") && self.kw_at(1, "ENFORCED") {
                self.at += 2;
                modifiers.enforced = Some(false);
                written.push("NOT ENFORCED".to_owned());
            } else if self.eat_kw("RELY") {
                modifiers.rely = true;
                written.push("RELY".to_owned());
            } else if self.eat_kw("NORELY") {
                modifiers.norely = true;
                written.push("NORELY".to_owned());
            } else {
                if written.len() > 1 {
                    modifiers.properties = written;
                }
                return Ok(modifiers);
            }
        }
    }

    /// `constraint` with the `words` after it as its modifiers when written once each, else a gap.
    pub(crate) fn constraint_validation(
        &mut self,
        mut constraint: TableConstraint,
        words: &[&str],
    ) -> TableConstraint {
        let start: usize = self.offset();
        let mut written: Vec<String> = Vec::new();
        while let Some(word) = self
            .word_at(0)
            .filter(|word| words.contains(&word.as_str()))
        {
            self.at += 1;
            written.push(word);
        }
        if written.is_empty() {
            return constraint;
        }
        let read: Option<(Option<bool>, Option<bool>)> = match written
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>()
            .as_slice()
        {
            [enable] => {
                validation_words(None, Some(enable)).or(validation_words(Some(enable), None))
            }
            [enable, validate] => validation_words(Some(enable), Some(validate)),
            _ => None,
        };
        match (read, modifiers_of(&mut constraint)) {
            (Some((enabled, validated)), Some(modifiers)) => {
                modifiers.enabled = enabled;
                modifiers.validated = validated;
                self.treed("constraint_validation", start);
            }
            _ => self.gap("constraint_validation", start),
        }
        constraint
    }

    /// `tableConstraintDefinition`.
    pub(crate) fn table_constraint(&mut self) -> Parsing<TableConstraint> {
        let has_constraint_keyword: bool = self.eat_kw("CONSTRAINT");
        let name: Option<Identifier> = match has_constraint_keyword {
            true => Some(self.ident()?),
            false => None,
        };
        if self.kw("CHECK") {
            let expression: Expression = self.check_constraint()?;
            let modifiers: ConstraintModifiers =
                self.constraint_characteristics(Deferral::Neither)?;
            return Ok(TableConstraint::Check {
                name,
                expression,
                modifiers,
            });
        }
        if self.eat_kw("FOREIGN") {
            self.expect_kw("KEY")?;
            let columns: Vec<Identifier> = self.identifier_list()?;
            let mut reference: ForeignKeyRef = self.reference_spec()?;
            reference.has_foreign_key_keywords = true;
            let modifiers: ConstraintModifiers = self.constraint_characteristics(Deferral::Both)?;
            return Ok(TableConstraint::ForeignKey {
                name,
                columns,
                references: Some(Box::new(reference)),
                on_delete: None,
                on_update: None,
                modifiers,
            });
        }
        let primary: bool = self.kw("PRIMARY") && self.kw_at(1, "KEY");
        if primary {
            self.at += 2;
        } else {
            self.expect_kw("UNIQUE")?;
        }
        let mut timeseries_columns: Vec<Identifier> = Vec::new();
        let columns: Vec<Identifier> = self.parenthesized(|parser| {
            let column: Identifier = parser.ident()?;
            if primary && parser.eat_kw("TIMESERIES") {
                timeseries_columns.push(column.clone());
            }
            Ok(column)
        })?;
        if timeseries_columns.len() > 1 {
            return Err(self.error_with(
                "[UNSUPPORTED_TIMESERIES_WITH_MORE_THAN_ONE_COLUMN] Creating primary key with \
                 more than one timeseries column is not supported",
                self.offset(),
                false,
            ));
        }
        let deferral: Deferral = match primary {
            true => Deferral::Both,
            false => Deferral::Deferrable,
        };
        let modifiers: ConstraintModifiers = self.constraint_characteristics(deferral)?;
        Ok(match primary {
            true => TableConstraint::PrimaryKey {
                name,
                columns,
                key_parts: Vec::new(),
                timeseries_columns,
                include_columns: Vec::new(),
                modifiers,
                has_constraint_keyword,
            },
            false => TableConstraint::Unique {
                name,
                columns,
                key_parts: Vec::new(),
                columns_parenthesized: true,
                modifiers,
                has_constraint_keyword,
                nulls_not_distinct: false,
            },
        })
    }

    fn create_view(&mut self, head: Head) -> Parsing<Expression> {
        let start_offset: usize = self.offset();
        self.expect_kw("VIEW")?;
        let if_not_exists: bool = self.if_not_exists();
        let name_at: usize = self.at;
        let name: TableRef = self.table_reference()?;
        if head.temporary && !if_not_exists && self.temporary_view_using_ahead() {
            self.at = name_at;
            return self.temporary_view_using(head.at, name_at - 1);
        }
        let mut columns: Vec<ViewColumn> = Vec::new();
        if self.is_char('(') {
            columns = self.parenthesized(|parser| {
                let mut column: ViewColumn = ViewColumn::new(String::new());
                column.name = parser.ident()?;
                if parser.eat_kw("COMMENT") {
                    (column.comment, column.comment_written) = given(parser.written_string()?);
                }
                Ok(column)
            })?;
        }
        let mut comment: Option<(String, Option<Literal>)> = None;
        let mut table_properties: Vec<Expression> = Vec::new();
        let mut copy_grants: bool = false;
        let mut options: Vec<Expression> = Vec::new();
        loop {
            let start: usize = self.offset();
            if let Some(option) = self.view_option(start)? {
                options.push(option);
                continue;
            }
            if self.eat_kw("COMMENT") {
                if comment.is_some() {
                    return Err(self.error_with("Found duplicate clauses: COMMENT", start, true));
                }
                comment = Some(self.written_string()?);
            } else if self.eat_kw("TBLPROPERTIES") {
                let properties: Expression = self.properties_node(false)?;
                table_properties.push(properties);
            } else if self.kw("WITH") && self.kw_at(1, "METRICS") {
                if head.global {
                    self.at += 1;
                    return Err(self.error());
                }
                let mut node: CreateView = CreateView::new(String::new(), null());
                node.name = name;
                node.columns = columns;
                node.or_replace = head.or_replace;
                node.if_not_exists = if_not_exists;
                node.temporary = head.temporary;
                node.temporary_spelling = head.temporary_spelling;
                (node.comment, node.comment_written) = comment.map_or((None, None), given);
                node.table_properties = table_properties;
                node.copy_grants = copy_grants;
                node.options = options;
                return self.metric_view(node);
            } else if self.kw("COPY") && self.char_at(1, '*') {
                self.at += 2;
                copy_grants = true;
            } else {
                break;
            }
        }
        self.expect_kw("AS")?;
        if head.or_replace && if_not_exists {
            let message: &str = "CREATE VIEW with both IF NOT EXISTS and REPLACE is not allowed";
            return Err(self.error_with(message, start_offset, true));
        }
        if head.temporary && if_not_exists {
            let message: &str = "[TEMP_VIEW_IF_NOT_EXISTS_NOT_ALLOWED] It is not allowed to define a \
                                 TEMPORARY view with IF NOT EXISTS";
            return Err(self.error_with(message, start_offset, false));
        }
        if head.temporary && !table_properties.is_empty() {
            let message: &str =
                "Operation not allowed: TBLPROPERTIES can't coexist with CREATE TEMPORARY VIEW";
            return Err(self.error_with(message, start_offset, true));
        }
        let (query, parenthesized) = self.flagged_query()?;
        let mut node: CreateView = CreateView::new(String::new(), query);
        node.name = name;
        node.columns = columns;
        node.or_replace = head.or_replace;
        node.if_not_exists = if_not_exists;
        node.temporary = head.temporary;
        node.temporary_spelling = head.temporary_spelling;
        (node.comment, node.comment_written) = comment.map_or((None, None), given);
        node.table_properties = table_properties;
        node.query_parenthesized = parenthesized;
        node.copy_grants = copy_grants;
        node.options = options;
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// A view's schema binding, default collation, partitioning columns or `NOCOPY *`, read from `start`.
    fn view_option(&mut self, start: usize) -> Parsing<Option<Expression>> {
        let (construct, option) = if self.kw("WITH") && self.kw_at(1, "SCHEMA") {
            self.at += 2;
            let mode: &str = match self.word_at(0).as_deref() {
                Some("BINDING") => "BINDING",
                Some("COMPENSATION") => "COMPENSATION",
                Some("EVOLUTION") => "EVOLUTION",
                Some("TYPE") if self.kw_at(1, "EVOLUTION") => "TYPE EVOLUTION",
                _ => return Err(self.error()),
            };
            self.at += mode.split(' ').count();
            ("schema_binding", property("WITH SCHEMA", var(mode)))
        } else if self.kw("DEFAULT") && self.kw_at(1, "COLLATION") {
            self.at += 2;
            let name: Identifier = self.ident()?;
            (
                "view_collation",
                property("DEFAULT COLLATION", Expression::Identifier(name)),
            )
        } else if self.kw("PARTITIONED") && self.kw_at(1, "ON") {
            self.at += 2;
            let columns: Vec<Expression> = self
                .identifier_list()?
                .into_iter()
                .map(Expression::Identifier)
                .collect();
            (
                "view_partitioned_on",
                property("PARTITIONED ON", tuple(columns)),
            )
        } else if self.kw("NOCOPY") && self.char_at(1, '*') {
            self.at += 2;
            ("view_nocopy", property("NOCOPY", var("*")))
        } else {
            return Ok(None);
        };
        self.treed(construct, start);
        Ok(Some(option))
    }

    /// `createMetricView` from `WITH METRICS`: its clauses, then `AS $$ body $$`, the body as the view's query.
    fn metric_view(&mut self, mut node: CreateView) -> Parsing<Expression> {
        loop {
            if self.eat_kws(&["WITH", "METRICS"]) {
                node.options.push(property("WITH", var("METRICS")));
            } else if self.eat_kw("LANGUAGE") {
                let language: Token = self.expect_token(|token| token.kind == TokenKind::Word)?;
                node.options
                    .push(property("LANGUAGE", var(&language.value)));
            } else if self.eat_kw("COMMENT") {
                let (comment, written) = self.written_string()?;
                node.comment = Some(comment);
                node.comment_written = written;
            } else if self.eat_kw("TBLPROPERTIES") {
                let properties: Expression = self.properties_node(false)?;
                node.table_properties.push(properties);
            } else {
                break;
            }
        }
        self.expect_kw("AS")?;
        let body: Token = self.expect_token(|token| {
            token.kind == TokenKind::DollarString && !token.value.is_empty()
        })?;
        node.query = literal(Literal::DollarString(body.value));
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// Whether `createTempViewUsing` follows the view name: `USING`, or a column list with types.
    fn temporary_view_using_ahead(&self) -> bool {
        self.kw("USING")
            || (self.is_char('(')
                && self.is_at(1, is_identifier)
                && !self.char_at(2, ',')
                && !self.char_at(2, ')')
                && !self.kw_at(2, "COMMENT"))
    }

    /// `createTempViewUsing` after the view, from `CREATE` at `from` to `VIEW` at `view`; a gap unless its documented form reads it all.
    fn temporary_view_using(&mut self, from: usize, view: usize) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.ident()?;
        if self.is_char('.') {
            self.at += 1;
            self.ident()?;
        }
        if self.is_char('(') {
            self.parenthesized(|parser| {
                parser.ident()?;
                parser.data_type()?;
                if parser.kw("NOT") && parser.kw_at(1, "NULL") {
                    parser.at += 2;
                }
                if parser.eat_kw("COMMENT") {
                    parser.string_spelling()?;
                }
                Ok(())
            })?;
        }
        self.expect_kw("USING")?;
        self.multipart_identifier()?;
        if self.eat_kw("OPTIONS") {
            let options_at: usize = self.offset();
            let options: Vec<Expression> = self.property_list()?;
            if options
                .iter()
                .any(|option| !matches!(option, Expression::Eq(_)))
            {
                return Err(self.error_with(
                    "Operation not allowed: Values must be specified for key(s)",
                    options_at,
                    true,
                ));
            }
        }
        match self.tree_within(from, self.at, view + 1 - from, TEMPORARY_VIEW_USING) {
            Some(tree) => {
                self.treed("temporary_view_using", start);
                Ok(tree)
            }
            None => {
                self.gap("temporary_view_using", start);
                Ok(null())
            }
        }
    }

    /// `DROP TABLE` and `DROP VIEW`; any other `DROP` is an opaque gap.
    pub(crate) fn drop_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        self.expect_kw("DROP")?;
        let temporary: bool = self.kw_in(0, &["TEMP", "TEMPORARY"]) && self.kw_at(1, "TABLE");
        if temporary {
            self.at += 1;
        }
        match self.word_at(0).as_deref() {
            Some("TABLE") => {
                self.at += 1;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                let name: TableRef = self.table_reference()?;
                let mut node: DropTable = DropTable::new(String::new());
                node.names = vec![name];
                node.if_exists = if_exists;
                node.purge = self.eat_kw("PURGE");
                let (construct, start, lead, pattern) = match temporary {
                    true => (
                        "drop_temporary_table",
                        self.tokens[start_at].start,
                        3,
                        DROP_TEMPORARY_TABLE,
                    ),
                    false => ("drop_table_force", self.offset(), 2, DROP_TABLE_FORCE),
                };
                if !(temporary || self.eat_kw("FORCE")) {
                    return Ok(Expression::DropTable(Box::new(node)));
                }
                match self.tree_within(start_at, self.at, lead, pattern) {
                    Some(tree) => {
                        self.treed(construct, start);
                        Ok(tree)
                    }
                    None => {
                        self.gap(construct, start);
                        Ok(Expression::DropTable(Box::new(node)))
                    }
                }
            }
            Some("VIEW") => {
                self.at += 1;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                let mut node: DropView = DropView::new(String::new());
                node.name = self.table_reference()?;
                node.if_exists = if_exists;
                Ok(Expression::DropView(Box::new(node)))
            }
            Some("SCHEMA" | "DATABASE" | "NAMESPACE") if !temporary => self.drop_schema(),
            Some("FUNCTION") => self.drop_function(false),
            Some("TEMPORARY" | "TEMP") if self.kw_at(1, "FUNCTION") => {
                self.at += 1;
                self.drop_function(true)
            }
            Some("INDEX") => Err(self.error_with(
                "[INVALID_STATEMENT_OR_CLAUSE] The statement or clause: DROP INDEX is not valid.",
                self.tokens[start_at].start,
                true,
            )),
            Some("TEMPORARY" | "TEMP") if !self.kw_in(1, &["VIEW", "TABLE", "FUNCTION"]) => {
                self.at += 1;
                Err(self.error())
            }
            Some(kind) if DROP_KINDS.contains(&kind) => {
                self.at = start_at;
                self.opaque_statement()
            }
            _ => Err(self.error()),
        }
    }
}

/// How one `OPTIONS` entry was written: a string key, no `=`, and a raw or adjacent-string key.
pub(crate) struct OptionSpelling {
    string_key: bool,
    without_equals: bool,
    written: Option<Literal>,
}
