//! `CREATE`, `ALTER`, `DROP` and `TRUNCATE` for the objects SQLBuild manages; other kinds are gaps.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    AlterColumnAction, AlterTable, AlterTableAction, ColumnConstraint, ColumnDef, TableConstraint,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::queries::models::With;
use sqlquack_syntax::expressions::schema_objects::models::{
    AlterView, AlterViewAction, CreateFunction, CreateProcedure, CreateSchema, CreateView,
    DropSchema, DropView, FunctionBody, FunctionParameter, ParameterMode, ViewColumn,
};
use sqlquack_syntax::expressions::statements::models::Truncate;
use sqlquack_syntax::expressions::tables::models::{
    CreateIndex, CreateTable, DropTable, IndexColumn,
};

use crate::parse::_helpers::statements::queries::set_with;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier};
use crate::parse::_helpers::tree::nodes::{column_from, table, var};
use crate::scan::models::{Token, TokenKind};

impl<'sql> Parser<'sql> {
    /// A two- or three-part object name as a table reference.
    fn object_name(&mut self) -> Parsing<TableRef> {
        Ok(table(self.multipart(4)?))
    }

    /// `CREATE [OR ALTER] { TABLE | VIEW | FUNCTION | SCHEMA | [UNIQUE] INDEX } ...`.
    pub(crate) fn create_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("CREATE")?;
        let or_alter: bool = self.eat_kws(&["OR", "ALTER"]);
        match self.word_at(0).as_deref() {
            Some("TABLE") if !or_alter => self.create_table(),
            Some("VIEW") => self.create_view(or_alter, false),
            Some("FUNCTION") => self.create_function(or_alter),
            Some("SCHEMA") if !or_alter => self.create_schema(),
            Some("INDEX" | "UNIQUE" | "NONCLUSTERED" | "CLUSTERED") if !or_alter => {
                self.create_index()
            }
            Some("PROC" | "PROCEDURE") => self.create_procedure(or_alter),
            Some("TRIGGER" | "TYPE" | "SEQUENCE" | "SYNONYM" | "DEFAULT") => {
                Err(self.gap("create_object", start))
            }
            Some(_) if !or_alter => Err(self.gap("create_object", start)),
            _ => Err(self.error()),
        }
    }

    /// `CREATE [OR ALTER] PROC[EDURE] name [parameters] [WITH RECOMPILE | ENCRYPTION] AS [statement ...]`, its body the rest of the batch, which may be empty.
    fn create_procedure(&mut self, or_alter: bool) -> Parsing<Expression> {
        let start: usize = self.offset();
        let mut node: CreateProcedure = CreateProcedure::new(String::new());
        node.use_proc_keyword = self.kw("PROC");
        self.at += 1;
        node.name = self.object_name()?;
        node.or_alter = or_alter;
        if self.is_char(';') {
            return Err(self.gap("numbered_procedure", start));
        }
        node.has_parens = self.eat_char('(');
        let listed: bool = self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Variable);
        if listed {
            node.parameters = self.comma_list(|parser| parser.procedure_parameter())?;
        } else if node.has_parens {
            return Err(self.error());
        }
        if node.has_parens {
            self.expect_char(')')?;
        }
        if self.eat_kw("WITH") {
            node.with_options = self.comma_list(|parser| match parser.word_at(0).as_deref() {
                Some(option @ ("RECOMPILE" | "ENCRYPTION")) => {
                    let option: String = option.to_owned();
                    parser.at += 1;
                    Ok(option)
                }
                _ => Err(parser.gap("procedure_option", parser.offset())),
            })?;
        }
        if self.kw("FOR") {
            return Err(self.gap("procedure_for_replication", self.offset()));
        }
        self.expect_kw("AS")?;
        if self.kw("EXTERNAL") {
            return Err(self.gap("clr_procedure", start));
        }
        let mut body: Vec<Expression> = Vec::new();
        loop {
            while self.eat_char(';') {}
            if self.at_end() {
                break;
            }
            body.push(self.statement()?);
        }
        node.body = Some(FunctionBody::Statements(body));
        Ok(Expression::CreateProcedure(Box::new(node)))
    }

    /// `@name [AS] type [= default] [OUT | OUTPUT]`; `VARYING`, `NULL` and `READONLY` are gaps.
    fn procedure_parameter(&mut self) -> Parsing<FunctionParameter> {
        let token: Token = self.expect_token(|token| token.kind == TokenKind::Variable)?;
        self.eat_kw("AS");
        let data_type: DataType = self.data_type()?;
        if self.kw_in(0, &["VARYING", "NULL", "NOT", "READONLY"]) {
            return Err(self.gap("procedure_parameter_option", self.offset()));
        }
        let default: Option<Expression> = match self.eat_char('=') {
            true => Some(self.procedure_default()?),
            false => None,
        };
        let mode_text: Option<String> = match self.kw_in(0, &["OUT", "OUTPUT"]) {
            true => self.peek().map(|token| token.value.clone()),
            false => None,
        };
        self.at += usize::from(mode_text.is_some());
        if self.kw("READONLY") {
            return Err(self.gap("procedure_parameter_option", self.offset()));
        }
        Ok(FunctionParameter {
            name: Some(identifier(&token)),
            data_type,
            mode: mode_text.as_ref().map(|_| ParameterMode::Out),
            default,
            mode_text,
        })
    }

    /// A parameter's default, which SQL Server takes only as a constant: a literal, `-` and a number, or a bare or bracketed word.
    fn procedure_default(&mut self) -> Parsing<Expression> {
        let start: usize = self.at;
        let negative: bool = self.char_at(0, '-');
        let width: usize = 1 + usize::from(negative);
        let Some(value) = self.peek_at(width - 1).cloned() else {
            return Err(self.error());
        };
        let word: bool = matches!(value.kind, TokenKind::Word | TokenKind::Quoted(_))
            && !value.value.eq_ignore_ascii_case("NULL");
        let default: Expression = match value.kind {
            TokenKind::Number(_) => self.scalar()?,
            TokenKind::String { .. } | TokenKind::Word if !negative && !word => self.scalar()?,
            TokenKind::Word | TokenKind::Quoted(_) if !negative && !self.char_at(1, '(') => {
                self.at += 1;
                match value.kind {
                    TokenKind::Word => var(&value.value),
                    _ => column_from(vec![identifier(&value)]),
                }
            }
            _ => return Err(self.error_at(start + width - 1 + usize::from(word))),
        };
        if self.at != start + width {
            return Err(self.error_at(start + width));
        }
        Ok(default)
    }

    /// `CREATE TABLE name ( column_def | table_constraint, ... )`.
    fn create_table(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("TABLE")?;
        let mut node: CreateTable = CreateTable::new(String::new());
        node.name = self.object_name()?;
        node.temporary = node.name.name.name.starts_with('#');
        if self.kw("AS") {
            return Err(self.gap("create_table_as", start));
        }
        self.expect_char('(')?;
        loop {
            if self.kw("PERIOD") && self.kw_at(1, "FOR") {
                return Err(self.gap("period_for_system_time", self.offset()));
            }
            if self.kw_in(
                0,
                &[
                    "CONSTRAINT",
                    "PRIMARY",
                    "UNIQUE",
                    "FOREIGN",
                    "CHECK",
                    "INDEX",
                ],
            ) {
                node.constraints.push(self.table_constraint()?);
            } else {
                node.columns.push(self.column_definition()?);
            }
            if !self.eat_char(',') {
                break;
            }
        }
        self.expect_char(')')?;
        if self.kw_in(0, &["ON", "WITH", "TEXTIMAGE_ON", "FILESTREAM_ON", "AS"]) {
            return Err(self.gap("table_options", self.offset()));
        }
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// `name type [NULL | NOT NULL | DEFAULT value | IDENTITY [(seed, step)] | PRIMARY KEY | UNIQUE]...`.
    fn column_definition(&mut self) -> Parsing<ColumnDef> {
        let name: Identifier = self.name()?;
        if self.kw("AS") {
            return Err(self.gap("computed_column", self.offset()));
        }
        let data_type: DataType = self.data_type()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        loop {
            if self.eat_kws(&["NOT", "NULL"]) {
                column.nullable = Some(false);
                column.constraints.push(ColumnConstraint::NotNull);
            } else if self.eat_kw("NULL") {
                column.nullable = Some(true);
                column.constraints.push(ColumnConstraint::Null);
            } else if self.eat_kw("DEFAULT") {
                let value: Expression = self.scalar()?;
                column.default = Some(value.clone());
                column.constraints.push(ColumnConstraint::Default(value));
            } else if self.eat_kws(&["PRIMARY", "KEY"]) {
                column.primary_key = true;
                column.constraints.push(ColumnConstraint::PrimaryKey);
                self.eat_kw("CLUSTERED");
                self.eat_kw("NONCLUSTERED");
            } else if self.eat_kw("UNIQUE") {
                column.unique = true;
                column.constraints.push(ColumnConstraint::Unique);
                if !self.eat_kw("CLUSTERED") {
                    self.eat_kw("NONCLUSTERED");
                }
            } else if self.kw_in(
                0,
                &[
                    "COLLATE",
                    "IDENTITY",
                    "CONSTRAINT",
                    "CHECK",
                    "REFERENCES",
                    "FOREIGN",
                    "SPARSE",
                    "ROWGUIDCOL",
                    "FILESTREAM",
                    "MASKED",
                    "ENCRYPTED",
                    "GENERATED",
                    "INDEX",
                    "WITH",
                    "HIDDEN",
                    "COLUMN_SET",
                    "ON",
                ],
            ) {
                return Err(self.gap("column_option", self.offset()));
            } else {
                return Ok(column);
            }
        }
    }

    /// `[CONSTRAINT name] { PRIMARY KEY | UNIQUE } [CLUSTERED | NONCLUSTERED] ( columns )`.
    fn table_constraint(&mut self) -> Parsing<TableConstraint> {
        let start: usize = self.offset();
        let name: Option<Identifier> = match self.eat_kw("CONSTRAINT") {
            true => Some(self.name()?),
            false => None,
        };
        let primary: bool = match self.word_at(0).as_deref() {
            Some("PRIMARY") if self.kw_at(1, "KEY") => {
                self.at += 2;
                true
            }
            Some("UNIQUE") => {
                self.at += 1;
                false
            }
            _ => return Err(self.gap("table_constraint", start)),
        };
        if !self.eat_kw("CLUSTERED") {
            self.eat_kw("NONCLUSTERED");
        }
        let columns: Vec<Identifier> = self.parenthesized(|parser| {
            let column: Identifier = parser.name()?;
            if parser.kw_in(0, &["ASC", "DESC"]) {
                return Err(parser.gap("constraint_column_order", parser.offset()));
            }
            Ok(column)
        })?;
        if self.kw_in(0, &["WITH", "ON"]) {
            return Err(self.gap("constraint_options", self.offset()));
        }
        let has_constraint_keyword: bool = name.is_some();
        Ok(match primary {
            true => TableConstraint::PrimaryKey {
                name,
                columns,
                key_parts: Vec::new(),
                timeseries_columns: Vec::new(),
                include_columns: Vec::new(),
                modifiers: Default::default(),
                has_constraint_keyword,
            },
            false => TableConstraint::Unique {
                name,
                columns,
                key_parts: Vec::new(),
                columns_parenthesized: true,
                modifiers: Default::default(),
                has_constraint_keyword,
                nulls_not_distinct: false,
            },
        })
    }

    /// `VIEW name [(columns)] [WITH options] AS query`, for `CREATE`, `CREATE OR ALTER` and `ALTER`.
    fn create_view(&mut self, or_alter: bool, alter: bool) -> Parsing<Expression> {
        self.expect_kw("VIEW")?;
        let name: TableRef = self.object_name()?;
        let columns: Vec<Identifier> = match self.is_char('(') {
            true => self.parenthesized(Parser::name)?,
            false => Vec::new(),
        };
        if self.kw("WITH") {
            return Err(self.gap("view_options", self.offset()));
        }
        self.expect_kw("AS")?;
        let query: Expression = self.query_statement()?;
        if self.kw("WITH") && self.kw_at(1, "CHECK") {
            return Err(self.gap("view_check_option", self.offset()));
        }
        if alter {
            let mut node: AlterView = AlterView::new(String::new());
            node.name = name;
            node.columns = columns.into_iter().map(view_column).collect();
            node.actions = vec![AlterViewAction::AsSelect(Box::new(query))];
            return Ok(Expression::AlterView(Box::new(node)));
        }
        let mut node: CreateView = CreateView::new(String::new(), query);
        node.name = name;
        node.or_alter = or_alter;
        node.columns = columns.into_iter().map(view_column).collect();
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// `FUNCTION name (...) RETURNS ...`, a scalar or inline table function; a multi-statement one is a gap.
    fn create_function(&mut self, or_alter: bool) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("FUNCTION")?;
        let mut node: CreateFunction = CreateFunction::new(String::new());
        node.name = self.object_name()?;
        node.or_alter = or_alter;
        node.has_parens = true;
        self.expect_char('(')?;
        if !self.is_char(')') {
            node.parameters = self.comma_list(|parser| {
                let token: Token =
                    parser.expect_token(|token| token.kind == TokenKind::Variable)?;
                parser.eat_kw("AS");
                let data_type: DataType = parser.data_type()?;
                if parser.kw_in(0, &["=", "NULL", "NOT", "READONLY", "OUTPUT", "OUT"])
                    || parser.is_char('=')
                {
                    return Err(parser.gap("function_parameter_option", parser.offset()));
                }
                Ok(FunctionParameter {
                    name: Some(identifier(&token)),
                    data_type,
                    mode: None,
                    default: None,
                    mode_text: None,
                })
            })?;
        }
        self.expect_char(')')?;
        self.expect_kw("RETURNS")?;
        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Variable)
        {
            return Err(self.gap("multi_statement_table_function", start));
        }
        if self.kw("TABLE") && self.char_at(1, '(') {
            return Err(self.gap("clr_table_function", start));
        }
        if self.eat_kw("TABLE") {
            node.is_table_function = true;
            node.return_type = Some(DataType::Custom {
                name: "TABLE".to_owned(),
            });
            (node.options, node.returns_null_on_null_input) = self.function_options()?;
            self.eat_kw("AS");
            self.expect_kw("RETURN")?;
            let wrapped: bool = self.is_char('(') && self.kw_at(1, "WITH");
            if wrapped {
                self.at += 1;
            }
            let with: Option<With> = self.with_clause()?;
            let mut query: Expression = self.query_expression()?;
            if let Some(with) = with {
                set_with(&mut query, with);
            }
            if self.kw("FOR") && self.kw_in(1, &["XML", "JSON", "BROWSE"]) {
                return Err(self.gap("for_xml_json", self.offset()));
            }
            if wrapped {
                self.expect_char(')')?;
            }
            node.body = Some(FunctionBody::Return(query));
            return Ok(Expression::CreateFunction(Box::new(node)));
        }
        node.return_type = Some(self.data_type()?);
        (node.options, node.returns_null_on_null_input) = self.function_options()?;
        self.eat_kw("AS");
        if self.kw("EXTERNAL") || (self.kw("BEGIN") && self.kw_at(1, "ATOMIC")) {
            return Err(self.gap("clr_or_native_function", start));
        }
        if !self.kw("BEGIN") {
            return Err(self.error());
        }
        self.at += 1;
        let body: Vec<Expression> = self.block_body(&["END"])?;
        node.body = Some(FunctionBody::Statements(body));
        Ok(Expression::CreateFunction(Box::new(node)))
    }

    /// `WITH option, ...` after a function's return type, any known option for any kind of function.
    fn function_options(&mut self) -> Parsing<(Vec<Expression>, Option<bool>)> {
        let mut options: Vec<Expression> = Vec::new();
        let mut null_input: Option<bool> = None;
        if !self.eat_kw("WITH") {
            return Ok((options, null_input));
        }
        loop {
            let word: String = self.word_at(0).unwrap_or_default();
            let option: Expression = match word.as_str() {
                "SCHEMABINDING" | "ENCRYPTION" | "NATIVE_COMPILATION" => {
                    self.at += 1;
                    var(&word)
                }
                "RETURNS" => {
                    self.at += 1;
                    self.expect_kws(&["NULL", "ON", "NULL", "INPUT"])?;
                    null_input = Some(true);
                    var("RETURNS NULL ON NULL INPUT")
                }
                "CALLED" => {
                    self.at += 1;
                    self.expect_kws(&["ON", "NULL", "INPUT"])?;
                    null_input = Some(false);
                    var("CALLED ON NULL INPUT")
                }
                "EXECUTE" | "EXEC" => {
                    self.at += 1;
                    self.expect_kw("AS")?;
                    let principal: String = match self.word_at(0).as_deref() {
                        Some(name @ ("CALLER" | "SELF" | "OWNER")) => {
                            let name: String = name.to_owned();
                            self.at += 1;
                            name
                        }
                        _ => {
                            let token: Token = self.expect_token(|token| {
                                matches!(token.kind, TokenKind::String { .. })
                            })?;
                            self.sql[token.start..token.end].to_owned()
                        }
                    };
                    var(&format!("EXECUTE AS {principal}"))
                }
                "INLINE" => {
                    self.at += 1;
                    self.expect_char('=')?;
                    let setting: String = match self.word_at(0).as_deref() {
                        Some(setting @ ("ON" | "OFF")) => setting.to_owned(),
                        _ => return Err(self.error()),
                    };
                    self.at += 1;
                    var(&format!("INLINE = {setting}"))
                }
                _ => return Err(self.error_with("not a recognized option")),
            };
            options.push(option);
            if !self.eat_char(',') {
                return Ok((options, null_input));
            }
        }
    }

    /// `SCHEMA name [AUTHORIZATION owner]`; a schema element after it is a gap.
    fn create_schema(&mut self) -> Parsing<Expression> {
        self.expect_kw("SCHEMA")?;
        let mut node: CreateSchema = CreateSchema::new(String::new());
        node.name = vec![self.name()?];
        if self.eat_kw("AUTHORIZATION") {
            node.authorization = Some(self.name()?);
        }
        if !self.at_end() && !self.is_char(';') {
            return Err(self.gap("schema_element", self.offset()));
        }
        Ok(Expression::CreateSchema(Box::new(node)))
    }

    /// `[UNIQUE] [CLUSTERED | NONCLUSTERED] INDEX name ON table ( column [ASC|DESC], ... )`.
    fn create_index(&mut self) -> Parsing<Expression> {
        let unique: bool = self.eat_kw("UNIQUE");
        let clustered: Option<String> = match self.word_at(0).as_deref() {
            Some(word @ ("CLUSTERED" | "NONCLUSTERED")) => {
                let word: String = word.to_owned();
                self.at += 1;
                Some(word)
            }
            _ => None,
        };
        if self.kw("COLUMNSTORE") {
            return Err(self.gap("columnstore_index", self.offset()));
        }
        self.expect_kw("INDEX")?;
        let mut node: CreateIndex = CreateIndex::new(String::new(), String::new());
        node.name = self.name()?;
        node.unique = unique;
        node.clustered = clustered;
        self.expect_kw("ON")?;
        node.table = self.object_name()?;
        node.columns = self.parenthesized(|parser| {
            let column: Identifier = parser.name()?;
            let desc: bool = parser.eat_kw("DESC");
            let asc: bool = !desc && parser.eat_kw("ASC");
            Ok(IndexColumn {
                column,
                desc,
                asc,
                nulls_first: None,
                opclass: None,
            })
        })?;
        if self.eat_kw("INCLUDE") {
            node.include_columns = self.parenthesized(Parser::name)?;
        }
        if self.kw_in(0, &["WHERE", "WITH", "ON", "FILESTREAM_ON"]) {
            return Err(self.gap("index_options", self.offset()));
        }
        Ok(Expression::CreateIndex(Box::new(node)))
    }

    /// `ALTER TABLE`, `ALTER VIEW` and `ALTER SCHEMA ... TRANSFER`.
    pub(crate) fn alter_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("ALTER")?;
        match self.word_at(0).as_deref() {
            Some("TABLE") => self.alter_table(),
            Some("VIEW") => self.create_view(false, true),
            _ => Err(self.gap("alter_object", start)),
        }
    }

    /// `ALTER TABLE t { ADD column_def, ... | DROP COLUMN c, ... | ALTER COLUMN c type [NULL|NOT NULL] }`.
    fn alter_table(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("TABLE")?;
        let mut node: AlterTable = AlterTable::new(String::new());
        node.name = self.object_name()?;
        match self.word_at(0).as_deref() {
            Some("ADD")
                if !self.kw_in(
                    1,
                    &[
                        "CONSTRAINT",
                        "PRIMARY",
                        "UNIQUE",
                        "FOREIGN",
                        "CHECK",
                        "PERIOD",
                        "INDEX",
                        "DEFAULT",
                        "CONNECTION",
                    ],
                ) =>
            {
                self.at += 1;
                node.actions = self.comma_list(|parser| {
                    Ok(AlterTableAction::AddColumn {
                        column: parser.column_definition()?,
                        if_not_exists: false,
                        position: None,
                    })
                })?;
            }
            Some("DROP") if self.kw_at(1, "COLUMN") => {
                self.at += 2;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                node.actions = self.comma_list(|parser| {
                    Ok(AlterTableAction::DropColumn {
                        name: parser.name()?,
                        if_exists,
                        cascade: false,
                    })
                })?;
            }
            Some("ALTER") if self.kw_at(1, "COLUMN") => {
                self.at += 2;
                let name: Identifier = self.name()?;
                if self.kw_in(0, &["ADD", "DROP"]) {
                    return Err(self.gap("alter_column_option", start));
                }
                let data_type: DataType = self.data_type()?;
                if self.kw_in(0, &["COLLATE", "SPARSE", "HIDDEN", "MASKED", "ENCRYPTED"]) {
                    return Err(self.gap("alter_column_option", start));
                }
                let mut actions: Vec<AlterTableAction> = vec![AlterTableAction::AlterColumn {
                    name: name.clone(),
                    action: AlterColumnAction::SetDataType {
                        data_type,
                        using: None,
                        collate: None,
                    },
                    use_modify_keyword: false,
                }];
                let nullability: Option<AlterColumnAction> = if self.eat_kws(&["NOT", "NULL"]) {
                    Some(AlterColumnAction::SetNotNull)
                } else if self.eat_kw("NULL") {
                    Some(AlterColumnAction::DropNotNull)
                } else {
                    None
                };
                if let Some(action) = nullability {
                    actions.push(AlterTableAction::AlterColumn {
                        name,
                        action,
                        use_modify_keyword: false,
                    });
                }
                if self.kw("WITH") && self.char_at(1, '(') {
                    return Err(self.gap("alter_column_option", start));
                }
                node.actions = actions;
            }
            _ => return Err(self.gap("alter_table_action", start)),
        }
        Ok(Expression::AlterTable(Box::new(node)))
    }

    /// `DROP { TABLE | VIEW | SCHEMA } [IF EXISTS] name, ...`; other kinds are gaps.
    pub(crate) fn drop_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("DROP")?;
        let kind: Option<String> = self.word_at(0);
        match kind.as_deref() {
            Some("TABLE" | "VIEW" | "SCHEMA") => self.at += 1,
            _ => return Err(self.gap("drop_object", start)),
        }
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        match kind.as_deref() {
            Some("TABLE") => {
                let mut node: DropTable = DropTable::new(String::new());
                node.names = self.comma_list(Parser::object_name)?;
                node.if_exists = if_exists;
                Ok(Expression::DropTable(Box::new(node)))
            }
            Some("VIEW") => {
                let names: Vec<TableRef> = self.comma_list(Parser::object_name)?;
                if names.len() > 1 {
                    return Err(self.gap("drop_views", start));
                }
                let mut node: DropView = DropView::new(String::new());
                node.name = names
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| TableRef::new(String::new()));
                node.if_exists = if_exists;
                Ok(Expression::DropView(Box::new(node)))
            }
            _ => {
                let mut node: DropSchema = DropSchema::new(String::new());
                node.name = self.name()?;
                node.if_exists = if_exists;
                Ok(Expression::DropSchema(Box::new(node)))
            }
        }
    }

    /// `TRUNCATE TABLE name`.
    pub(crate) fn truncate_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kws(&["TRUNCATE", "TABLE"])?;
        let mut node: Truncate = Truncate::new(String::new());
        node.table = self.object_name()?;
        if self.kw("WITH") {
            return Err(self.gap("truncate_partitions", start));
        }
        Ok(Expression::Truncate(Box::new(node)))
    }
}

fn view_column(name: Identifier) -> ViewColumn {
    let mut column: ViewColumn = ViewColumn::new(String::new());
    column.name = name;
    column
}
