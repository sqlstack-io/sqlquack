//! The workload: wide, CTE-layered Snowflake models over two tables, as a dense project writes them.

use sqlquack_semantics::lineage::models::{Nullability, SchemaColumn};
use sqlquack_semantics::lineage::types::LineageSchema;

use crate::instructions::constants::{AGGREGATE_EVERY, MIN_WIDTH, STATEMENTS, WIDTH_STEP, WIDTHS};

/// The two relations the workload reads, as Snowflake's catalog names them.
#[derive(Debug)]
pub(crate) struct Tables {
    orders: Vec<SchemaColumn>,
    lookup: Vec<SchemaColumn>,
}

impl LineageSchema for Tables {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        match relation {
            "RAW_ORDERS" => Some(&self.orders),
            "ORDER_LOOKUP" => Some(&self.lookup),
            _ => None,
        }
    }
}

fn column(name: &str, data_type: &str) -> SchemaColumn {
    SchemaColumn {
        name: name.to_owned(),
        quoted: false,
        data_type: Some(data_type.to_owned()),
        nullability: Nullability::Unknown,
    }
}

/// `RAW_ORDERS` and `ORDER_LOOKUP`, with the columns the workload reads.
pub(crate) fn tables() -> Tables {
    Tables {
        orders: vec![
            column("ID", "NUMBER(38,0)"),
            column("AMOUNT", "FLOAT"),
            column("STATUS", "VARCHAR"),
            column("ORDERED_AT", "TIMESTAMP_NTZ"),
        ],
        lookup: vec![column("ID", "NUMBER(38,0)"), column("AMOUNT", "FLOAT")],
    }
}

/// `MEASURE_000` to the last of `width` measure columns, each through `spelled`.
fn measures(width: usize, spelled: impl Fn(&str, usize) -> String) -> Vec<String> {
    (0..width)
        .map(|index| spelled(&format!("MEASURE_{index:03}"), index))
        .collect()
}

/// One model with `width` measures: an import, a lookup join, a union, a window and lateral aliases.
fn model(position: usize, width: usize) -> String {
    let names: String = measures(width, |name, _| name.to_owned()).join(", ");
    let projected: String = measures(width, |name, index| {
        format!(
            "CAST(COALESCE(b.AMOUNT, 0) * {} + CAST(b.ID AS DOUBLE) AS DOUBLE) AS {name}",
            index + 1
        )
    })
    .join(",\n    ");
    let cast: String =
        measures(width, |name, _| format!("CAST({name} AS DOUBLE) AS {name}")).join(",\n    ");
    let outer: String = match position % AGGREGATE_EVERY {
        0 => "SELECT STATUS, COUNT(*) AS ORDERS, SUM(ADJUSTED) AS TOTAL, MAX(MEASURE_000) AS TOP\nFROM final\nGROUP BY STATUS".to_owned(),
        _ => format!("SELECT ID, STATUS, SIZE, ADJUSTED, {names}\nFROM final"),
    };
    format!(
        "WITH imported AS (SELECT * FROM RAW_ORDERS),
lookup AS (SELECT ID, AMOUNT FROM ORDER_LOOKUP),
projected AS (
  SELECT CAST(b.ID AS INTEGER) AS ID,
    CAST(b.AMOUNT + COALESCE(l.AMOUNT, 0) AS DOUBLE) AS AMOUNT,
    b.STATUS AS STATUS,
    {projected}
  FROM imported AS b
  LEFT JOIN lookup AS l ON l.ID = b.ID
  WHERE b.ORDERED_AT >= '2024-01-01'
),
combined AS (
  SELECT ID, AMOUNT, STATUS, {names} FROM projected
  UNION ALL
  SELECT ID, AMOUNT, STATUS, {names} FROM projected WHERE ID = {position}
),
ranked AS (
  SELECT ID, AMOUNT, STATUS, {names},
    ROW_NUMBER() OVER (PARTITION BY STATUS ORDER BY AMOUNT DESC) AS POSITION
  FROM combined
),
final AS (
  SELECT CAST(ID AS INTEGER) AS ID,
    STATUS,
    CASE WHEN AMOUNT > 100 THEN 'large' ELSE 'small' END AS SIZE,
    AMOUNT * 2 AS DOUBLED,
    DOUBLED + 1 AS ADJUSTED,
    {cast}
  FROM ranked
  WHERE POSITION <= 10
)
{outer}"
    )
}

/// Every workload statement, its widths cycling from narrow to wide.
pub(crate) fn statements() -> Vec<String> {
    (0..STATEMENTS)
        .map(|position| model(position, MIN_WIDTH + (position % WIDTHS) * WIDTH_STEP))
        .collect()
}
