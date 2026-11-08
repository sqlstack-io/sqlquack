//! `AlterStatement` (alter.gram), with the transformer's refusals.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::{
    AlterColumnAction, AlterTable, AlterTableAction, ColumnConstraint, ColumnDef,
};
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::schema_objects::models::{
    AlterSequence, AlterView, AlterViewAction,
};

use crate::parse::_helpers::commands::creates::is_varchar;
use crate::parse::_helpers::commands::objects::SequenceSetting;
use crate::parse::_helpers::commands::options::{RelOption, RelValue};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::{null, table};
use crate::parse::constants::{
    ADD_CHECK, ADD_FOREIGN_KEY, ADD_GENERATED, ADD_NESTED_COMPRESSION, ADD_NESTED_NOT_NULL,
    ADD_NESTED_UNIQUE, ADD_NO_TYPE, ADD_PRIMARY_KEY, ALTER_DROP_CONSTRAINT, ALTER_ONE,
    ALTER_SCHEMA, ALTER_SEQUENCE_OPTION, ALTER_SEQUENCE_OWNED_TWICE, ALTER_SEQUENCE_RENAME,
    ALTER_TYPE_USING, AUTO_COMPRESSION, COLLATION_TYPE, COLUMN_CONSTRAINT_STARTS,
    TABLE_CONSTRAINT_STARTS, VARCHAR_COLLATION,
};

/// The refusal of short `auto` compression in `ADD COLUMN`.
const ADD_COMPRESSION_AUTO: &str = "Unrecognized option for column compression";

