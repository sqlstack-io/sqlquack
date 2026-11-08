//! `ALTER TABLE`, `ALTER VIEW` and `ALTER SCHEMA`, with Databricks' own clauses.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    AlterColumnAction, AlterTable, AlterTableAction, ColumnDef, ColumnPosition,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{
    AlterObject, AlterView, AlterViewAction,
};
use sqlquack_syntax::expressions::tables::models::Partition;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{binary, column_from, joined, null, table, var};
use crate::parse::constants::{ALTER_COLUMN_WORDS, ALTER_KINDS, SET_TAGS};

/// The text of a property key or value: a string's contents, or a name or number as written.
pub(crate) fn property_text(expression: &Expression) -> String {
    match expression {
        Expression::Literal(literal) => literal.value_str().to_owned(),
        Expression::Identifier(name) => name.name.clone(),
        Expression::Boolean(value) => match value.value {
            true => "true".to_owned(),
            false => "false".to_owned(),
        },
        other => other.to_string(),
    }
}

/// A property as its key and value; a key without a value has a null value.
pub(crate) fn property_pair(property: Expression) -> (Expression, Expression) {
    match property {
        Expression::Eq(mut pair) => (
            std::mem::replace(&mut pair.left, null()),
            std::mem::replace(&mut pair.right, null()),
        ),
        key => (key, null()),
    }
}

/// A `partitionSpec` as a `Partition` of `name = value` equalities; a bare name stays a column.
pub(crate) fn partition(spec: Vec<(Identifier, Option<Expression>)>) -> Expression {
    let expressions: Vec<Expression> = spec
        .into_iter()
        .map(|(name, value)| {
            let column: Expression = column_from(vec![name]);
            match value {
                Some(value) => binary(Expression::Eq, column, value),
                None => column,
            }
        })
        .collect();
    Expression::Partition(Box::new(Partition {
        expressions,
        subpartition: false,
    }))
}

/// A `partitionSpec` as name and value pairs; a bare name has a null value.
fn partition_pairs(spec: Vec<(Identifier, Option<Expression>)>) -> Vec<(Identifier, Expression)> {
    spec.into_iter()
        .map(|(name, value)| (name, value.unwrap_or_else(null)))
        .collect()
}

