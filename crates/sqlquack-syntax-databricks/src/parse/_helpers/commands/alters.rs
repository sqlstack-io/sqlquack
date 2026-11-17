//! `ALTER TABLE`, `ALTER VIEW` and `ALTER SCHEMA`, with Databricks' own clauses.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    AlterColumnAction, AlterTable, AlterTableAction, ColumnDef, ColumnPosition,
    MaskingPolicyColumnConstraint,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{
    AlterObject, AlterView, AlterViewAction,
};
use sqlquack_syntax::expressions::tables::models::Partition;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_identifier};
use crate::parse::_helpers::tree::nodes::{
    binary, collate, column_from, joined, null, property, table, too_deep_to_keep, tuple, var,
};
use crate::parse::constants::{
    ALTER_COLUMN_WORDS, ALTER_KINDS, COLUMN_DROP_MASK, COLUMN_POSITION, COLUMN_SET_MASK,
    COLUMN_SET_TAGS, COLUMN_SYNC_IDENTITY, COLUMN_UNSET_TAGS, DEFAULT_COLLATION, DROP_FEATURE,
    DROP_FOREIGN_KEY, DROP_PRIMARY_KEY, DROP_ROW_FILTER, KEYED_ACTIONS, OWNER_TO,
    PREDICTIVE_OPTIMIZATION, RECOVER_PARTITIONS, RENAME_PARTITION, REPLACE_PARTITIONING,
    SET_EXTERNAL, SET_MANAGED, SET_OWNER_TO, SET_ROW_FILTER, SET_SERDE, SET_SERDEPROPERTIES,
    SET_TAGS, TRUNCATE_UNIFORM_HISTORY, UNSET_MANAGED,
};

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

