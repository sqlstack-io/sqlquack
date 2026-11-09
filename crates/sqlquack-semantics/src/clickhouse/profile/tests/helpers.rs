use std::collections::HashMap;

use sqlquack_syntax::Expression;

use crate::dialects::constants::CLICKHOUSE;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::replayed_types;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::{EngineTypeReplay, TypedShopSchema};

/// Each recorded ClickHouse result type against the ClickHouse profile's, `Nullable` set aside.
pub(crate) fn replayed_clickhouse_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, unwrapped_clickhouse, clickhouse_output_type)
}

/// The first output's type under the ClickHouse profile, when the native parser reads the query.
fn clickhouse_output_type(schema: &TypedShopSchema, sql: &str) -> Option<String> {
    let parsed = sqlquack_syntax_clickhouse::parse(sql).ok()?;
    let context = LineageContext {
        profile: &CLICKHOUSE,
        schema,
        sql: Some(sql),
    };
    statement_lineage(parsed.statements.first()?, context)
        .outputs
        .first()?
        .data_type
        .clone()
}

/// The first statement of `sql`, parsed by the native ClickHouse parser.
pub(crate) fn parsed_clickhouse(sql: &str) -> Expression {
    let mut parsed =
        sqlquack_syntax_clickhouse::parse(sql).unwrap_or_else(|error| panic!("{sql}: {error:?}"));
    parsed.statements.remove(0)
}

/// A ClickHouse type without its top-level `LowCardinality` and `Nullable` wrappers.
pub(crate) fn unwrapped_clickhouse(spelling: &str) -> String {
    ["LowCardinality(", "Nullable("]
        .iter()
        .fold(spelling.trim(), |text, wrapper| {
            text.strip_prefix(wrapper)
                .and_then(|inner| inner.strip_suffix(')'))
                .unwrap_or(text)
        })
        .to_owned()
}

/// A ClickHouse shop: types as ClickHouse names them.
pub(crate) fn clickhouse_shop_schema() -> TypedShopSchema {
    let table = |columns: &[(&str, &str)]| {
        columns
            .iter()
            .map(|(name, data_type)| typed_column(name, data_type))
            .collect()
    };
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                table(&[
                    ("id", "Int32"),
                    ("quantity", "Int32"),
                    ("price", "Decimal(10, 2)"),
                    ("status", "String"),
                ]),
            ),
            (
                "customers".to_owned(),
                table(&[("id", "Int32"), ("name", "String")]),
            ),
            (
                "nested".to_owned(),
                table(&[
                    ("arr", "Array(Tuple(a Int32))"),
                    ("tup", "Tuple(a Int32)"),
                    ("zq.order_id", "Int32"),
                ]),
            ),
        ]),
        ..TypedShopSchema::default()
    }
}

/// Three ClickHouse tables: `a` and `b` share `id` and `v`, whose types differ; `c` shares none.
pub(crate) fn clickhouse_join_schema() -> TypedShopSchema {
    let table = |columns: &[(&str, &str)]| {
        columns
            .iter()
            .map(|(name, data_type)| typed_column(name, data_type))
            .collect()
    };
    TypedShopSchema {
        closed: HashMap::from([
            (
                "a".to_owned(),
                table(&[("id", "Int32"), ("v", "Int32"), ("k", "String")]),
            ),
            (
                "b".to_owned(),
                table(&[("id", "Int32"), ("v", "String"), ("w", "Int64")]),
            ),
            ("c".to_owned(), table(&[("z", "Int32")])),
        ]),
        ..TypedShopSchema::default()
    }
}
