//! CREATE TABLE and INSERT INTO it VALUES in a case's setup, as DuckDB statements.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_duckdb::blocks::main::column_type::column_type;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_semantics::types::models::ColumnSpec;
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_snowflake::typing::models::SnowflakeType;
use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::statements::models::Insert;
use sqlquack_syntax::expressions::tables::models::CreateTable;
use sqlquack_syntax::expressions::tree::models::Expression;

use crate::conversion::_helpers::expressions::casts::data_type_name;
use crate::conversion::_helpers::support::tree::with_user_types;
use crate::conversion::models::ConvertedSetup;

/// `CREATE TABLE name (column type, …)` and nothing else: its DuckDB statement, name and columns.
pub(crate) fn created(
    table: &CreateTable,
    converted: &ConvertedSetup,
    log: &ProvenanceLog,
) -> Result<(String, String, Vec<ColumnSpec>), String> {
    let bare = CreateTable {
        name: TableRef {
            name: table.name.name.clone(),
            ..TableRef::new("")
        },
        columns: table
            .columns
            .iter()
            .map(|column| ColumnDef {
                name: column.name.clone(),
                ..ColumnDef::new("", column.data_type.clone())
            })
            .collect(),
        ..CreateTable::new("")
    };
    if *table != bare || table.columns.is_empty() {
        return Err("has a clause other than unqualified, typed columns".to_owned());
    }
    let name = resolved(&table.name.name)?;
    if converted.relations.contains_key(&name) {
        return Err(format!("creates {name} again"));
    }
    let mut columns: Vec<ColumnSpec> = Vec::with_capacity(table.columns.len());
    let mut written: Vec<String> = Vec::with_capacity(table.columns.len());
    for column in &table.columns {
        let column_name = resolved(&column.name)?;
        if columns.iter().any(|earlier| earlier.name == column_name) {
            return Err(format!("declares {column_name} twice"));
        }
        let spelled = serde_json::to_value(&column.data_type).map_err(|error| {
            format!("types {column_name} with a type that does not serialise: {error}")
        })?;
        let spelled = with_user_types(&spelled, &converted.types);
        let type_name = data_type_name(&spelled)
            .ok_or_else(|| format!("types {column_name} with a type that has no name"))?;
        let data_type = cast_target(&type_name, log)?.data_type;
        if !matches!(
            data_type,
            SnowflakeType::Number { .. } | SnowflakeType::Varchar
        ) {
            return Err(format!(
                "types {column_name} {type_name}, which setup does not replay"
            ));
        }
        written.push(format!(
            "{} {}",
            quoted_identifier(&column_name, log),
            column_type(data_type, log)?
        ));
        columns.push(ColumnSpec::new(column_name, data_type));
    }
    let sql = format!(
        "CREATE TABLE {} ({})",
        quoted_identifier(&name, log),
        written.join(", ")
    );
    Ok((sql, name, columns))
}

/// `INSERT INTO name VALUES (…), …` into a relation setup created, of literals its columns hold exactly.
pub(crate) fn inserted(
    insert: &Insert,
    converted: &ConvertedSetup,
    log: &ProvenanceLog,
) -> Result<String, String> {
    let Insert {
        table,
        columns,
        values,
        query: None,
        overwrite: false,
        partition,
        directory: None,
        returning,
        output: None,
        on_conflict: None,
        leading_comments: _,
        if_exists: false,
        with: None,
        ignore: false,
        source_alias: None,
        alias: None,
        alias_explicit_as: false,
        default_values: false,
        by_name: false,
        conflict_action: None,
        is_replace: false,
        hint: None,
        replace_where: None,
        source: None,
        function_target: None,
        partition_by: None,
        settings,
    } = insert
    else {
        return Err("has a clause other than VALUES".to_owned());
    };
    let bare_table = TableRef {
        name: table.name.clone(),
        ..TableRef::new("")
    };
    if *table != bare_table
        || !columns.is_empty()
        || values.is_empty()
        || !partition.is_empty()
        || !returning.is_empty()
        || !settings.is_empty()
    {
        return Err("has a clause other than VALUES into an unqualified table".to_owned());
    }
    let name = resolved(&table.name)?;
    let target = converted
        .relations
        .get(&name)
        .ok_or_else(|| format!("inserts into {name}, which setup did not create"))?;
    let mut rows: Vec<String> = Vec::with_capacity(values.len());
    for row in values {
        if row.len() != target.len() {
            return Err(format!(
                "has a row of {} values for {} columns",
                row.len(),
                target.len()
            ));
        }
        let cells: Vec<String> = row
            .iter()
            .zip(target)
            .map(|(value, column)| cell(value, column, log))
            .collect::<Result<_, _>>()?;
        rows.push(format!("({})", cells.join(", ")));
    }
    Ok(format!(
        "INSERT INTO {} VALUES {}",
        quoted_identifier(&name, log),
        rows.join(", ")
    ))
}

/// One value: NULL, a string for a VARCHAR, or an unsigned number a NUMBER column holds unrounded.
fn cell(value: &Expression, column: &ColumnSpec, log: &ProvenanceLog) -> Result<String, String> {
    match (value, column.data_type) {
        (Expression::Null(_), _) => Ok("NULL".to_owned()),
        (Expression::Literal(literal), SnowflakeType::Varchar) => match literal.as_ref() {
            Literal::String(text) => Ok(string_literal(text, log)),
            _ => Err(format!(
                "puts a value other than a string or NULL into {}",
                column.name
            )),
        },
        (Expression::Literal(literal), SnowflakeType::Number { precision, scale }) => {
            match literal.as_ref() {
                Literal::Number(text) if fits(text, precision, scale) => Ok(format!(
                    "CAST({text} AS {})",
                    column_type(column.data_type, log)?
                )),
                _ => Err(format!(
                    "puts a value into {} that is not a number it holds without rounding",
                    column.name
                )),
            }
        }
        _ => Err(format!(
            "puts a value other than a literal or NULL into {}",
            column.name
        )),
    }
}

/// Whether the digits `text` writes, with at most one point, fit NUMBER(precision, scale) exactly.
fn fits(text: &str, precision: u8, scale: u8) -> bool {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let whole = whole.trim_start_matches('0');
    !text.is_empty()
        && text.matches('.').count() <= 1
        && text
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
        && fraction.len() <= usize::from(scale)
        && whole.len() <= usize::from(precision.saturating_sub(scale))
}

/// An unquoted name as Snowflake resolves it, in upper case; a quoted one refuses.
pub(crate) fn resolved(identifier: &Identifier) -> Result<String, String> {
    match identifier.quoted {
        true => Err(format!("quotes the name {}", identifier.name)),
        false => Ok(identifier.name.to_ascii_uppercase()),
    }
}