/// An `ALTER VIEW` action with no shared variant, as an `AlterObject` of kind `VIEW`.
fn view_object(
    name: TableRef,
    action: String,
    if_exists: bool,
    operands: Vec<Expression>,
) -> Expression {
    Expression::AlterObject(Box::new(AlterObject {
        kind: "VIEW".to_owned(),
        name: Some(name),
        action,
        operand_if_exists: if_exists,
        operands,
        ..Default::default()
    }))
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

    /// The action `key` of `KEYED_ACTIONS` holding `values`, read from `start` as `construct`.
    fn keyed_action(
        &mut self,
        (construct, start): (&'static str, usize),
        key: &str,
        mut values: Vec<Expression>,
    ) -> Parsing<Option<AlterTableAction>> {
        if !KEYED_ACTIONS.contains(&key) || values.iter().any(too_deep_to_keep) {
            return self.alter_gap(construct, start);
        }
        self.treed(construct, start);
        let value: Expression = match values.len() {
            0 => null(),
            1 => values.remove(0),
            _ => tuple(values),
        };
        Ok(Some(AlterTableAction::SetProperty {
            properties: vec![(key.to_owned(), value)],
        }))
    }

    /// `propertyList` whose every key has a value, refused as Spark refuses one without.
    pub(crate) fn valued_property_list(&mut self) -> Parsing<Vec<(Expression, Expression)>> {
        Ok(self
            .spelled_valued_property_list()?
            .into_iter()
            .map(|(pair, _)| pair)
            .collect())
    }

    /// `valued_property_list`, each pair with whether its value followed the key without `=`.
    pub(crate) fn spelled_valued_property_list(
        &mut self,
    ) -> Parsing<Vec<((Expression, Expression), bool)>> {
        let at: usize = self.offset();
        let properties: Vec<(Expression, bool)> = self.spelled_property_list()?;
        if properties
            .iter()
            .any(|(property, _)| !matches!(property, Expression::Eq(_)))
        {
            return Err(self.error_with(
                "Operation not allowed: Values must be specified for key(s)",
                at,
                true,
            ));
        }
        Ok(properties
            .into_iter()
            .map(|(property, without_equals)| (property_pair(property), without_equals))
            .collect())
    }

    /// `propertyList` of keys only, refused as Spark refuses one with a value.
    pub(crate) fn key_property_list(&mut self) -> Parsing<Vec<Expression>> {
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
        Ok(properties)
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
        let deep: bool = too_deep_to_keep(&partition(spec.clone()));
        if self.eat_kw("RENAME") {
            self.expect_kw("TO")?;
            let to: Expression = partition(self.partition_spec()?);
            if deep {
                self.gap("rename_partition", start);
                return Ok(node);
            }
            node.partition = Some(partition_pairs(spec));
            node.actions.extend(self.keyed_action(
                ("rename_partition", start),
                RENAME_PARTITION,
                vec![to],
            )?);
        } else if self.kw("SET") && self.kw_at(1, "LOCATION") {
            self.at += 2;
            let location: String = self.string_lit()?;
            node.partition = Some(partition_pairs(spec));
            node.actions
                .push(AlterTableAction::SetLocation { location });
        } else if self.kw("SET") && self.kw_in(1, &["SERDE", "SERDEPROPERTIES"]) {
            let (key, values) = self.serde_clause()?;
            if deep {
                self.gap("serde", start);
                return Ok(node);
            }
            node.partition = Some(partition_pairs(spec));
            node.actions
                .extend(self.keyed_action(("serde", start), key, values)?);
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
    fn serde_clause(&mut self) -> Parsing<(&'static str, Vec<Expression>)> {
        self.expect_kw("SET")?;
        if self.eat_kw("SERDE") {
            let mut values: Vec<Expression> = vec![self.string_literal()?];
            if self.eat_kws(&["WITH", "SERDEPROPERTIES"]) {
                values.push(tuple(self.property_list()?));
            }
            return Ok((SET_SERDE, values));
        }
        self.expect_kw("SERDEPROPERTIES")?;
        Ok((SET_SERDEPROPERTIES, vec![tuple(self.property_list()?)]))
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
                let owner: Expression = Expression::Identifier(self.principal()?);
                self.keyed_action(("owner", start), OWNER_TO, vec![owner])
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
                let collation: Expression = Expression::Identifier(self.ident()?);
                self.keyed_action(
                    ("table_collation", start),
                    DEFAULT_COLLATION,
                    vec![collation],
                )
            }
            "RECOVER" => {
                self.at += 1;
                self.expect_kw("PARTITIONS")?;
                self.keyed_action(
                    ("recover_partitions", start),
                    RECOVER_PARTITIONS,
                    Vec::new(),
                )
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
                let mut values: Vec<Expression> = Vec::new();
                if self.kw_in(0, &["AUTO", "NONE"]) {
                    values.push(var(&self.word_at(0).unwrap_or_default()));
                    self.at += 1;
                } else if self.is_char('(') {
                    values.push(tuple(self.parenthesized(|parser| parser.expression())?));
                }
                self.keyed_action(
                    ("replace_partitioning", start),
                    REPLACE_PARTITIONING,
                    values,
                )
            }
            "ENABLE" | "DISABLE" | "INHERIT" => {
                self.at += 1;
                self.expect_kw("PREDICTIVE")?;
                self.expect_kw("OPTIMIZATION")?;
                self.keyed_action(
                    ("predictive_optimization", start),
                    PREDICTIVE_OPTIMIZATION,
                    vec![var(&word)],
                )
            }
            _ => Err(self.error()),
        }
    }

    /// `ADD COLUMN[S] ...`, `ADD [IF NOT EXISTS] PARTITION ...` and `ADD` of a table constraint.
    fn alter_table_add(&mut self) -> Parsing<Vec<AlterTableAction>> {
        self.expect_kw("ADD")?;
        let plural: bool = self.kw("COLUMNS");
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
                    column_keyword: false,
                    keyword: plural.then(|| "COLUMNS".to_owned()),
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
        let constraint = self
            .constraint_validation(constraint, &["ENABLE", "DISABLE", "VALIDATE", "NOVALIDATE"]);
        Ok(vec![AlterTableAction::AddConstraint(constraint)])
    }

    /// `qualifiedColTypeWithPosition`, with Databricks' `COLLATE` and `MASK` descriptors.
    fn added_column(&mut self) -> Parsing<(ColumnDef, Option<ColumnPosition>)> {
        let name: Identifier = joined(self.multipart_identifier()?);
        let (data_type, collation) = self.collated_type()?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column.name = name;
        column = collate(column, collation);
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
                let mask: Expression = self.column_mask()?;
                match too_deep_to_keep(&mask) {
                    true => self.gap("column_mask", start),
                    false => {
                        column.options.push(mask);
                        self.treed("column_mask", start);
                    }
                }
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
    pub(crate) fn column_mask(&mut self) -> Parsing<Expression> {
        let function: Expression = Expression::Table(Box::new(table(self.multipart_identifier()?)));
        let mut expressions: Vec<Expression> = Vec::new();
        if self.eat_kws(&["USING", "COLUMNS"]) {
            expressions.push(tuple(self.parenthesized(|parser| parser.expression())?));
        }
        Ok(Expression::MaskingPolicyColumnConstraint(Box::new(
            MaskingPolicyColumnConstraint {
                this: Box::new(function),
                expressions,
                with: false,
            },
        )))
    }

    /// `ON ( [column, ...] )` after a row filter's function: the columns passed to it.
    pub(crate) fn row_filter_columns(&mut self) -> Parsing<Vec<Expression>> {
        self.expect_kw("ON")?;
        self.expect_char('(')?;
        let columns: Vec<Expression> = match self.is_char(')') {
            true => Vec::new(),
            false => self.comma_list(|parser| parser.expression())?,
        };
        self.expect_char(')')?;
        Ok(columns)
    }

    /// A column's position, mask or identity sync after `values`, the lead words and the column.
    fn keyed_column_action(
        &mut self,
        start: usize,
        mut values: Vec<Expression>,
    ) -> Parsing<Option<AlterTableAction>> {
        if self.kw_in(0, &["FIRST", "AFTER"]) {
            values.push(match self.column_position()? {
                ColumnPosition::First => var("FIRST"),
                ColumnPosition::After(column) => {
                    tuple(vec![var("AFTER"), Expression::Identifier(column)])
                }
            });
            return self.keyed_action(("alter_column_position", start), COLUMN_POSITION, values);
        }
        if self.kw("SET") && self.kw_at(1, "MASK") {
            self.at += 2;
            values.push(self.column_mask()?);
            return self.keyed_action(("column_mask", start), COLUMN_SET_MASK, values);
        }
        if self.kw("DROP") && self.kw_at(1, "MASK") {
            self.at += 2;
            return self.keyed_action(("column_mask", start), COLUMN_DROP_MASK, values);
        }
        if self.eat_kw("SYNC") {
            self.expect_kw("IDENTITY")?;
            return self.keyed_action(("identity_sync", start), COLUMN_SYNC_IDENTITY, values);
        }
        Ok(None)
    }

    /// `DROP` of columns, partitions, constraints, keys, a feature or a row filter.
    fn alter_table_drop(&mut self, start: usize) -> Parsing<Vec<AlterTableAction>> {
        self.expect_kw("DROP")?;
        let plural: bool = self.kw("COLUMNS");
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
                    column_keyword: false,
                    keyword: plural.then(|| "COLUMNS".to_owned()),
                })
                .collect();
            return Ok(dropped);
        }
        if self.eat_kw("CONSTRAINT") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: Identifier = self.ident()?;
            let cascade: bool = self.eat_kw("CASCADE");
            let restrict: bool = !cascade && self.eat_kw("RESTRICT");
            return Ok(vec![AlterTableAction::DropConstraint {
                name,
                if_exists,
                restrict,
                cascade,
            }]);
        }
        if self.kw("PRIMARY") {
            self.at += 1;
            self.expect_kw("KEY")?;
            let mut values: Vec<Expression> = Vec::new();
            if self.eat_kws(&["IF", "EXISTS"]) {
                values.push(var("IF EXISTS"));
            }
            if self.eat_kw("RESTRICT") {
                values.push(var("RESTRICT"));
            } else if self.eat_kw("CASCADE") {
                values.push(var("CASCADE"));
            }
            return self
                .keyed_action(("drop_primary_key", start), DROP_PRIMARY_KEY, values)
                .map(Vec::from_iter);
        }
        if self.kw("FOREIGN") {
            self.at += 1;
            self.expect_kw("KEY")?;
            let mut values: Vec<Expression> = Vec::new();
            if self.eat_kws(&["IF", "EXISTS"]) {
                values.push(var("IF EXISTS"));
            }
            let columns: Vec<Expression> = self
                .identifier_list()?
                .into_iter()
                .map(Expression::Identifier)
                .collect();
            values.push(tuple(columns));
            return self
                .keyed_action(("drop_foreign_key", start), DROP_FOREIGN_KEY, values)
                .map(Vec::from_iter);
        }
        if self.eat_kw("FEATURE") {
            let mut values: Vec<Expression> = vec![match self.string_at(0) {
                true => self.string_literal()?,
                false => Expression::Identifier(self.ident()?),
            }];
            if self.eat_kw("TRUNCATE") {
                self.expect_kw("HISTORY")?;
                values.push(var("TRUNCATE HISTORY"));
            }
            return self
                .keyed_action(("drop_feature", start), DROP_FEATURE, values)
                .map(Vec::from_iter);
        }
        if self.eat_kw("ROW") {
            self.expect_kw("FILTER")?;
            return self
                .keyed_action(("row_filter", start), DROP_ROW_FILTER, Vec::new())
                .map(Vec::from_iter);
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
        Ok(vec![AlterTableAction::DropPartition {
            partitions,
            if_exists,
            purge: self.eat_kw("PURGE"),
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
            let column: Expression = column_from(self.multipart_identifier()?);
            let key: &str = match self.kw("SET") {
                true => COLUMN_SET_TAGS,
                false => COLUMN_UNSET_TAGS,
            };
            self.at += 1;
            self.expect_kw("TAGS")?;
            let tags: Expression = tuple(self.property_list()?);
            return self
                .keyed_action(("column_tags", start), key, vec![column, tags])
                .map(Vec::from_iter);
        }
        let mut altered: Vec<AlterTableAction> = Vec::new();
        loop {
            let spec_start: usize = self.offset();
            let parts: Vec<Identifier> = self.multipart_identifier()?;
            if parts.len() > 1 && self.kw("SET") && self.kw_at(1, "MASK") {
                return Err(self.error_with(
                    "[COLUMN_MASKS_MULTI_PART_TARGET_COLUMN_NAME] A column mask applies to a \
                     top-level column, not a nested field",
                    spec_start,
                    true,
                ));
            }
            let lead: Expression = var(match (change, column_keyword) {
                (true, true) => "CHANGE COLUMN",
                (true, false) => "CHANGE",
                (false, true) => "ALTER COLUMN",
                (false, false) => "ALTER",
            });
            let keyed: Option<AlterTableAction> =
                self.keyed_column_action(spec_start, vec![lead, column_from(parts.clone())])?;
            let name: Identifier = joined(parts);
            if let Some(keyed) = keyed {
                altered.push(keyed);
            } else {
                match self.column_action()? {
                    Some(action) => altered.push(AlterTableAction::AlterColumn {
                        name,
                        action,
                        use_modify_keyword: false,
                        column_keyword,
                        action_words: None,
                        keyword: change.then(|| "CHANGE".to_owned()),
                    }),
                    None if self.opaque => {}
                    None if self.read_gap() => self.gap("alter_column_without_action", spec_start),
                    None => {
                        return Err(self.error_with(
                            "Operation not allowed: ALTER TABLE table ALTER COLUMN requires a TYPE, \
                             a SET/DROP, a COMMENT, or a FIRST/AFTER",
                            spec_start,
                            true,
                        ));
                    }
                }
            }
            if !self.eat_char(',') {
                break;
            }
        }
        Ok(altered)
    }

    /// Whether `name SET|UNSET TAGS` follows, which only `ALTER COLUMN` takes, on a top-level column.
    fn tag_column_ahead(&self) -> bool {
        self.kw_in(1, &["SET", "UNSET"]) && self.kw_at(2, "TAGS")
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
    fn column_action(&mut self) -> Parsing<Option<AlterColumnAction>> {
        if self.eat_kw("TYPE") {
            let data_type: DataType = self.data_type()?;
            return Ok(Some(AlterColumnAction::SetDataType {
                data_type,
                using: None,
                collate: None,
                rename: None,
            }));
        }
        if self.eat_kw("COMMENT") {
            return Ok(Some(AlterColumnAction::Comment(self.string_lit()?)));
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
        Ok(None)
    }

    /// `SET` of table properties, a location, a SerDe, tags, an owner, a row filter or management.
    fn alter_table_set(&mut self, start: usize) -> Parsing<Option<AlterTableAction>> {
        if self.kw_at(1, "SERDE") || self.kw_at(1, "SERDEPROPERTIES") {
            let (key, values) = self.serde_clause()?;
            return self.keyed_action(("serde", start), key, values);
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
                let owner: Expression = Expression::Identifier(self.principal()?);
                self.keyed_action(("owner", start), SET_OWNER_TO, vec![owner])
            }
            "ROW" => {
                self.expect_kw("FILTER")?;
                let filter: Expression =
                    Expression::Table(Box::new(table(self.multipart_identifier()?)));
                let columns: Vec<Expression> = self.row_filter_columns()?;
                self.keyed_action(
                    ("row_filter", start),
                    SET_ROW_FILTER,
                    vec![filter, tuple(columns)],
                )
            }
            "MANAGED" => {
                let mode: Option<&str> = match self.word_at(0).as_deref() {
                    Some("COPY") => Some("COPY"),
                    Some("MOVE") => Some("MOVE"),
                    Some("TRUNCATE") => Some(TRUNCATE_UNIFORM_HISTORY),
                    _ => None,
                };
                if mode.is_some() && !self.eat_kw("COPY") && !self.eat_kw("MOVE") {
                    self.at += 1;
                    self.expect_kw("UNIFORM")?;
                    self.expect_kw("HISTORY")?;
                }
                let values: Vec<Expression> = mode.map(var).into_iter().collect();
                self.keyed_action(("managed", start), SET_MANAGED, values)
            }
            "EXTERNAL" => {
                let dry_run: bool = self.eat_kw("DRY");
                if dry_run {
                    self.expect_kw("RUN")?;
                }
                let values: Vec<Expression> = dry_run.then(|| var("DRY RUN")).into_iter().collect();
                self.keyed_action(("managed", start), SET_EXTERNAL, values)
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
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            return Ok(Some(AlterTableAction::UnsetTableProperties {
                properties: self.key_property_list()?,
                if_exists,
            }));
        }
        if self.eat_kw("TAGS") {
            let names: Vec<String> = self.property_list()?.iter().map(property_text).collect();
            return Ok(Some(AlterTableAction::UnsetTag { names }));
        }
        self.expect_kw("MANAGED")?;
        let truncate: bool = self.eat_kw("TRUNCATE");
        if truncate {
            self.expect_kw("UNIFORM")?;
            self.expect_kw("HISTORY")?;
        }
        let values: Vec<Expression> = truncate
            .then(|| var(TRUNCATE_UNIFORM_HISTORY))
            .into_iter()
            .collect();
        self.keyed_action(("managed", start), UNSET_MANAGED, values)
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
                Some(AlterViewAction::SetTableProperties(
                    self.valued_property_list()?,
                ))
            }
            Some("UNSET") if self.kw_at(1, "TBLPROPERTIES") => {
                self.at += 2;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                Some(AlterViewAction::UnsetTableProperties {
                    properties: self.key_property_list()?,
                    if_exists,
                })
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
            Some(word @ ("SET" | "UNSET")) if self.kw_at(1, "TAGS") => {
                let action: String = format!("{word} TAGS");
                self.at += 2;
                let tags: Vec<Expression> = self.tags(word == SET_TAGS)?;
                return Ok(view_object(node.name, action, false, tags));
            }
            Some("SET") if self.kw_at(1, "MANAGED") => {
                self.at += 2;
                return Ok(view_object(
                    node.name,
                    SET_MANAGED.to_owned(),
                    false,
                    Vec::new(),
                ));
            }
            Some("WITH") if self.kw_at(1, "SCHEMA") => {
                self.at += 2;
                let binding: &str = match self.word_at(0).as_deref() {
                    Some("BINDING") => "BINDING",
                    Some("COMPENSATION") => "COMPENSATION",
                    Some("EVOLUTION") => "EVOLUTION",
                    Some("TYPE") if self.kw_at(1, "EVOLUTION") => "TYPE EVOLUTION",
                    _ => return Err(self.error()),
                };
                self.at += binding.split(' ').count();
                let operands: Vec<Expression> = vec![var(binding)];
                return Ok(view_object(
                    node.name,
                    "WITH SCHEMA".to_owned(),
                    false,
                    operands,
                ));
            }
            Some("ADD") if self.kw_at(1, "CONSTRAINT") || self.kw_in(1, &["MANY", "ONE"]) => {
                self.at += 1;
                let mut operands: Vec<Expression> = Vec::new();
                let action: &str = match self.eat_kw("CONSTRAINT") {
                    true => {
                        operands.push(Expression::Identifier(self.ident()?));
                        "ADD CONSTRAINT"
                    }
                    false => "ADD",
                };
                operands.extend(self.relationship_constraint()?);
                return Ok(view_object(node.name, action.to_owned(), false, operands));
            }
            Some("DROP") if self.kw_at(1, "CONSTRAINT") => {
                self.at += 2;
                let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
                let operands: Vec<Expression> = vec![Expression::Identifier(self.ident()?)];
                return Ok(view_object(
                    node.name,
                    "DROP CONSTRAINT".to_owned(),
                    if_exists,
                    operands,
                ));
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

    /// `( 'key' = value, ... )` after `SET TAGS`, or `( 'key', ... )` after `UNSET TAGS`.
    fn tags(&mut self, set: bool) -> Parsing<Vec<Expression>> {
        self.parenthesized(|parser| {
            let key: Expression = parser.string_literal()?;
            match set {
                true => {
                    parser.expect_char('=')?;
                    let value: Expression = parser.primary_expression()?;
                    Ok(binary(Expression::Eq, key, value))
                }
                false => Ok(key),
            }
        })
    }

    /// A view's `relationship_constraint`: cardinality, target view with alias, then `ON` or `USING`.
    fn relationship_constraint(&mut self) -> Parsing<Vec<Expression>> {
        let mut cardinality: Vec<String> = Vec::new();
        for expected in [Some(["MANY", "ONE"]), None, Some(["MANY", "ONE"])] {
            match expected {
                Some(words) if self.kw_in(0, &words) => {
                    cardinality.push(self.word_at(0).unwrap_or_default());
                    self.at += 1;
                }
                Some(_) => return Err(self.error()),
                None => {
                    self.expect_kw("TO")?;
                    cardinality.push("TO".to_owned());
                }
            }
        }
        self.expect_kw("RELATIONSHIP")?;
        self.expect_kw("TO")?;
        let mut target: TableRef = self.table_reference()?;
        if let Some(alias) = self.table_alias()? {
            target.alias = Some(alias.name);
            target.alias_explicit_as = alias.explicit;
            target.column_aliases = alias.columns;
        }
        let join: Expression = match self.eat_kw("ON") {
            true => property("ON", Some(self.expression()?)),
            false => {
                self.expect_kw("USING")?;
                let columns: Vec<Expression> = self
                    .identifier_list()?
                    .into_iter()
                    .map(Expression::Identifier)
                    .collect();
                property("USING", Some(tuple(columns)))
            }
        };
        Ok(vec![
            var(&cardinality.join(" ")),
            property("RELATIONSHIP TO", Some(Expression::Table(Box::new(target)))),
            join,
        ])
    }

    /// `ALTER SCHEMA` as an `AlterObject` whose action names the clause and holds its operands.
    fn alter_schema(&mut self) -> Parsing<Expression> {
        let kind: String = self.word_at(0).unwrap_or_default();
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
                let tags: Vec<Expression> = self.tags(word == SET_TAGS)?;
                (format!("{word} TAGS"), tags)
            }
            Some("RETAIN" | "SET") if self.kw_at(0, "RETAIN") || self.kw_at(1, "RETAIN") => {
                let set: bool = self.eat_kw("SET");
                self.expect_kw("RETAIN")?;
                self.expect_kw("DROPPED")?;
                self.expect_kw("TO")?;
                (
                    format!("{}RETAIN DROPPED TO", if set { "SET " } else { "" }),
                    vec![self.retention_period()?],
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
                let set: bool = self.eat_kw("SET");
                self.expect_kw("OWNER")?;
                self.expect_kw("TO")?;
                (
                    format!("{}OWNER TO", if set { "SET " } else { "" }),
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
            kind,
            name: Some(name),
            arguments: Vec::new(),
            if_exists: false,
            action,
            operand_if_exists: false,
            operand_if_not_exists: false,
            operands,
            properties: Vec::new(),
            ..Default::default()
        })))
    }
}