/// One `AlterTableOptions` entry: an action the tree holds, or a construct kept as a gap.
enum TableAction {
    Tree(Box<AlterTableAction>),
    Gap(&'static str),
}

impl Parser<'_> {
    /// `AlterStatement`: tables, views and sequences as trees, databases and schema options as gaps.
    pub(crate) fn alter_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("ALTER")?;
        if self.eat_kw("TABLE") {
            return self.alter_table(start);
        }
        if self.eat_kw("VIEW") {
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let name: TableRef = self.base_table_name()?;
            let renamed: Identifier = self.rename_to()?;
            if if_exists {
                self.gap("alter_view_if_exists", start);
            }
            let mut node: AlterView = AlterView::new(String::new());
            node.name = name;
            node.actions = vec![AlterViewAction::Rename(Box::new(table(vec![renamed])))];
            return Ok(Expression::AlterView(Box::new(node)));
        }
        if self.eat_kw("SEQUENCE") {
            return self.alter_sequence(start);
        }
        if self.eat_kw("DATABASE") {
            self.eat_kws(&["IF", "EXISTS"]);
            self.name(is_identifier)?;
            for word in ["SET", "ALIAS", "TO"] {
                self.expect_kw(word)?;
            }
            self.name(is_identifier)?;
            self.gap("alter_database", start);
            return Ok(null());
        }
        self.expect_kw("SCHEMA")?;
        self.eat_kws(&["IF", "EXISTS"]);
        self.qualified_name()?;
        if self.kw("RENAME") {
            self.rename_to()?;
            return Err(self.error_with(ALTER_SCHEMA, start, false));
        }
        let reset: bool = self.eat_kw("RESET");
        if !reset {
            self.expect_kw("SET")?;
        }
        self.option_change(reset, start)?;
        self.gap("schema_options", start);
        Ok(null())
    }

    /// `AlterTableStmt` after `TABLE`: `[IF EXISTS] name action, ...`, of which one is allowed.
    fn alter_table(&mut self, start: usize) -> Parsing<Expression> {
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let name: TableRef = self.base_table_name()?;
        let mut actions: Vec<TableAction> = vec![self.alter_table_action(start)?];
        while self.eat_char(',') {
            if self.terminal_at(0) {
                break;
            }
            actions.push(self.alter_table_action(start)?);
        }
        if actions.len() > 1 {
            self.refuse_later(ALTER_ONE, start);
        }
        let mut node: AlterTable = AlterTable::new(String::new());
        node.name = name;
        node.if_exists = if_exists;
        for action in actions {
            match action {
                TableAction::Tree(action) => node.actions.push(*action),
                TableAction::Gap(construct) => self.gap(construct, start),
            }
        }
        Ok(Expression::AlterTable(Box::new(node)))
    }

    /// `AlterTableOptions`, in the grammar's order of choice.
    fn alter_table_action(&mut self, start: usize) -> Parsing<TableAction> {
        if self.kw("ADD") {
            self.at += 1;
            if self.kw_in(0, &TABLE_CONSTRAINT_STARTS) {
                let constraint = self.table_constraint()?;
                return Ok(TableAction::Tree(Box::new(
                    AlterTableAction::AddConstraint(constraint),
                )));
            }
            return self.add_column(start);
        }
        if self.kw("DROP") && self.kw_at(1, "CONSTRAINT") {
            self.at += 2;
            self.eat_kws(&["IF", "EXISTS"]);
            self.col_id_or_string()?;
            self.drop_cascade();
            return Err(self.error_with(ALTER_DROP_CONSTRAINT, start, false));
        }
        if self.eat_kw("DROP") {
            self.eat_kw("COLUMN");
            let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
            let mut path: Vec<Identifier> = self.nested_column_name()?;
            let cascade: bool = self.drop_cascade();
            let action: AlterTableAction = match path.len() {
                1 => AlterTableAction::DropColumn {
                    name: path.remove(0),
                    if_exists,
                    cascade,
                },
                _ => AlterTableAction::DropField {
                    names: path,
                    if_exists,
                    cascade,
                },
            };
            return Ok(TableAction::Tree(Box::new(action)));
        }
        if self.eat_kw("ALTER") {
            self.eat_kw("COLUMN");
            let path: Vec<Identifier> = self.nested_column_name()?;
            let action: AlterColumnAction = self.alter_column_entry(start)?;
            return Ok(match nested(path) {
                Ok(name) => TableAction::Tree(Box::new(AlterTableAction::AlterColumn {
                    name,
                    action,
                    use_modify_keyword: false,
                })),
                Err(()) => TableAction::Gap("nested_field"),
            });
        }
        if self.kw("RENAME") && !self.kw_at(1, "TO") {
            self.at += 1;
            self.eat_kw("COLUMN");
            let mut path: Vec<Identifier> = self.nested_column_name()?;
            self.expect_kw("TO")?;
            let new_name: Identifier = self.col_id_or_string()?;
            let action: AlterTableAction = match path.len() {
                1 => AlterTableAction::RenameColumn {
                    old_name: path.remove(0),
                    new_name,
                    if_exists: false,
                },
                _ => AlterTableAction::RenameField {
                    names: path,
                    new_name,
                },
            };
            return Ok(TableAction::Tree(Box::new(action)));
        }
        if self.kw("RENAME") {
            let renamed: Identifier = self.rename_to()?;
            return Ok(TableAction::Tree(Box::new(AlterTableAction::RenameTable(
                table(vec![renamed]),
            ))));
        }
        let reset: bool = self.eat_kw("RESET");
        if !reset {
            self.expect_kw("SET")?;
        }
        if self.kw_in(0, &["PARTITIONED", "SORTED"]) {
            let sorted: bool = self.kw("SORTED");
            self.at += 1;
            self.expect_kw("BY")?;
            if !reset {
                self.expect_char('(')?;
                match sorted {
                    true => {
                        self.comma_list(|parser| parser.sort_item())?;
                    }
                    false => {
                        self.expression_list(')')?;
                    }
                }
                self.expect_char(')')?;
            }
            return Ok(TableAction::Gap("table_layout"));
        }
        let options: Vec<RelOption> = self.option_change(reset, start)?;
        let action: AlterTableAction = match reset {
            true => AlterTableAction::UnsetProperty {
                properties: options.into_iter().map(|option| option.name).collect(),
            },
            false => AlterTableAction::SetProperty {
                properties: options
                    .into_iter()
                    .map(|option| (option.name, option.expression))
                    .collect(),
            },
        };
        Ok(TableAction::Tree(Box::new(action)))
    }

    /// `SetOptions` or `ResetOptions` after the word: a reset option may hold no value but `NULL`.
    fn option_change(&mut self, reset: bool, start: usize) -> Parsing<Vec<RelOption>> {
        let options: Vec<RelOption> = self.rel_options()?;
        let valued = options
            .iter()
            .find(|option| !matches!(option.value, RelValue::Absent | RelValue::Null));
        if let (true, Some(RelOption { name, .. })) = (reset, valued) {
            let message: String =
                format!("Reset option \"{name}\" cannot set any value. Did you mean to use SET?");
            self.refuse_later(&message, start);
        }
        Ok(options)
    }

    /// `AddColumn` after `ADD`: `[COLUMN] [IF NOT EXISTS] name[.field...] [type] constraint...`.
    fn add_column(&mut self, start: usize) -> Parsing<TableAction> {
        self.eat_kw("COLUMN");
        let if_not_exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        let mut path: Vec<Identifier> = vec![self.name(is_identifier)?];
        while self.eat_char('.') {
            path.push(self.name(is_col_label)?);
        }
        let typed: bool = !(self.terminal_at(0)
            || self.kw_in(0, &COLUMN_CONSTRAINT_STARTS)
            || self.kw_in(0, &["GENERATED", "AS"]));
        let data_type: DataType = match typed {
            true => self.data_type()?,
            false => DataType::Unknown,
        };
        let generated: Option<ColumnConstraint> = self.generated_column("", start)?;
        let mut column: ColumnDef = ColumnDef::new(String::new(), data_type);
        column = self.column_constraints(column)?;
        let has = |test: fn(&ColumnConstraint) -> bool| column.constraints.iter().any(test);
        let not_null: bool = column.nullable == Some(false);
        let collated: bool = has(|constraint| matches!(constraint, ColumnConstraint::Collate(_)));
        let compressed: bool = column.encoding.is_some();
        let refusal: Option<&str> = if !typed && generated.is_none() {
            Some(ADD_NO_TYPE)
        } else if generated.is_some() {
            Some(ADD_GENERATED)
        } else if column.primary_key {
            Some(ADD_PRIMARY_KEY)
        } else if has(|constraint| matches!(constraint, ColumnConstraint::Check(_))) {
            Some(ADD_CHECK)
        } else if has(|constraint| matches!(constraint, ColumnConstraint::References(_))) {
            Some(ADD_FOREIGN_KEY)
        } else if column.encoding.as_deref() == Some(AUTO_COMPRESSION) {
            Some(ADD_COMPRESSION_AUTO)
        } else if collated && !is_varchar(&column.data_type) {
            Some(VARCHAR_COLLATION)
        } else if path.len() > 1 && not_null {
            Some(ADD_NESTED_NOT_NULL)
        } else if path.len() > 1 && column.unique {
            Some(ADD_NESTED_UNIQUE)
        } else if path.len() > 1 && compressed {
            Some(ADD_NESTED_COMPRESSION)
        } else {
            None
        };
        if let Some(message) = refusal {
            self.refuse_later(message, start);
        }
        let Ok(name) = nested(path) else {
            return Ok(TableAction::Gap("nested_field"));
        };
        column.name = name;
        Ok(TableAction::Tree(Box::new(AlterTableAction::AddColumn {
            column,
            if_not_exists,
            position: None,
        })))
    }

    /// `AlterColumnEntry`: a default, a nullability, or a new type.
    fn alter_column_entry(&mut self, start: usize) -> Parsing<AlterColumnAction> {
        if self.eat_kws(&["SET", "DEFAULT"]) {
            return Ok(AlterColumnAction::SetDefault(self.expression()?));
        }
        if self.eat_kws(&["DROP", "DEFAULT"]) {
            return Ok(AlterColumnAction::DropDefault);
        }
        if self.eat_kws(&["DROP", "NOT", "NULL"]) {
            return Ok(AlterColumnAction::DropNotNull);
        }
        if self.eat_kws(&["SET", "NOT", "NULL"]) {
            return Ok(AlterColumnAction::SetNotNull);
        }
        if self.eat_kw("SET") {
            self.eat_kw("DATA");
        }
        self.expect_kw("TYPE")?;
        let typed: bool = !(self.terminal_at(0) || self.kw_in(0, &["COLLATE", "USING"]));
        let data_type: Option<DataType> = match typed {
            true => Some(self.data_type()?),
            false => None,
        };
        let collate: Option<String> = match self.eat_kw("COLLATE") {
            true => {
                let mut parts: Vec<String> = vec![self.name(is_identifier)?.name];
                while self.eat_char('.') {
                    parts.push(self.name(is_col_label)?.name);
                }
                Some(parts.join("."))
            }
            false => None,
        };
        let using: Option<Expression> = match self.eat_kw("USING") {
            true => Some(self.expression()?),
            false => None,
        };
        let refusal: Option<&str> = match (&data_type, &collate, &using) {
            (None, _, None) => Some(ALTER_TYPE_USING),
            (None, Some(_), _) => Some(COLLATION_TYPE),
            (Some(data_type), Some(_), _) if !is_varchar(data_type) => Some(VARCHAR_COLLATION),
            _ => None,
        };
        if let Some(message) = refusal {
            self.refuse_later(message, start);
        }
        Ok(AlterColumnAction::SetDataType {
            data_type: data_type.unwrap_or(DataType::Unknown),
            using,
            collate,
        })
    }

    /// `AlterSequenceStmt` after `SEQUENCE`: only `OWNED BY` takes effect; a rename is refused.
    fn alter_sequence(&mut self, start: usize) -> Parsing<Expression> {
        let if_exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let name: Vec<Identifier> = self.dotted_name(is_identifier, Some(3))?;
        if self.kw("RENAME") {
            self.rename_to()?;
            return Err(self.error_with(ALTER_SEQUENCE_RENAME, start, false));
        }
        let mut owners: Vec<Vec<Identifier>> = Vec::new();
        loop {
            if self.kw("OWNED") && self.kw_at(1, "BY") {
                self.at += 2;
                owners.push(self.qualified_name()?);
                continue;
            }
            let setting: Option<SequenceSetting> = self.sequence_option()?;
            if setting.is_none() {
                break;
            }
        }
        if owners.len() > 1 {
            self.refuse_later(ALTER_SEQUENCE_OWNED_TWICE, start);
        }
        let Some(owner) = owners.pop() else {
            return Err(self.error_with(ALTER_SEQUENCE_OPTION, start, false));
        };
        let mut node: AlterSequence = AlterSequence::new(String::new());
        node.name = table(name);
        node.if_exists = if_exists;
        node.owned_by = Some(Some(table(owner)));
        Ok(Expression::AlterSequence(Box::new(node)))
    }

    /// `RenameAlter`: `RENAME TO name`, where the name may be a string.
    fn rename_to(&mut self) -> Parsing<Identifier> {
        self.expect_kw("RENAME")?;
        self.expect_kw("TO")?;
        self.col_id_or_string()
    }

    /// `NestedColumnName`: `[name.]...name`, every part an `Identifier`.
    fn nested_column_name(&mut self) -> Parsing<Vec<Identifier>> {
        let mut path: Vec<Identifier> = vec![self.name(is_identifier)?];
        while self.eat_char('.') {
            path.push(self.name(is_identifier)?);
        }
        Ok(path)
    }

    /// `DropBehavior`, as whether it cascades.
    fn drop_cascade(&mut self) -> bool {
        let cascade: bool = self.eat_kw("CASCADE");
        if !cascade {
            self.eat_kw("RESTRICT");
        }
        cascade
    }
}

/// The one name of an unnested column path, or `Err` for a struct field the tree cannot name.
fn nested(mut path: Vec<Identifier>) -> Result<Identifier, ()> {
    match path.len() {
        1 => path.pop().ok_or(()),
        _ => Err(()),
    }
}
