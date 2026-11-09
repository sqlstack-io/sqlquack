//! `ParserCreateQuery`, `ParserDropQuery` and `ParserAlterQuery`, for the forms SQLBuild emits.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    AlterTableAction, ColumnDef, ColumnPosition, MergeTreeTTL,
};
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::PrimaryKey;
use sqlquack_syntax::expressions::queries::models::{OrderBy, Ordered};
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateDatabase, CreateFunction, CreateView, DropDatabase, DropFunction, DropView, FunctionBody,
    FunctionParameter,
};
use sqlquack_syntax::expressions::tables::models::{
    CreateTable, DropTable, EngineProperty, OnCluster, PartitionedByProperty, SampleProperty,
    SchemaCommentProperty, SettingsProperty,
};
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::commands::statements::alter;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::table;
use crate::scan::models::TokenKind;

fn boxed(this: Expression) -> Box<Expression> {
    Box::new(this)
}

impl Parser<'_> {
    /// `CREATE [OR REPLACE] ...`: a database, a table (temporary or not), a view or a function.
    pub(crate) fn create_statement(&mut self) -> Parsing<Expression> {
        let at: usize = self.at;
        self.expect_kw("CREATE")?;
        let or_replace: bool = self.eat_kws(&["OR", "REPLACE"]);
        let temporary: bool = self.kws(&["TEMPORARY", "TABLE"]);
        if temporary {
            self.at += 1;
        }
        match self.word_at(0).as_deref() {
            Some("DATABASE") => self.create_database(or_replace),
            Some("TABLE") => self.create_table(or_replace, temporary),
            Some("VIEW") => self.create_view(or_replace),
            Some("FUNCTION") => self.create_function(or_replace),
            _ => {
                self.at = at;
                self.unsupported_statement()
            }
        }
    }

    /// `IF NOT EXISTS`, if next.
    fn if_not_exists(&mut self) -> bool {
        self.eat_kws(&["IF", "NOT", "EXISTS"])
    }

    /// `DATABASE [IF NOT EXISTS] name [ON CLUSTER c] [ENGINE = e] [COMMENT 'c']`.
    fn create_database(&mut self, or_replace: bool) -> Parsing<Expression> {
        self.expect_kw("DATABASE")?;
        let if_not_exists: bool = self.if_not_exists();
        let name: Identifier = self.ident()?;
        let mut node: CreateDatabase = CreateDatabase::new(String::new());
        node.name = name;
        node.or_replace = or_replace;
        node.if_not_exists = if_not_exists;
        if let Some(cluster) = self.on_cluster()? {
            node.properties
                .push(Expression::OnCluster(Box::new(cluster)));
        }
        if self.eat_kw("ENGINE") {
            self.eat(TokenKind::Equals);
            let engine: Expression = self.engine()?;
            node.properties
                .push(Expression::EngineProperty(Box::new(EngineProperty {
                    this: boxed(engine),
                })));
        }
        if self.eat_kw("COMMENT") {
            let comment: Expression = self.string_expression()?;
            node.properties
                .push(Expression::SchemaCommentProperty(Box::new(
                    SchemaCommentProperty {
                        this: boxed(comment),
                    },
                )));
        }
        Ok(Expression::CreateDatabase(Box::new(node)))
    }

    /// `ParserIdentifierWithOptionalParameters`: an engine name, with arguments in brackets.
    fn engine(&mut self) -> Parsing<Expression> {
        let parts: Vec<Identifier> = self.compound_identifier()?;
        match self.eat(TokenKind::OpeningRoundBracket) {
            true => self.function_layer(parts),
            false => Ok(crate::parse::_helpers::tree::nodes::column_from(parts)),
        }
    }

    /// `TABLE name [UUID] [ON CLUSTER] [(columns)] [AS source] [storage] [COMMENT] [AS query]`.
    fn create_table(&mut self, or_replace: bool, temporary: bool) -> Parsing<Expression> {
        self.expect_kw("TABLE")?;
        let mut node: CreateTable = CreateTable::new(String::new());
        node.or_replace = or_replace;
        node.temporary = temporary;
        node.if_not_exists = self.if_not_exists();
        node.name = self.table_name()?;
        if self.eat_kw("UUID") {
            node.uuid = Some(self.string_literal()?);
        }
        node.on_cluster = self.on_cluster()?;
        if self.eat(TokenKind::OpeningRoundBracket) {
            node.columns = self.comma_list(|parser| parser.column_declaration())?;
            self.expect_kind(TokenKind::ClosingRoundBracket)?;
        }
        if node.columns.is_empty() && self.kw("AS") && !self.subquery_or_select_at(1) {
            self.at += 1;
            let start: usize = self.offset();
            let source = self.compound_identifier()?;
            if self.eat(TokenKind::OpeningRoundBracket) {
                self.function_layer(source)?;
                self.gap("create_table_as_function", start);
            } else {
                node.clone_source = Some(table(source));
            }
        }
        node.properties = self.storage(node.properties)?;
        if self.eat_kw("COMMENT") {
            let comment: Expression = self.string_expression()?;
            node.properties
                .push(Expression::SchemaCommentProperty(Box::new(
                    SchemaCommentProperty {
                        this: boxed(comment),
                    },
                )));
        }
        if self.eat_kw("AS") {
            if self.kw("SELECT") || self.kw("WITH") || self.is(TokenKind::OpeningRoundBracket) {
                node.as_select = Some(self.select_with_union()?);
            } else {
                return Err(self.error());
            }
        }
        Ok(Expression::CreateTable(Box::new(node)))
    }

    /// A mutation's `[IN PARTITION p] WHERE e`: the partition a gap, an aliased `e` refused.
    fn mutation_predicate(&mut self, start: usize) -> Parsing<Expression> {
        if self.eat_kws(&["IN", "PARTITION"]) {
            self.partition()?;
            self.gap("alter_in_partition", start);
        }
        self.expect_kw("WHERE")?;
        let predicate: Expression = self.expression()?;
        let mut inner: &Expression = &predicate;
        while let Expression::Paren(paren) = inner {
            inner = &paren.this;
        }
        if matches!(inner, Expression::Alias(_)) {
            return Err(self.error());
        }
        Ok(predicate)
    }

    /// Whether a query starts at `offset`: `SELECT`, `WITH` or a bracketed query.
    fn subquery_or_select_at(&self, offset: usize) -> bool {
        self.kw_at(offset, "SELECT") || self.kw_at(offset, "WITH") || self.subquery_at(offset)
    }

    /// `ParserStorage`: `ENGINE`, then the key and `TTL` clauses in any order, then `SETTINGS`.
    fn storage(&mut self, mut properties: Vec<Expression>) -> Parsing<Vec<Expression>> {
        if self.eat_kw("ENGINE") {
            self.eat(TokenKind::Equals);
            let engine: Expression = self.engine()?;
            properties.push(Expression::EngineProperty(Box::new(EngineProperty {
                this: boxed(engine),
            })));
        }
        let mut seen: [bool; 5] = [false; 5];
        loop {
            let clause: Option<usize> = [
                self.kws(&["PARTITION", "BY"]),
                self.kws(&["PRIMARY", "KEY"]),
                self.kws(&["ORDER", "BY"]),
                self.kws(&["SAMPLE", "BY"]),
                self.kw("TTL"),
            ]
            .iter()
            .enumerate()
            .position(|(index, found)| *found && !seen[index]);
            let Some(index) = clause else {
                break;
            };
            seen[index] = true;
            self.at += match index {
                4 => 1,
                _ => 2,
            };
            properties.push(self.storage_clause(index)?);
        }
        if self.eat_kw("SETTINGS") {
            let expressions: Vec<Expression> = self.settings_list()?;
            properties.push(Expression::SettingsProperty(Box::new(SettingsProperty {
                expressions,
            })));
        }
        Ok(properties)
    }

    /// The storage clause `index` names in `storage`, its words consumed.
    fn storage_clause(&mut self, index: usize) -> Parsing<Expression> {
        Ok(match index {
            0 => Expression::PartitionedByProperty(Box::new(PartitionedByProperty {
                this: boxed(self.expression()?),
            })),
            1 => Expression::PrimaryKey(Box::new(PrimaryKey {
                this: None,
                expressions: vec![self.expression()?],
                options: Vec::new(),
                include: None,
            })),
            2 => Expression::OrderBy(Box::new(OrderBy {
                expressions: vec![self.storage_order()?],
                siblings: false,
                comments: Vec::new(),
                all: None,
            })),
            3 => Expression::SampleProperty(Box::new(SampleProperty {
                this: boxed(self.expression()?),
            })),
            _ => Expression::MergeTreeTTL(Box::new(MergeTreeTTL {
                expressions: self.comma_list(|parser| parser.ttl_element())?,
                where_: None,
                group: None,
                aggregates: None,
            })),
        })
    }

    /// `ParserStorageOrderByClause`: one key with an optional direction, or `[tuple](...)`.
    fn storage_order(&mut self) -> Parsing<Ordered> {
        let this: Expression = self.expression()?;
        let desc: bool = self.eat_kw("DESC") || self.eat_kw("DESCENDING");
        let asc: bool = !desc && (self.eat_kw("ASC") || self.eat_kw("ASCENDING"));
        Ok(crate::parse::_helpers::tree::nodes::ordered(
            this, desc, asc, None,
        ))
    }

    /// `ParserTTLElement`, for the `expr [DELETE]` form; moves and recompression are gaps.
    fn ttl_element(&mut self) -> Parsing<Expression> {
        let this: Expression = self.expression()?;
        let start: usize = self.offset();
        if self.eat_kw("DELETE") {
            if self.eat_kw("WHERE") {
                self.expression()?;
                self.gap("ttl_action", start);
            }
            return Ok(this);
        }
        if self.kws(&["TO", "DISK"])
            || self.kws(&["TO", "VOLUME"])
            || self.kw("RECOMPRESS")
            || self.kws(&["GROUP", "BY"])
        {
            self.skip_ttl_action()?;
            self.gap("ttl_action", start);
        }
        Ok(this)
    }

    /// The rest of a TTL action, up to the next `,` at this depth or the clause's end.
    fn skip_ttl_action(&mut self) -> Parsing<()> {
        while !self.at_end()
            && !self.is(TokenKind::Comma)
            && !self.kw("SETTINGS")
            && !self.kw("AS")
            && !self.is(TokenKind::ClosingRoundBracket)
        {
            if self.eat(TokenKind::OpeningRoundBracket) {
                self.layer_elements(TokenKind::ClosingRoundBracket)?;
                continue;
            }
            self.advance()?;
        }
        Ok(())
    }

    /// `ParserColumnDeclaration`; a table index, constraint or projection is a gap.
    fn column_declaration(&mut self) -> Parsing<ColumnDef> {
        if self.kw_in(0, &["INDEX", "CONSTRAINT", "PROJECTION"]) || self.kws(&["PRIMARY", "KEY"]) {
            let start: usize = self.offset();
            self.skip_element()?;
            self.gap("table_element", start);
            return Ok(ColumnDef::new(String::new(), DataType::Unknown));
        }
        let name: Identifier = self.ident()?;
        let typed: bool = !self.kw_in(
            0,
            &[
                "NULL",
                "NOT",
                "DEFAULT",
                "MATERIALIZED",
                "EPHEMERAL",
                "ALIAS",
                "COMMENT",
                "CODEC",
                "TTL",
            ],
        ) && !self.is(TokenKind::Comma)
            && !self.is(TokenKind::ClosingRoundBracket);
        let data_type: DataType = match typed {
            true => self.data_type()?,
            false => DataType::Unknown,
        };
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        column.no_type = !typed;
        if self.eat_kws(&["NOT", "NULL"]) {
            column.nullable = Some(false);
        } else if self.eat_kw("NULL") {
            column.nullable = Some(true);
        }
        match self.word_at(0).as_deref() {
            Some("DEFAULT") => {
                self.at += 1;
                column.default = Some(self.expression()?);
            }
            Some("MATERIALIZED") => {
                self.at += 1;
                column.materialized_expr = Some(boxed(self.expression()?));
            }
            Some("ALIAS") => {
                self.at += 1;
                column.alias_expr = Some(boxed(self.expression()?));
            }
            Some("EPHEMERAL") => {
                self.at += 1;
                let value: Option<Box<Expression>> = match self.is(TokenKind::Comma)
                    || self.is(TokenKind::ClosingRoundBracket)
                    || self.kw_in(0, &["COMMENT", "CODEC", "TTL"])
                {
                    true => None,
                    false => Some(boxed(self.expression()?)),
                };
                column.ephemeral = Some(value);
            }
            _ => {}
        }
        if self.eat_kw("COMMENT") {
            column.comment = Some(self.string_literal()?);
        }
        if self.kw("CODEC") && self.kind_at(1, TokenKind::OpeningRoundBracket) {
            let start: usize = self.offset();
            self.at += 2;
            self.layer_elements(TokenKind::ClosingRoundBracket)?;
            column.codec = Some(self.text_from(start));
        }
        if self.eat_kw("TTL") {
            column.ttl_expr = Some(boxed(self.expression()?));
        }
        Ok(column)
    }

    /// A table element this parser does not read, up to the next `,` or `)` at this depth.
    fn skip_element(&mut self) -> Parsing<()> {
        while !self.at_end()
            && !self.is(TokenKind::Comma)
            && !self.is(TokenKind::ClosingRoundBracket)
        {
            if self.eat(TokenKind::OpeningRoundBracket) {
                self.layer_elements(TokenKind::ClosingRoundBracket)?;
                continue;
            }
            self.advance()?;
        }
        Ok(())
    }

    /// `VIEW [IF NOT EXISTS] name [ON CLUSTER c] AS query`.
    fn create_view(&mut self, or_replace: bool) -> Parsing<Expression> {
        self.expect_kw("VIEW")?;
        let if_not_exists: bool = self.if_not_exists();
        let name = self.table_name()?;
        let on_cluster: Option<OnCluster> = self.on_cluster()?;
        self.expect_kw("AS")?;
        let query: Expression = self.select_with_union()?;
        let mut node: CreateView = CreateView::new(String::new(), query);
        node.name = name;
        node.or_replace = or_replace;
        node.if_not_exists = if_not_exists;
        node.on_cluster = on_cluster;
        Ok(Expression::CreateView(Box::new(node)))
    }

    /// `ParserCreateFunctionQuery`: `FUNCTION [IF NOT EXISTS] name [ON CLUSTER c] AS lambda`.
    fn create_function(&mut self, or_replace: bool) -> Parsing<Expression> {
        self.expect_kw("FUNCTION")?;
        let if_not_exists: bool = self.if_not_exists();
        let name: Identifier = self.ident()?;
        let start: usize = self.offset();
        if self.on_cluster()?.is_some() {
            self.gap("function_on_cluster", start);
        }
        self.expect_kw("AS")?;
        let body: Expression = self.expression()?;
        let Expression::Lambda(lambda) = body else {
            return Err(self.error_with("Syntax error: the function body must be a lambda", true));
        };
        let mut node: CreateFunction = CreateFunction::new(String::new());
        node.name = table(vec![name]);
        node.or_replace = or_replace;
        node.if_not_exists = if_not_exists;
        node.has_parens = true;
        node.parameters = lambda
            .parameters
            .iter()
            .map(|parameter| FunctionParameter {
                name: Some(parameter.clone()),
                data_type: DataType::Unknown,
                mode: None,
                default: None,
                mode_text: None,
                span: SourceSpan::default(),
            })
            .collect();
        node.body = Some(FunctionBody::Expression(Expression::Lambda(lambda)));
        Ok(Expression::CreateFunction(Box::new(node)))
    }

    /// `ParserDropQuery` and `ParserDropFunctionQuery`.
    pub(crate) fn drop_statement(&mut self) -> Parsing<Expression> {
        let at: usize = self.at;
        self.expect_kw("DROP")?;
        let kind: String = self.word_at(0).unwrap_or_default();
        if !["TABLE", "VIEW", "DATABASE", "FUNCTION"].contains(&kind.as_str()) {
            self.at = at;
            return self.unsupported_statement();
        }
        self.at += 1;
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let start: usize = self.offset();
        let node: Expression = match kind.as_str() {
            "DATABASE" => {
                let mut node: DropDatabase = DropDatabase::new(String::new());
                node.name = self.ident()?;
                node.if_exists = if_exists;
                Expression::DropDatabase(Box::new(node))
            }
            "FUNCTION" => {
                let mut node: DropFunction = DropFunction::new(String::new());
                node.name = table(vec![self.ident()?]);
                node.if_exists = if_exists;
                Expression::DropFunction(Box::new(node))
            }
            "VIEW" => {
                let mut node: DropView = DropView::new(String::new());
                node.name = self.table_name()?;
                node.if_exists = if_exists;
                Expression::DropView(Box::new(node))
            }
            _ => {
                let mut node: DropTable = DropTable::new(String::new());
                node.names = self.comma_list(|parser| parser.table_name())?;
                node.if_exists = if_exists;
                Expression::DropTable(Box::new(node))
            }
        };
        if self.on_cluster()?.is_some() {
            self.gap("drop_on_cluster", start);
        }
        self.eat_kw("SYNC");
        Ok(node)
    }

    /// `ParserAlterQuery` for a table: `ALTER TABLE name [ON CLUSTER c] action, ...`.
    pub(crate) fn alter_statement(&mut self) -> Parsing<Expression> {
        self.expect_kws(&["ALTER", "TABLE"])?;
        let name = self.table_name()?;
        let on_cluster: Option<OnCluster> = self.on_cluster()?;
        let actions: Vec<AlterTableAction> = self.comma_list(|parser| parser.alter_action())?;
        self.output_clauses()?;
        Ok(alter(name, actions, on_cluster))
    }

    /// One ALTER action of the forms SQLBuild emits; any other is a gap.
    fn alter_action(&mut self) -> Parsing<AlterTableAction> {
        let start: usize = self.offset();
        if self.eat_kws(&["ADD", "COLUMN"]) {
            let if_not_exists: bool = self.if_not_exists();
            let column: ColumnDef = self.column_declaration()?;
            let position: Option<ColumnPosition> = self.column_position()?;
            return Ok(AlterTableAction::AddColumn {
                column,
                if_not_exists,
                position,
            });
        }
        if self.eat_kws(&["DROP", "COLUMN"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.ident()?;
            return Ok(AlterTableAction::DropColumn {
                name,
                if_exists,
                cascade: false,
            });
        }
        if self.eat_kws(&["MODIFY", "COLUMN"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let column: ColumnDef = self.column_declaration()?;
            let position: Option<ColumnPosition> = self.column_position()?;
            return Ok(AlterTableAction::ModifyColumn {
                column,
                if_exists,
                position,
            });
        }
        if self.eat_kws(&["RENAME", "COLUMN"]) {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let old_name: Identifier = self.ident()?;
            self.expect_kw("TO")?;
            let new_name: Identifier = self.ident()?;
            return Ok(AlterTableAction::RenameColumn {
                old_name,
                new_name,
                if_exists,
            });
        }
        if self.eat_kw("DELETE") {
            let where_clause: Expression = self.mutation_predicate(start)?;
            return Ok(AlterTableAction::Delete { where_clause });
        }
        if self.eat_kw("UPDATE") {
            self.comma_list(|parser| {
                parser.ident()?;
                parser.expect_kind(TokenKind::Equals)?;
                parser.expression()
            })?;
            self.mutation_predicate(start)?;
            self.opaque = true;
            return Ok(AlterTableAction::Raw {
                sql: self.text_from(start),
            });
        }
        if self.eat_kws(&["REPLACE", "PARTITION"]) {
            let partition: Expression = self.partition()?;
            self.expect_kw("FROM")?;
            let source: Expression = Expression::Table(Box::new(self.table_name()?));
            return Ok(AlterTableAction::ReplacePartition {
                partition,
                source: Some(boxed(source)),
            });
        }
        if self.kws(&["DROP", "PARTITION"]) || self.kws(&["ATTACH", "PARTITION"]) {
            self.at += 2;
            self.partition()?;
            if self.eat_kw("FROM") {
                self.table_name()?;
            }
            return Ok(AlterTableAction::Raw {
                sql: self.text_from(start),
            });
        }
        if self.kws(&["MODIFY", "COMMENT"]) {
            self.at += 2;
            self.string_literal()?;
            return Ok(AlterTableAction::Raw {
                sql: self.text_from(start),
            });
        }
        if self.kws(&["MODIFY", "TTL"]) {
            self.at += 2;
            self.comma_list(|parser| parser.ttl_element())?;
            return Ok(AlterTableAction::Raw {
                sql: self.text_from(start),
            });
        }
        if !self.is(TokenKind::BareWord) {
            return Err(self.error());
        }
        self.skip_element()?;
        self.gap("alter_action", start);
        Ok(AlterTableAction::Raw {
            sql: self.text_from(start),
        })
    }

    /// `FIRST` or `AFTER name`, if next.
    fn column_position(&mut self) -> Parsing<Option<ColumnPosition>> {
        if self.eat_kw("FIRST") {
            return Ok(Some(ColumnPosition::First));
        }
        if self.eat_kw("AFTER") {
            return Ok(Some(ColumnPosition::After(self.ident()?)));
        }
        Ok(None)
    }
}
