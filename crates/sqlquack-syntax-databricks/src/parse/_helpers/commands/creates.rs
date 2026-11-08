//! `createTable`, `replaceTable`, `createView`, `dropTable` and `dropView` (SqlBaseParser.g4).

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    ColumnConstraint, ColumnDef, ComputedColumn, ConstraintModifiers, ForeignKeyRef,
    GeneratedAsIdentity, TableConstraint,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{CreateView, DropView, ViewColumn};
use sqlquack_syntax::expressions::tables::models::{
    ClusterByColumnsProperty, CreateTable, DropTable, FileFormatProperty, LocationProperty,
    OptionEntry, OptionsProperty, PartitionedByProperty, Properties, Property,
    SchemaCommentProperty,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{binary, column_from, joined, null, string, tuple, var};
use crate::parse::constants::{CREATE_KINDS, DROP_KINDS};
use crate::scan::models::{NumberKind, TokenKind};

/// What `CREATE` heads: a table (`OR REPLACE`, `TEMPORARY`, `EXTERNAL`) or a view.
pub(crate) struct Head {
    pub(crate) or_replace: bool,
    pub(crate) temporary: bool,
    pub(crate) global: bool,
    pub(crate) external: bool,
}

pub(crate) fn property(name: &str, value: Expression) -> Expression {
    Expression::Property(Box::new(Property {
        this: Box::new(var(name)),
        value: Some(Box::new(value)),
    }))
}

impl Parser<'_> {
    /// `CREATE ...` or `REPLACE TABLE ...`; any other `CREATE` is an opaque gap.
    pub(crate) fn create_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        let mut head = Head {
            or_replace: false,
            temporary: false,
            global: false,
            external: false,
        };
        if self.eat_kw("REPLACE") {
            head.or_replace = true;
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
        head.temporary = self.eat_kw("TEMPORARY") || self.eat_kw("TEMP");
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
        node.temporary = head.temporary;
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
                    let validation: usize = self.offset();
                    if keyed && self.kw_in(0, &["ENABLE", "NOVALIDATE"]) {
                        while self.eat_kw("ENABLE") || self.eat_kw("NOVALIDATE") {}
                        self.gap("constraint_validation", validation);
                    }
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
            let parenthesized: bool = self.is_char('(');
            node.as_select = Some(self.query()?);
            node.as_select_parenthesized = parenthesized;
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
                let entries: Vec<OptionEntry> =
                    self.parenthesized(|parser| parser.table_option())?;
                let options = Expression::OptionsProperty(Box::new(OptionsProperty { entries }));
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
                    self.parenthesized(|parser| parser.expression())?;
                    self.gap("cluster_by_expression", start);
                    None
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
                let expressions: Vec<Expression> = self
                    .valued_property_list()?
                    .into_iter()
                    .map(|(key, value)| binary(Expression::Eq, key, value))
                    .collect();
                let properties = Expression::Properties(Box::new(Properties { expressions }));
                ("TBLPROPERTIES", Some(properties))
            }
            "WITH" if self.kw_at(1, "ROW") && self.kw_at(2, "FILTER") => {
                self.at += 3;
                self.multipart_identifier()?;
                self.expect_kw("ON")?;
                self.expect_char('(')?;
                if !self.is_char(')') {
                    self.comma_list(|parser| parser.expression())?;
                }
                self.expect_char(')')?;
                self.gap("row_filter", start);
                ("WITH", None)
            }
            "DEFAULT" if self.kw_at(1, "COLLATION") => {
                self.at += 2;
                let name: Identifier = self.ident()?;
                let collation = property("DEFAULT COLLATION", string(&name.name));
                ("COLLATION", Some(collation))
            }
            "ROW" if self.kw_at(1, "FORMAT") => {
                self.row_format()?;
                self.gap("row_format", start);
                ("ROW", None)
            }
            "STORED" if self.kw_in(1, &["AS", "BY"]) => {
                self.create_file_format()?;
                self.gap("stored_as", start);
                ("STORED", None)
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

    /// One `OPTIONS` entry: adjacent strings are the key and its value unless `=` follows them.
    fn table_option(&mut self) -> Parsing<OptionEntry> {
        let mut strings: usize = 0;
        while self.string_at(strings) {
            strings += 1;
        }
        let key: Identifier = match self.string_at(0) {
            true if !self.char_at(strings, '=') => Identifier::new(self.advance()?.value),
            true => Identifier::new(self.string_lit()?),
            false => joined(self.multipart_identifier()?),
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
        Ok(OptionEntry { key, value })
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
    pub(crate) fn create_file_format(&mut self) -> Parsing<()> {
        self.expect_kw("STORED")?;
        if self.eat_kw("BY") {
            self.string_lit()?;
            if self.eat_kws(&["WITH", "SERDEPROPERTIES"]) {
                self.property_list()?;
            }
            return Ok(());
        }
        self.expect_kw("AS")?;
        if self.eat_kw("INPUTFORMAT") {
            self.string_lit()?;
            self.expect_kw("OUTPUTFORMAT")?;
            self.string_lit()?;
        } else {
            self.ident()?;
        }
        Ok(())
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
                let expressions: Vec<Expression> = self.property_list()?;
                node.properties
                    .push(Expression::Properties(Box::new(Properties { expressions })));
            } else if self.kw("ROW") && self.kw_at(1, "FORMAT") {
                self.row_format()?;
                self.gap("row_format", start);
            } else if self.kw("STORED") && self.kw_in(1, &["AS", "BY"]) {
                self.create_file_format()?;
                self.gap("stored_as", start);
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
        node.clone_at_clause = self.temporal_clause()?.map(Expression::Version);
        let mut seen: Vec<&str> = Vec::new();
        loop {
            let start: usize = self.offset();
            let clause: &str = if self.eat_kw("TBLPROPERTIES") {
                let expressions: Vec<Expression> = self.property_list()?;
                node.properties
                    .push(Expression::Properties(Box::new(Properties { expressions })));
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

    /// `rowFormat`.
    pub(crate) fn row_format(&mut self) -> Parsing<()> {
        self.expect_kw("ROW")?;
        self.expect_kw("FORMAT")?;
        if self.eat_kw("SERDE") {
            self.string_lit()?;
            if self.eat_kws(&["WITH", "SERDEPROPERTIES"]) {
                self.property_list()?;
            }
            return Ok(());
        }
        self.expect_kw("DELIMITED")?;
        if self.eat_kws(&["FIELDS", "TERMINATED", "BY"]) {
            self.string_lit()?;
            if self.eat_kws(&["ESCAPED", "BY"]) {
                self.string_lit()?;
            }
        }
        if self.eat_kws(&["COLLECTION", "ITEMS", "TERMINATED", "BY"]) {
            self.string_lit()?;
        }
        if self.eat_kws(&["MAP", "KEYS", "TERMINATED", "BY"]) {
            self.string_lit()?;
        }
        if self.eat_kws(&["LINES", "TERMINATED", "BY"]) {
            self.string_lit()?;
        }
        if self.eat_kws(&["NULL", "DEFINED", "AS"]) {
            self.string_lit()?;
        }
        Ok(())
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
            column.comment = Some(self.string_lit()?);
        }
        Ok(Expression::ColumnDef(Box::new(column)))
    }

    /// `colDefinition`: `name dataType colDefinitionOption*`.
    pub(crate) fn column_definition(&mut self) -> Parsing<ColumnDef> {
        let name: Identifier = self.ident()?;
        let data_type: DataType = self.data_type()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        loop {
            if (self.kw("NOT") || self.is_char('!')) && self.kw_at(1, "NULL") {
                self.at += 2;
                column.nullable = Some(false);
            } else if self.eat_kw("DEFAULT") {
                column.default = Some(self.expression()?);
            } else if self.eat_kw("COMMENT") {
                column.comment = Some(self.string_lit()?);
            } else if self.eat_kw("GENERATED") {
                column.constraints.push(self.generation_expression()?);
            } else if self.kw("MASK") {
                let start: usize = self.offset();
                self.at += 1;
                self.column_mask()?;
                self.gap("column_mask", start);
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
                if self.eat_kw("CONSTRAINT") {
                    self.ident()?;
                }
                if self.kw("PRIMARY") && self.kw_at(1, "KEY") {
                    self.at += 2;
                    column.primary_key = true;
                } else if self.eat_kw("UNIQUE") {
                    column.unique = true;
                } else if self.kw("CHECK") {
                    column
                        .constraints
                        .push(ColumnConstraint::Check(self.check_constraint()?));
                } else {
                    self.eat_kws(&["FOREIGN", "KEY"]);
                    let reference: ForeignKeyRef = self.reference_spec()?;
                    column
                        .constraints
                        .push(ColumnConstraint::References(Box::new(reference)));
                }
                self.constraint_characteristics()?;
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
            sqlquack_syntax::expressions::names::models::Literal::Number(text),
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
        loop {
            if self.eat_kws(&["MATCH", "FULL"]) {
                match_full = true;
            } else if self.kw("ON")
                && self.kw_in(1, &["DELETE", "UPDATE"])
                && self.kw_at(2, "NO")
                && self.kw_at(3, "ACTION")
            {
                self.at += 4;
            } else {
                break;
            }
        }
        Ok(ForeignKeyRef {
            table,
            columns,
            on_delete: None,
            on_update: None,
            on_update_first: false,
            match_type: match_full
                .then_some(sqlquack_syntax::expressions::columns::models::MatchType::Full),
            match_after_actions: false,
            constraint_name: None,
            deferrable: None,
            has_foreign_key_keywords: false,
        })
    }

    /// `constraintCharacteristic*`: `[NOT] ENFORCED`, `RELY` and `NORELY`.
    fn constraint_characteristics(&mut self) -> Parsing<ConstraintModifiers> {
        let mut modifiers: ConstraintModifiers = ConstraintModifiers::default();
        loop {
            if self.eat_kw("ENFORCED") {
                modifiers.enforced = Some(true);
            } else if self.kw("NOT") && self.kw_at(1, "ENFORCED") {
                self.at += 2;
                modifiers.enforced = Some(false);
            } else if self.eat_kw("RELY") {
                modifiers.rely = true;
            } else if self.eat_kw("NORELY") {
                modifiers.norely = true;
            } else {
                return Ok(modifiers);
            }
        }
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
            let modifiers: ConstraintModifiers = self.constraint_characteristics()?;
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
            let modifiers: ConstraintModifiers = self.constraint_characteristics()?;
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
        let modifiers: ConstraintModifiers = self.constraint_characteristics()?;
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
            return self.temporary_view_using();
        }
        let mut columns: Vec<ViewColumn> = Vec::new();
        if self.is_char('(') {
            columns = self.parenthesized(|parser| {
                let mut column: ViewColumn = ViewColumn::new(String::new());
                column.name = parser.ident()?;
                if parser.eat_kw("COMMENT") {
                    column.comment = Some(parser.string_lit()?);
                }
                Ok(column)
            })?;
        }
        let mut comment: Option<String> = None;
        let mut table_properties: Vec<Expression> = Vec::new();
        let mut copy_grants: bool = false;
        loop {
            let start: usize = self.offset();
            if self.eat_kw("COMMENT") {
                if comment.is_some() {
                    return Err(self.error_with("Found duplicate clauses: COMMENT", start, true));
                }
                comment = Some(self.string_lit()?);
            } else if self.eat_kw("TBLPROPERTIES") {
                let expressions: Vec<Expression> = self.property_list()?;
                table_properties.push(Expression::Properties(Box::new(Properties { expressions })));
            } else if self.kw("WITH") && self.kw_at(1, "SCHEMA") {
                self.at += 2;
                if !(self.eat_kw("BINDING")
                    || self.eat_kw("COMPENSATION")
                    || self.eat_kw("EVOLUTION")
                    || self.eat_kws(&["TYPE", "EVOLUTION"]))
                {
                    return Err(self.error());
                }
                self.gap("schema_binding", start);
            } else if self.kw("WITH") && self.kw_at(1, "METRICS") {
                if head.global {
                    self.at += 1;
                    return Err(self.error());
                }
                return self.metric_view(start);
            } else if self.kw("DEFAULT") && self.kw_at(1, "COLLATION") {
                self.at += 2;
                self.ident()?;
                self.gap("view_collation", start);
            } else if self.kw("PARTITIONED") && self.kw_at(1, "ON") {
                self.at += 2;
                self.identifier_list()?;
                self.gap("view_partitioned_on", start);
            } else if self.kw("COPY") && self.char_at(1, '*') {
                self.at += 2;
                copy_grants = true;
            } else if self.kw("NOCOPY") && self.char_at(1, '*') {
                self.at += 2;
                self.gap("view_nocopy", start);
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
        let parenthesized: bool = self.is_char('(');
        let query: Expression = self.query()?;
        let mut node: CreateView = CreateView::new(String::new(), query);
        node.name = name;
        node.columns = columns;
        node.or_replace = head.or_replace;
        node.if_not_exists = if_not_exists;
        node.temporary = head.temporary;
        node.comment = comment;
        node.table_properties = table_properties;
        node.query_parenthesized = parenthesized;
        node.copy_grants = copy_grants;
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// `createMetricView` from `WITH METRICS`: its clauses, then `AS $$ body $$`; kept as a gap.
    fn metric_view(&mut self, start: usize) -> Parsing<Expression> {
        loop {
            if self.eat_kws(&["WITH", "METRICS"]) {
            } else if self.eat_kw("LANGUAGE") {
                self.expect_token(|token| token.kind == TokenKind::Word)?;
            } else if self.eat_kw("COMMENT") {
                self.string_lit()?;
            } else if self.eat_kw("TBLPROPERTIES") {
                self.property_list()?;
            } else {
                break;
            }
        }
        self.expect_kw("AS")?;
        self.expect_token(|token| {
            token.kind == TokenKind::DollarString && !token.value.is_empty()
        })?;
        self.gap("metric_view", start);
        Ok(null())
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

    /// `createTempViewUsing` after the view, every option with a value; kept as a gap.
    fn temporary_view_using(&mut self) -> Parsing<Expression> {
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
                    parser.string_lit()?;
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
        self.gap("temporary_view_using", start);
        Ok(null())
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
                if !temporary {
                    let start: usize = self.offset();
                    if self.eat_kw("FORCE") {
                        self.gap("drop_table_force", start);
                    }
                }
                if temporary {
                    let start: usize = self.tokens[start_at].start;
                    self.gap("drop_temporary_table", start);
                }
                Ok(Expression::DropTable(Box::new(node)))
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
