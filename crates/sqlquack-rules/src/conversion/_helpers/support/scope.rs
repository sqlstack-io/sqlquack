//! Name resolution: relations in scope, CTEs, select-list aliases and outer queries.

use sqlquack_core::parsing::models::SourceLocation;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_semantics::dialects::constants::SNOWFLAKE_TO_DUCKDB;
use sqlquack_semantics::names::main::relation_columns::relation_columns;
use sqlquack_semantics::scopes::main::resolve_name::resolve_name;
use sqlquack_semantics::scopes::models::{Resolution, Scope as SemanticScope};
use sqlquack_semantics::types::models::TypedLeaves;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::support::locations::Locator;
use crate::conversion::_helpers::support::tree::node_construct;
use crate::conversion::constants::SUBQUERY_PARAMETER_PREFIX;

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
    /// For a TIMESTAMP_TZ from CONVERT_TIMEZONE of two arguments: its target zone's DuckDB SQL.
    pub(crate) zone: Option<String>,
    /// For VARIANT values: how DuckDB holds them.
    pub(crate) held: Holding,
    /// For VARIANT-held values: what their root is known to be.
    pub(crate) root: VariantRoot,
    /// Whether the value is proven to hold no typed leaf: built without one, reading only such values.
    pub(crate) untainted: bool,
    /// Whether the value reads a source or schema VARIANT trusted to hold no typed leaf.
    pub(crate) sourced: bool,
    /// Whether the value is proven never NULL: a non-NULL literal, a column proven so, or a cast of one.
    pub(crate) not_null: bool,
    /// Whether the value depends on an array order Snowflake leaves undefined.
    pub(crate) unordered: bool,
    /// Whether the value is SYSTEM$TYPEOF text whose plan-dependent storage suffix is still on.
    pub(crate) suffixed: bool,
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
            zone: None,
            held: Holding::Json,
            root: VariantRoot::Dynamic,
            untainted: false,
            sourced: false,
            not_null: false,
            unordered: false,
            suffixed: false,
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
            sql: read_column(alias, column, log)?,
            held: column.holding,
            untainted: column.untainted,
            sourced: column.sourced,
            utc_offset: column.utc_offset,
            declared: column.declared_offsets.clone(),
            not_null: column.not_null,
            unordered: column.unordered,
            suffixed: column.suffixed,
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

/// A column read through its alias; an inner USING key TRY_CASTs, as unmatched rows never survive.
pub(crate) fn read_column(
    alias: &str,
    column: &ColumnSpec,
    log: &ProvenanceLog,
) -> Result<String, Refusal> {
    let sql = format!(
        "{}.{}",
        quoted_identifier(alias, log),
        quoted_identifier(&column.name, log)
    );
    match column.widened {
        true => cast(&sql, column.data_type, log)
            .map(|cast_sql| format!("TRY_{cast_sql}"))
            .map_err(|reason| outside_conditions(node_construct("column"), reason)),
        false => Ok(sql),
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
    /// Unordered values passed up unchecked so far, to tell what a conversion passed up.
    pub(crate) order: &'a OrderTrack,
    /// The subquery values emitted so far, which DuckDB cannot bind inside a lambda body.
    pub(crate) subqueries: &'a SubqueryTrack,
    /// The name each schema relation was written as so far.
    pub(crate) qualifiers: &'a QualifierTrack,
    /// Whether query results may keep unordered arrays: CTE and FROM subquery bodies.
    pub(crate) free_projections: bool,
    /// Whether this converts a case's input fixture, whose literals stand for source data.
    pub(crate) fixture_body: bool,
}

/// How many unordered values were passed up unchecked, to positions where order cannot matter.
#[derive(Debug, Default)]
pub(crate) struct OrderTrack {
    passed: std::cell::Cell<usize>,
}

impl OrderTrack {
    /// An unordered value passed up unchecked.
    pub(crate) fn pass(&self) {
        self.passed.set(self.passed.get() + 1);
    }

    /// The count so far.
    pub(crate) fn mark(&self) -> usize {
        self.passed.get()
    }

    /// Forgets what was passed up since `mark`, which a consumer has absorbed.
    pub(crate) fn absorb(&self, mark: usize) {
        self.passed.set(mark);
    }
}

/// What a statement's conversion tracks as it goes: unordered values passed up, subqueries emitted.
#[derive(Debug, Default)]
pub(crate) struct StatementTracks {
    pub(crate) order: OrderTrack,
    pub(crate) subqueries: SubqueryTrack,
    pub(crate) qualifiers: QualifierTrack,
}

/// The name each schema relation was written as, so two written relations never read one.
#[derive(Debug, Default)]
pub(crate) struct QualifierTrack {
    read: std::cell::RefCell<std::collections::HashMap<String, String>>,
}

impl QualifierTrack {
    /// Records the relation `key` read as `written`; the earlier name when it was written otherwise.
    pub(crate) fn read(&self, key: &str, written: &str) -> Option<String> {
        let mut read = self.read.borrow_mut();
        let earlier = read
            .entry(key.to_owned())
            .or_insert_with(|| written.to_owned());
        (earlier != written).then(|| earlier.clone())
    }
}

/// A subquery value emitted in a statement's conversion.
#[derive(Clone, Debug)]
pub(crate) struct SubqueryValue {
    pub(crate) sql: String,
    /// Whether it may call a function whose result can change between calls.
    pub(crate) volatile: bool,
    pub(crate) location: Option<SourceLocation>,
}

/// The subquery values a statement's conversion emitted, in order, and the parameters it bound.
#[derive(Debug, Default)]
pub(crate) struct SubqueryTrack {
    values: std::cell::RefCell<Vec<SubqueryValue>>,
    bound: std::cell::Cell<usize>,
}

impl SubqueryTrack {
    /// A subquery value emitted.
    pub(crate) fn record(&self, value: SubqueryValue) {
        self.values.borrow_mut().push(value);
    }

    /// The count so far.
    pub(crate) fn mark(&self) -> usize {
        self.values.borrow().len()
    }

    /// The values emitted since `mark`.
    pub(crate) fn since(&self, mark: usize) -> Vec<SubqueryValue> {
        self.values.borrow()[mark..].to_vec()
    }

    /// A lambda parameter name not yet used in the statement.
    pub(crate) fn parameter(&self) -> String {
        self.bound.set(self.bound.get() + 1);
        format!("{SUBQUERY_PARAMETER_PREFIX}{}", self.bound.get())
    }
}

impl Context<'_> {
    /// The same context for a query whose results may, with `free`, keep unordered arrays.
    pub(crate) fn projecting(&self, free: bool) -> Self {
        Self {
            free_projections: free,
            ..self.clone()
        }
    }

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