impl Parser<'_> {
    /// `ALTER TABLE`, `ALTER VIEW` and `ALTER SCHEMA`; another object kind is an opaque gap.
    pub(crate) fn alter_statement(&mut self) -> Parsing<Expression> {
        let start_at: usize = self.at;
        self.expect_kw("ALTER")?;
        match self.word_at(0).as_deref() {
            Some("TABLE") => self.alter_table(),
            Some("VIEW") => self.alter_view(),
            Some("SCHEMA" | "DATABASE" | "NAMESPACE") => self.alter_schema(),
            Some(kind) if ALTER_KINDS.contains(&kind) => {
                self.at = start_at;
                self.opaque_statement()
            }
            _ if self.is(is_identifier)
                && (self.kw_at(1, "OWNER") || (self.kw_at(1, "SET") && self.kw_at(2, "OWNER"))) =>
            {
                let start: usize = self.tokens[start_at].start;
                self.at += 1;
                self.eat_kw("SET");
                self.expect_kw("OWNER")?;
                self.expect_kw("TO")?;
                self.principal()?;
                self.gap("alter_owner", start);
                Ok(null())
            }
            _ => Err(self.error()),
        }
    }

    /// A clause the shared tree cannot hold, already consumed from `start`: recorded as `construct`.
    fn alter_gap(
        &mut self,
        construct: &'static str,
        start: usize,
    ) -> Parsing<Option<AlterTableAction>> {
        self.gap(construct, start);
        Ok(None)
    }

    /// `propertyList` whose every key has a value, refused as Spark refuses one without.
    pub(crate) fn valued_property_list(&mut self) -> Parsing<Vec<(Expression, Expression)>> {
        let at: usize = self.offset();
        let properties: Vec<Expression> = self.property_list()?;
        if properties
            .iter()
            .any(|property| !matches!(property, Expression::Eq(_)))
        {
            return Err(self.error_with(
                "Operation not allowed: Values must be specified for key(s)",
                at,
                true,
            ));
        }
        Ok(properties.into_iter().map(property_pair).collect())
    }

    /// `propertyList` of keys only, refused as Spark refuses one with a value.
    pub(crate) fn key_property_list(&mut self) -> Parsing<Vec<String>> {
        let at: usize = self.offset();
        let properties: Vec<Expression> = self.property_list()?;
        if properties
            .iter()
            .any(|property| matches!(property, Expression::Eq(_)))
        {
            return Err(self.error_with(
                "Operation not allowed: Values should not be specified for key(s)",
                at,
                true,
            ));
        }
        Ok(properties.iter().map(property_text).collect())
    }

    /// A principal: a user, group or service principal name.
    fn principal(&mut self) -> Parsing<Identifier> {
        self.ident()
    }

    fn alter_table(&mut self) -> Parsing<Expression> {
        self.expect_kw("TABLE")?;
        let mut node: AlterTable = AlterTable::new(String::new());
        node.name = self.table_reference()?;
        let start: usize = self.offset();
        let node: AlterTable = match self.kw("PARTITION") {
            true => self.partition_alter(node, start)?,
            false => {
                node.actions.extend(self.alter_table_action(start)?);
                node
            }
        };
        Ok(Expression::AlterTable(Box::new(node)))
    }

    /// An action after `ALTER TABLE t PARTITION (...)`: a rename, a location, a SerDe or columns.
    fn partition_alter(&mut self, mut node: AlterTable, start: usize) -> Parsing<AlterTable> {
        let spec = self.partition_spec()?;
        if self.eat_kw("RENAME") {
            self.expect_kw("TO")?;
            self.partition_spec()?;
            self.gap("rename_partition", start);
        } else if self.kw("SET") && self.kw_at(1, "LOCATION") {
            self.at += 2;
            let location: String = self.string_lit()?;
            node.partition = Some(partition_pairs(spec));
            node.actions
                .push(AlterTableAction::SetLocation { location });
        } else if self.kw("SET") && self.kw_in(1, &["SERDE", "SERDEPROPERTIES"]) {
            self.serde_clause()?;
            self.gap("serde", start);
        } else if self.eat_kw("CHANGE") {
            self.eat_kw("COLUMN");
            self.hive_change_column()?;
            self.gap("partition_change_column", start);
        } else if self.kw("REPLACE") && self.kw_at(1, "COLUMNS") {
            self.at += 2;
            self.parenthesized(|parser| parser.added_column())?;
            self.gap("replace_columns", start);
        } else {
            return Err(self.error());
        }
        Ok(node)
    }

    /// `SET SERDE 'class' [WITH SERDEPROPERTIES (...)]` or `SET SERDEPROPERTIES (...)`.
    fn serde_clause(&mut self) -> Parsing<()> {
        self.expect_kw("SET")?;
        if self.eat_kw("SERDE") {
            self.string_lit()?;
            if self.eat_kws(&["WITH", "SERDEPROPERTIES"]) {
                self.property_list()?;
            }
            return Ok(());
        }
        self.expect_kw("SERDEPROPERTIES")?;
        self.property_list()?;
        Ok(())
    }

    /// The actions of one `ALTER TABLE` clause; a clause the shared tree cannot hold is a gap.
    fn alter_table_action(&mut self, start: usize) -> Parsing<Vec<AlterTableAction>> {
        match self.word_at(0).as_deref() {
            Some("ADD") => self.alter_table_add(),
            Some("DROP") => self.alter_table_drop(start),
            Some("ALTER" | "CHANGE") => self.alter_table_column(start),
            _ => Ok(Vec::from_iter(self.single_table_action(start)?)),
        }
    }

    /// An `ALTER TABLE` clause of one action; a clause the shared tree cannot hold yields none.
    fn single_table_action(&mut self, start: usize) -> Parsing<Option<AlterTableAction>> {
        let Some(word) = self.word_at(0) else {
            return Err(self.error());
        };
        match word.as_str() {
            "RENAME" if self.kw_at(1, "COLUMN") => {
                self.at += 2;
                let old_name: Identifier = joined(self.multipart_identifier()?);
                self.expect_kw("TO")?;
                let new_name: Identifier = self.ident()?;
                Ok(Some(AlterTableAction::RenameColumn {
                    old_name,
                    new_name,
                    if_exists: false,
                }))
            }
            "RENAME" => {
                self.at += 1;
                self.expect_kw("TO")?;
                let to: TableRef = table(self.multipart_identifier()?);
                Ok(Some(AlterTableAction::RenameTable(to)))
            }
            "SET" => self.alter_table_set(start),
            "UNSET" => self.alter_table_unset(start),
            "OWNER" => {
                self.at += 1;
                self.expect_kw("TO")?;
                self.principal()?;
                self.alter_gap("owner", start)
            }
            "CLUSTER" if self.kw_at(1, "BY") => {
                self.at += 2;
                let expressions: Vec<Expression> =
                    match self.kw_in(0, &["NONE", "AUTO"]) && !self.char_at(1, '(') {
                        true => {
                            let word: String = self.word_at(0).unwrap_or_default();
                            self.at += 1;
                            vec![var(&word)]
                        }
                        false => self.parenthesized(|parser| {
                            parser.multipart_identifier().map(column_from)
                        })?,
                    };
                Ok(Some(AlterTableAction::ClusterBy { expressions }))
            }
            "DEFAULT" if self.kw_at(1, "COLLATION") => {
                self.at += 2;
                self.ident()?;
                self.alter_gap("table_collation", start)
            }
            "RECOVER" => {
                self.at += 1;
                self.expect_kw("PARTITIONS")?;
                self.alter_gap("recover_partitions", start)
            }
            "REPLACE" if self.kw_at(1, "COLUMNS") => {
                self.at += 2;
                self.parenthesized(|parser| parser.added_column())?;
                self.alter_gap("replace_columns", start)
            }
            "REPLACE" => {
                self.at += 1;
                self.expect_kw("PARTITIONED")?;
                self.expect_kw("BY")?;
                self.expect_kw("WITH")?;
                self.expect_kw("CLUSTER")?;
                self.expect_kw("BY")?;
                if self.kw_in(0, &["AUTO", "NONE"]) {
                    self.at += 1;
                } else if self.is_char('(') {
                    self.parenthesized(|parser| parser.expression())?;
                }
                self.alter_gap("replace_partitioning", start)
            }
            "ENABLE" | "DISABLE" | "INHERIT" => {
                self.at += 1;
                self.expect_kw("PREDICTIVE")?;
                self.expect_kw("OPTIMIZATION")?;
                self.alter_gap("predictive_optimization", start)
            }
            _ => Err(self.error()),
        }
    }

    /// `ADD COLUMN[S] ...`, `ADD [IF NOT EXISTS] PARTITION ...` and `ADD` of a table constraint.
    fn alter_table_add(&mut self) -> Parsing<Vec<AlterTableAction>> {
        self.expect_kw("ADD")?;
        if self.eat_kw("COLUMN") || self.eat_kw("COLUMNS") {
            let columns = match self.is_char('(') {
                true => self.parenthesized(|parser| parser.added_column())?,
                false => self.comma_list(|parser| parser.added_column())?,
            };
            return Ok(columns
                .into_iter()
                .map(|(column, position)| AlterTableAction::AddColumn {
                    column,
                    if_not_exists: false,
                    position,
                })
                .collect());
        }
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        if if_not_exists || self.kw("PARTITION") {
            let mut added: Vec<AlterTableAction> = Vec::new();
            loop {
                let spec = self.partition_spec()?;
                let location: Option<Expression> = match self.eat_kw("LOCATION") {
                    true => Some(self.string_literal()?),
                    false => None,
                };
                added.push(AlterTableAction::AddPartition {
                    partition: partition(spec),
                    if_not_exists,
                    location,
                });
                if !self.kw("PARTITION") {
                    break;
                }
            }
            return Ok(added);
        }
        let constraint = self.table_constraint()?;
        let validation: usize = self.offset();
        let mut validated: bool = false;
        while self.kw_in(0, &["ENABLE", "DISABLE", "VALIDATE", "NOVALIDATE"]) {
            self.at += 1;
            validated = true;
        }
        if validated {
            self.gap("constraint_validation", validation);
        }
        Ok(vec![AlterTableAction::AddConstraint(constraint)])
    }

    /// `qualifiedColTypeWithPosition`, with Databricks' `COLLATE` and `MASK` descriptors.
    fn added_column(&mut self) -> Parsing<(ColumnDef, Option<ColumnPosition>)> {
        let name: Identifier = joined(self.multipart_identifier()?);
        let data_type: DataType = self.data_type()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        let mut position: Option<ColumnPosition> = None;
        loop {
            let start: usize = self.offset();
            if (self.kw("NOT") || self.is_char('!')) && self.kw_at(1, "NULL") {
                self.at += 2;
                column.nullable = Some(false);
            } else if self.eat_kw("DEFAULT") {
                column.default = Some(self.expression()?);
            } else if self.eat_kw("COMMENT") {
                column.comment = Some(self.string_lit()?);
            } else if self.kw_in(0, &["FIRST", "AFTER"]) {
                position = Some(self.column_position()?);
            } else if self.eat_kw("MASK") {
                self.column_mask()?;
                self.gap("column_mask", start);
            } else {
                return Ok((column, position));
            }
        }
    }

    /// `colPosition`: `FIRST` or `AFTER column`.
    fn column_position(&mut self) -> Parsing<ColumnPosition> {
        match self.eat_kw("FIRST") {
            true => Ok(ColumnPosition::First),
            false => {
                self.expect_kw("AFTER")?;
                Ok(ColumnPosition::After(self.ident()?))
            }
        }
    }

    /// A column mask after `MASK`: `function [USING COLUMNS (expr, ...)]`.
    pub(crate) fn column_mask(&mut self) -> Parsing<()> {
        self.multipart_identifier()?;
        if self.eat_kws(&["USING", "COLUMNS"]) {
            self.parenthesized(|parser| parser.expression())?;
        }
        Ok(())
    }

    /// `DROP` of columns, partitions, constraints, keys, a feature or a row filter.
    fn alter_table_drop(&mut self, start: usize) -> Parsing<Vec<AlterTableAction>> {
        self.expect_kw("DROP")?;
        if self.eat_kw("COLUMN") || self.eat_kw("COLUMNS") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let names: Vec<Vec<Identifier>> = match self.is_char('(') {
                true => self.parenthesized(|parser| parser.multipart_identifier())?,
                false => self.comma_list(|parser| parser.multipart_identifier())?,
            };
            let dropped: Vec<AlterTableAction> = names
                .into_iter()
                .map(|name| AlterTableAction::DropColumn {
                    name: joined(name),
                    if_exists,
                    cascade: false,
                })
                .collect();
            return Ok(dropped);
        }
        if self.eat_kw("CONSTRAINT") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.ident()?;
            let behaviour: usize = self.offset();
            if self.eat_kw("CASCADE") {
                self.gap("drop_constraint_cascade", behaviour);
            } else {
                self.eat_kw("RESTRICT");
            }
            return Ok(vec![AlterTableAction::DropConstraint { name, if_exists }]);
        }
        if self.kw("PRIMARY") {
            self.at += 1;
            self.expect_kw("KEY")?;
            self.eat_kws(&["IF", "EXISTS"]);
            if !self.eat_kw("RESTRICT") {
                self.eat_kw("CASCADE");
            }
            return self
                .alter_gap("drop_primary_key", start)
                .map(Vec::from_iter);
        }
        if self.kw("FOREIGN") {
            self.at += 1;
            self.expect_kw("KEY")?;
            self.eat_kws(&["IF", "EXISTS"]);
            self.identifier_list()?;
            return self
                .alter_gap("drop_foreign_key", start)
                .map(Vec::from_iter);
        }
        if self.eat_kw("FEATURE") {
            match self.string_at(0) {
                true => {
                    self.string_lit()?;
                }
                false => {
                    self.ident()?;
                }
            }
            if self.eat_kw("TRUNCATE") {
                self.expect_kw("HISTORY")?;
            }
            return self.alter_gap("drop_feature", start).map(Vec::from_iter);
        }
        if self.eat_kw("ROW") {
            self.expect_kw("FILTER")?;
            return self.alter_gap("row_filter", start).map(Vec::from_iter);
        }
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        if !self.kw("PARTITION") {
            return Err(self.error());
        }
        let mut partitions: Vec<Vec<(Identifier, Expression)>> =
            vec![partition_pairs(self.partition_spec()?)];
        while self.eat_char(',') {
            partitions.push(partition_pairs(self.partition_spec()?));
        }
        let purge: usize = self.offset();
        if self.eat_kw("PURGE") {
            self.gap("drop_partition_purge", purge);
        }
        Ok(vec![AlterTableAction::DropPartition {
            partitions,
            if_exists,
        }])
    }

    /// `{ALTER | CHANGE} [COLUMN] spec, ...`, or Hive's `CHANGE [COLUMN] name newName type`.
    fn alter_table_column(&mut self, start: usize) -> Parsing<Vec<AlterTableAction>> {
        let change: bool = self.kw("CHANGE");
        self.at += 1;
        let column_keyword: bool = self.eat_kw("COLUMN");
        if change && self.hive_change_ahead() {
            return self.hive_change_column().map(|action| vec![action]);
        }
        if !change && column_keyword && self.tag_column_ahead() {
            self.multipart_identifier()?;
            self.at += 1;
            self.expect_kw("TAGS")?;
            self.property_list()?;
            return self.alter_gap("column_tags", start).map(Vec::from_iter);
        }
        let mut altered: Vec<AlterTableAction> = Vec::new();
        loop {
            let spec_start: usize = self.offset();
            let name: Identifier = joined(self.multipart_identifier()?);
            match self.column_action(spec_start)? {
                Some(action) => altered.push(AlterTableAction::AlterColumn {
                    name,
                    action,
                    use_modify_keyword: false,
                }),
                None if self.opaque => {}
                None => {
                    return Err(self.error_with(
                        "Operation not allowed: ALTER TABLE table ALTER COLUMN requires a TYPE, \
                         a SET/DROP, a COMMENT, or a FIRST/AFTER",
                        spec_start,
                        true,
                    ));
                }
            }
            if !self.eat_char(',') {
                break;
            }
        }
        Ok(altered)
    }

    /// Whether `name SET|UNSET TAGS` follows, which only `ALTER COLUMN` takes.
    fn tag_column_ahead(&self) -> bool {
        let mut offset: usize = 1;
        while self.char_at(offset, '.') && self.is_at(offset + 1, is_identifier) {
            offset += 2;
        }
        self.kw_in(offset, &["SET", "UNSET"]) && self.kw_at(offset + 1, "TAGS")
    }

    /// Whether Hive's `CHANGE name newName type` follows: a second name that begins no action.
    fn hive_change_ahead(&self) -> bool {
        let mut offset: usize = 1;
        while self.char_at(offset, '.') && self.is_at(offset + 1, is_identifier) {
            offset += 2;
        }
        self.is_at(offset, is_identifier)
            && !self
                .word_at(offset)
                .is_some_and(|word| ALTER_COLUMN_WORDS.contains(&word.as_str()))
    }

    /// Hive's `name colType [colPosition]` after `CHANGE [COLUMN]`.
    fn hive_change_column(&mut self) -> Parsing<AlterTableAction> {
        let old_name: Identifier = joined(self.multipart_identifier()?);
        let start: usize = self.offset();
        let new_name: Identifier = self.ident()?;
        let data_type: DataType = self.data_type()?;
        if (self.kw("NOT") || self.is_char('!')) && self.kw_at(1, "NULL") {
            self.at += 2;
            self.gap("change_column_not_null", start);
        }
        let comment: Option<String> = match self.eat_kw("COMMENT") {
            true => Some(self.string_lit()?),
            false => None,
        };
        if self.kw_in(0, &["FIRST", "AFTER"]) {
            self.column_position()?;
            self.gap("change_column_position", start);
        }
        Ok(AlterTableAction::ChangeColumn {
            old_name,
            new_name,
            data_type: Some(data_type),
            comment,
            cascade: false,
        })
    }

    /// `alterColumnAction`, with Databricks' masks and identity sync; none if absent or a gap.
    fn column_action(&mut self, start: usize) -> Parsing<Option<AlterColumnAction>> {
        if self.eat_kw("TYPE") {
            let data_type: DataType = self.data_type()?;
            return Ok(Some(AlterColumnAction::SetDataType {
                data_type,
                using: None,
                collate: None,
            }));
        }
        if self.eat_kw("COMMENT") {
            return Ok(Some(AlterColumnAction::Comment(self.string_lit()?)));
        }
        if self.kw_in(0, &["FIRST", "AFTER"]) {
            self.column_position()?;
            self.gap("alter_column_position", start);
            return Ok(None);
        }
        if self.kw_in(0, &["SET", "DROP"]) && (self.kw_at(1, "NOT") || self.char_at(1, '!')) {
            let set: bool = self.kw("SET");
            self.at += 2;
            self.expect_kw("NULL")?;
            return Ok(Some(match set {
                true => AlterColumnAction::SetNotNull,
                false => AlterColumnAction::DropNotNull,
            }));
        }
        if self.kw("SET") && self.kw_at(1, "DEFAULT") {
            self.at += 2;
            return Ok(Some(AlterColumnAction::SetDefault(self.expression()?)));
        }
        if self.kw("DROP") && self.kw_at(1, "DEFAULT") {
            self.at += 2;
            return Ok(Some(AlterColumnAction::DropDefault));
        }
        if self.kw("SET") && self.kw_at(1, "MASK") {
            self.at += 2;
            self.column_mask()?;
            self.gap("column_mask", start);
            return Ok(None);
        }
        if self.kw("DROP") && self.kw_at(1, "MASK") {
            self.at += 2;
            self.gap("column_mask", start);
            return Ok(None);
        }
        if self.kw("SYNC") {
            self.at += 1;
            self.expect_kw("IDENTITY")?;
            self.gap("identity_sync", start);
            return Ok(None);
        }
        Ok(None)
    }

    /// `SET` of table properties, a location, a SerDe, tags, an owner, a row filter or management.
    fn alter_table_set(&mut self, start: usize) -> Parsing<Option<AlterTableAction>> {
        if self.kw_at(1, "SERDE") || self.kw_at(1, "SERDEPROPERTIES") {
            self.serde_clause()?;
            return self.alter_gap("serde", start);
        }
        self.expect_kw("SET")?;
        let Some(word) = self.word_at(0) else {
            return Err(self.error());
        };
        self.at += 1;
        match word.as_str() {
            "TBLPROPERTIES" => Ok(Some(AlterTableAction::SetTableProperties {
                properties: self.valued_property_list()?,
            })),
            "LOCATION" => Ok(Some(AlterTableAction::SetLocation {
                location: self.string_lit()?,
            })),
            "TAGS" => {
                let expressions: Vec<(String, Expression)> = self
                    .property_list()?
                    .into_iter()
                    .map(property_pair)
                    .map(|(key, value)| (property_text(&key), value))
                    .collect();
                Ok(Some(AlterTableAction::SetTag { expressions }))
            }
            "OWNER" => {
                self.expect_kw("TO")?;
                self.principal()?;
                self.alter_gap("owner", start)
            }
            "ROW" => {
                self.expect_kw("FILTER")?;
                self.multipart_identifier()?;
                self.expect_kw("ON")?;
                self.expect_char('(')?;
                if !self.is_char(')') {
                    self.comma_list(|parser| parser.expression())?;
                }
                self.expect_char(')')?;
                self.alter_gap("row_filter", start)
            }
            "MANAGED" => {
                if !self.eat_kw("COPY") && !self.eat_kw("MOVE") && self.eat_kw("TRUNCATE") {
                    self.expect_kw("UNIFORM")?;
                    self.expect_kw("HISTORY")?;
                }
                self.alter_gap("managed", start)
            }
            "EXTERNAL" => {
                if self.eat_kw("DRY") {
                    self.expect_kw("RUN")?;
                }
                self.alter_gap("managed", start)
            }
            _ => {
                self.at -= 1;
                Err(self.error())
            }
        }
    }

    /// `UNSET` of table properties, tags or management.
    fn alter_table_unset(&mut self, start: usize) -> Parsing<Option<AlterTableAction>> {
        self.expect_kw("UNSET")?;
        if self.eat_kw("TBLPROPERTIES") {
            let if_exists: usize = self.offset();
            if self.eat_kws(&["IF", "EXISTS"]) {
                self.gap("unset_if_exists", if_exists);
            }
            return Ok(Some(AlterTableAction::UnsetProperty {
                properties: self.key_property_list()?,
            }));
        }
        if self.eat_kw("TAGS") {
            let names: Vec<String> = self.property_list()?.iter().map(property_text).collect();
            return Ok(Some(AlterTableAction::UnsetTag { names }));
        }
        self.expect_kw("MANAGED")?;
        if self.eat_kw("TRUNCATE") {
            self.expect_kw("UNIFORM")?;
            self.expect_kw("HISTORY")?;
        }
        self.alter_gap("managed", start)
    }

    fn alter_view(&mut self) -> Parsing<Expression> {
        self.expect_kw("VIEW")?;
        let mut node: AlterView = AlterView::new(String::new());
        node.name = self.table_reference()?;
        let start: usize = self.offset();
        let action: Option<AlterViewAction> = match self.word_at(0).as_deref() {
            Some("RENAME") => {
                self.at += 1;
                self.expect_kw("TO")?;
                let to: TableRef = table(self.multipart_identifier()?);
                Some(AlterViewAction::Rename(Box::new(to)))
            }
            Some("SET") if self.kw_at(1, "TBLPROPERTIES") => {
                self.at += 2;
                let properties: Vec<(String, String)> = self
                    .valued_property_list()?
                    .iter()
                    .map(|(key, value)| (property_text(key), property_text(value)))
                    .collect();
                Some(AlterViewAction::SetTblproperties(properties))
            }
            Some("UNSET") if self.kw_at(1, "TBLPROPERTIES") => {
                self.at += 2;
                let if_exists: usize = self.offset();
                if self.eat_kws(&["IF", "EXISTS"]) {
                    self.gap("unset_if_exists", if_exists);
                }
                Some(AlterViewAction::UnsetTblproperties(
                    self.key_property_list()?,
                ))
            }
            Some("OWNER") => {
                self.at += 1;
                self.expect_kw("TO")?;
                Some(AlterViewAction::OwnerTo(self.principal()?))
            }
            Some("SET") if self.kw_at(1, "OWNER") => {
                self.at += 2;
                self.expect_kw("TO")?;
                Some(AlterViewAction::OwnerTo(self.principal()?))
            }
            Some("SET" | "UNSET") if self.kw_at(1, "TAGS") => {
                self.at += 2;
                self.property_list()?;
                self.gap("view_tags", start);
                None
            }
            Some("SET") if self.kw_at(1, "MANAGED") => {
                self.at += 2;
                self.gap("managed", start);
                None
            }
            Some("WITH") if self.kw_at(1, "SCHEMA") => {
                self.at += 2;
                if !(self.eat_kw("BINDING")
                    || self.eat_kw("COMPENSATION")
                    || self.eat_kw("EVOLUTION")
                    || self.eat_kws(&["TYPE", "EVOLUTION"]))
                {
                    return Err(self.error());
                }
                self.gap("schema_binding", start);
                None
            }
            Some("ADD" | "DROP") if self.kw_at(1, "CONSTRAINT") => {
                self.at += 2;
                self.ident()?;
                self.skip_statement_rest()?;
                self.gap("view_constraint", start);
                None
            }
            Some("ADD") if self.kw_at(1, "PARTITION") || self.kw_at(1, "IF") => {
                self.at += 1;
                self.eat_kws(&["IF", "NOT", "EXISTS"]);
                loop {
                    self.partition_spec()?;
                    if self.eat_kw("LOCATION") {
                        self.string_lit()?;
                    }
                    if !self.kw("PARTITION") {
                        break;
                    }
                }
                self.gap("view_partition", start);
                None
            }
            Some("DROP") if self.kw_at(1, "PARTITION") || self.kw_at(1, "IF") => {
                self.at += 1;
                self.eat_kws(&["IF", "EXISTS"]);
                self.comma_list(|parser| parser.partition_spec())?;
                self.eat_kw("PURGE");
                self.gap("view_partition", start);
                None
            }
            _ => {
                self.eat_kw("AS");
                Some(AlterViewAction::AsSelect(Box::new(self.query()?)))
            }
        };
        node.actions.extend(action);
        Ok(Expression::AlterView(Box::new(node)))
    }

    /// `ALTER SCHEMA` as an `AlterObject` whose action names the clause and holds its operands.
    fn alter_schema(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let name: TableRef = table(self.multipart_identifier()?);
        let (action, operands): (String, Vec<Expression>) = match self.word_at(0).as_deref() {
            Some(word @ ("SET" | "UNSET")) if self.kw_in(1, &["DBPROPERTIES", "PROPERTIES"]) => {
                let action: String = format!("{word} {}", self.word_at(1).unwrap_or_default());
                self.at += 2;
                (action, self.property_list()?)
            }
            Some(word @ ("SET" | "UNSET")) if self.kw_at(1, "TAGS") => {
                self.at += 2;
                let set: bool = word == SET_TAGS;
                let tags: Vec<Expression> = self.parenthesized(|parser| {
                    let key: Expression =
                        Expression::Identifier(joined(parser.multipart_identifier()?));
                    match set {
                        true => {
                            parser.expect_char('=')?;
                            let value: Expression = parser.primary_expression()?;
                            Ok(binary(Expression::Eq, key, value))
                        }
                        false => Ok(key),
                    }
                })?;
                (format!("{word} TAGS"), tags)
            }
            Some("RETAIN" | "SET") if self.kw_at(0, "RETAIN") || self.kw_at(1, "RETAIN") => {
                self.eat_kw("SET");
                self.expect_kw("RETAIN")?;
                self.expect_kw("DROPPED")?;
                self.expect_kw("TO")?;
                let count: String = self
                    .expect_token(|token| {
                        token.kind
                            == crate::scan::models::TokenKind::Number(
                                crate::scan::models::NumberKind::Integer,
                            )
                    })?
                    .value;
                let unit: String = match self.word_at(0) {
                    Some(unit)
                        if ["HOUR", "HOURS", "DAY", "DAYS", "WEEK", "WEEKS"]
                            .contains(&unit.as_str()) =>
                    {
                        unit
                    }
                    _ => return Err(self.error()),
                };
                self.at += 1;
                (
                    "RETAIN DROPPED TO".to_owned(),
                    vec![var(&format!("{count} {unit}"))],
                )
            }
            Some("SET") if self.kw_at(1, "LOCATION") => {
                self.at += 2;
                ("SET LOCATION".to_owned(), vec![self.string_literal()?])
            }
            Some("SET") if self.kw_at(1, "MANAGED") && self.kw_at(2, "LOCATION") => {
                self.at += 3;
                (
                    "SET MANAGED LOCATION".to_owned(),
                    vec![self.string_literal()?],
                )
            }
            Some("DEFAULT") if self.kw_at(1, "COLLATION") => {
                self.at += 2;
                let collation: Identifier = self.ident()?;
                (
                    "DEFAULT COLLATION".to_owned(),
                    vec![Expression::Identifier(collation)],
                )
            }
            Some("OWNER") | Some("SET") if self.kw_at(1, "TO") || self.kw_at(1, "OWNER") => {
                self.eat_kw("SET");
                self.expect_kw("OWNER")?;
                self.expect_kw("TO")?;
                (
                    "OWNER TO".to_owned(),
                    vec![Expression::Identifier(self.principal()?)],
                )
            }
            Some(word @ ("ENABLE" | "DISABLE" | "INHERIT")) => {
                let action: String = format!("{word} PREDICTIVE OPTIMIZATION");
                self.at += 1;
                self.expect_kw("PREDICTIVE")?;
                self.expect_kw("OPTIMIZATION")?;
                (action, Vec::new())
            }
            _ => return Err(self.error()),
        };
        Ok(Expression::AlterObject(Box::new(AlterObject {
            kind: "SCHEMA".to_owned(),
            name,
            arguments: Vec::new(),
            if_exists: false,
            action,
            operands,
            properties: Vec::new(),
        })))
    }
}
