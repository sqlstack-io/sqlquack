//! Name resolution: relations in scope, CTEs, select-list aliases and outer queries.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::support::tree::unresolved;
use crate::conversion::models::{ColumnSpec, ConversionOptions, Schema};

/// A converted scalar: DuckDB SQL whose type is the DuckDB pairing of `data_type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Emitted {
    pub(crate) sql: String,
    pub(crate) data_type: SnowflakeType,
    /// For literals: the literal's text, which some rules read.
    pub(crate) literal: Option<Literal>,
    /// For TIMESTAMP_TZ values: whether every value's offset is known to be +00:00.
    pub(crate) utc_offset: bool,
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
        }
    }
}

/// A relation visible in a FROM clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScopeRelation {
    pub(crate) alias: String,
    pub(crate) columns: Vec<ColumnSpec>,
}

/// What a query level can see.
#[derive(Clone, Debug)]
pub(crate) struct Scope<'a> {
    pub(crate) relations: Vec<ScopeRelation>,
    pub(crate) aliases: Vec<(String, Emitted)>,
    pub(crate) outer: Option<&'a Scope<'a>>,
    /// Whether unqualified names resolve to select aliases first, as in Snowflake's ORDER BY.
    pub(crate) aliases_first: bool,
}

impl<'a> Scope<'a> {
    pub(crate) fn new(relations: Vec<ScopeRelation>, outer: Option<&'a Scope<'a>>) -> Self {
        Self {
            relations,
            aliases: Vec::new(),
            outer,
            aliases_first: false,
        }
    }

    /// Resolves a name: FROM columns, then select aliases, then outer queries; case-only clashes refuse.
    pub(crate) fn resolve(
        &self,
        qualifier: Option<&str>,
        name: &str,
        log: &ProvenanceLog,
    ) -> Result<Emitted, Refusal> {
        if self.aliases_first
            && qualifier.is_none()
            && let Some((_, emitted)) = self.aliases.iter().rev().find(|(alias, _)| alias == name)
        {
            return Ok(emitted.clone());
        }
        let mut found: Vec<Emitted> = Vec::new();
        for relation in &self.relations {
            if qualifier.is_some_and(|qualifier| !same_name(&relation.alias, qualifier)) {
                continue;
            }
            for column in relation
                .columns
                .iter()
                .filter(|column| same_name(&column.name, name))
            {
                if column.name != name
                    || qualifier.is_some_and(|qualifier| relation.alias != qualifier)
                {
                    return Err(unresolved(format!(
                        "`{name}` matches `{}` only without regard to case, which DuckDB \
                         cannot tell apart",
                        column.name
                    )));
                }
                found.push(Emitted::new(
                    format!(
                        "{}.{}",
                        quoted_identifier(&relation.alias, log),
                        quoted_identifier(&column.name, log)
                    ),
                    column.data_type,
                ));
            }
        }
        match found.len() {
            1 => return Ok(found.remove(0)),
            0 => {}
            _ => return Err(unresolved(format!("column `{name}` is ambiguous"))),
        }
        if qualifier.is_none()
            && let Some((_, emitted)) = self.aliases.iter().rev().find(|(alias, _)| alias == name)
        {
            return Ok(emitted.clone());
        }
        match self.outer {
            Some(outer) => outer.resolve(qualifier, name, log),
            None => Err(unresolved(match qualifier {
                Some(qualifier) => format!("column `{qualifier}.{name}` does not resolve"),
                None => format!("column `{name}` does not resolve"),
            })),
        }
    }

    /// The relations a star expands to: one by alias, or all.
    pub(crate) fn star_relations(
        &self,
        qualifier: Option<&str>,
    ) -> Result<Vec<&ScopeRelation>, Refusal> {
        let relations: Vec<&ScopeRelation> = self
            .relations
            .iter()
            .filter(|relation| qualifier.is_none_or(|qualifier| relation.alias == qualifier))
            .collect();
        if relations.is_empty() {
            return Err(unresolved(format!(
                "`{}.*` names no relation in scope",
                qualifier.unwrap_or_default()
            )));
        }
        Ok(relations)
    }
}

fn same_name(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

/// The relations a statement can read: the schema, then CTEs from outermost to innermost.
#[derive(Clone, Debug)]
pub(crate) struct Context<'a> {
    pub(crate) log: &'a ProvenanceLog,
    pub(crate) schema: &'a Schema,
    pub(crate) options: &'a ConversionOptions,
    pub(crate) ctes: Vec<(String, Vec<ColumnSpec>)>,
}

impl Context<'_> {
    /// The columns of a relation name: the innermost CTE, else the schema.
    pub(crate) fn relation(&self, name: &str) -> Result<Vec<ColumnSpec>, Refusal> {
        if let Some((_, columns)) = self.ctes.iter().rev().find(|(cte, _)| cte == name) {
            return Ok(columns.clone());
        }
        if let Some(columns) = self.schema.relations.get(name) {
            return Ok(columns.clone());
        }
        if self
            .ctes
            .iter()
            .map(|(cte, _)| cte.as_str())
            .chain(self.schema.relations.keys().map(String::as_str))
            .any(|known| same_name(known, name))
        {
            return Err(unresolved(format!(
                "relation `{name}` matches another only without regard to case"
            )));
        }
        Err(unresolved(format!(
            "relation `{name}` has no known columns"
        )))
    }
}
