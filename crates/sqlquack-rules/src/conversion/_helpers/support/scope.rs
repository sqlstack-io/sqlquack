//! Name resolution: relations in scope, CTEs, select-list aliases and outer queries.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_semantics::dialects::constants::SNOWFLAKE_TO_DUCKDB;
use sqlquack_semantics::names::main::relation_columns::relation_columns;
use sqlquack_semantics::scopes::main::resolve_name::resolve_name;
use sqlquack_semantics::scopes::models::{Resolution, Scope as SemanticScope};
use sqlquack_semantics::types::models::TypedLeaves;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::support::locations::Locator;

use crate::conversion::models::ConversionOptions;
use sqlquack_semantics::types::models::{ColumnSpec, Holding, Schema};

/// A converted scalar: DuckDB SQL whose type is the DuckDB pairing of `data_type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Emitted {
    pub(crate) sql: String,
    pub(crate) data_type: SnowflakeType,
    /// For literals: the literal's text, which some rules read.
    pub(crate) literal: Option<Literal>,
    /// For TIMESTAMP_TZ values: whether every value's offset is known to be +00:00.
    pub(crate) utc_offset: bool,
    /// The schema columns whose declared offset the UTC mark relies on, as `RELATION.COLUMN`.
    pub(crate) declared: Vec<String>,
    /// For VARIANT values: how DuckDB holds them.
    pub(crate) held: Holding,
    /// For VARIANT-held values: what their root is known to be.
    pub(crate) root: VariantRoot,
    /// Whether the value is proven to hold no typed leaf: built without one, reading only such values.
    pub(crate) untainted: bool,
    /// Whether the value reads a source or schema VARIANT trusted to hold no typed leaf.
    pub(crate) sourced: bool,
}

/// What a VARIANT-held value's root is known to be, which decides how it is written as text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VariantRoot {
    /// An object or an array.
    Container,
    /// A FLOAT, whose text Snowflake writes differently when it folds a constant.
    Double,
    /// A value a path reads out of another: a FLOAT there is written as its own text, folded or not.
    Element,
    /// Unknown until run time, such as a column.
    #[default]
    Dynamic,
}

/// A literal's kind and text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Literal {
    Text(String),
    Number(String),
}

impl Emitted {
    pub(crate) fn new(sql: String, data_type: SnowflakeType) -> Self {
        Self {
            sql,
            data_type,
            literal: None,
            utc_offset: false,
            declared: Vec::new(),
            held: Holding::Json,
            root: VariantRoot::Dynamic,
            untainted: false,
            sourced: false,
        }
    }

    /// A value built from typed scalars, held as a DuckDB `VARIANT` or its JSON, its root `root`.
    pub(crate) fn variant_held(sql: String, root: VariantRoot) -> Self {
        Self {
            held: Holding::Variant,
            root,
            ..Self::new(sql, SnowflakeType::SemiStructured)
        }
    }

    /// Whether the value is a DuckDB `VARIANT` built from typed scalars.
    pub(crate) fn is_variant_held(&self) -> bool {
        self.held == Holding::Variant
    }
}

/// What a query level can see, its aliases bound to converted values.
pub(crate) type Scope<'a> = SemanticScope<'a, Emitted>;

/// Resolves a name to its converted value: a column read through its relation, or an alias's.
pub(crate) fn resolve(
    scope: &Scope<'_>,
    qualifier: Option<&str>,
    name: &str,
    log: &ProvenanceLog,
) -> Result<Emitted, Refusal> {
    match resolve_name(scope, (qualifier, name), SNOWFLAKE_TO_DUCKDB)? {
        Resolution::Alias(emitted) => Ok(emitted.clone()),
        Resolution::Column { alias, column } => Ok(Emitted {
            held: column.holding,
            untainted: column.untainted,
            sourced: column.sourced,
            utc_offset: column.utc_offset,
            declared: column.declared_offsets.clone(),
            ..Emitted::new(
                format!(
                    "{}.{}",
                    quoted_identifier(alias, log),
                    quoted_identifier(&column.name, log)
                ),
                column.data_type,
            )
        }),
    }
}

/// The relations a statement can read: the schema, then CTEs from outermost to innermost.
#[derive(Clone, Debug)]
pub(crate) struct Context<'a> {
    pub(crate) log: &'a ProvenanceLog,
    pub(crate) schema: &'a Schema,
    pub(crate) options: &'a ConversionOptions,
    pub(crate) ctes: Vec<(String, Vec<ColumnSpec>)>,
    pub(crate) locator: &'a Locator<'a>,
    pub(crate) typed: &'a TypedLeaves,
}

impl Context<'_> {
    /// The columns of a relation name: the innermost CTE, else the schema.
    pub(crate) fn relation(&self, name: &str) -> Result<Vec<ColumnSpec>, Refusal> {
        relation_columns(
            name,
            (&self.ctes, self.schema),
            self.typed,
            SNOWFLAKE_TO_DUCKDB,
        )
    }
}
