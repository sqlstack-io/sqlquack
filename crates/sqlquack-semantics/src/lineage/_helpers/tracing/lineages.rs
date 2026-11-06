//! Building and combining the lineage of values.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::dialects::types::DialectProfile;
use crate::lineage::_helpers::typing::duckdb_types::{DuckType, common_type, parsed_type};
use crate::lineage::_helpers::typing::postgres_typing::postgres_set_type;
use crate::lineage::_helpers::typing::snowflake_sets::snowflake_set_type;
use crate::lineage::constants::{DUCKDB_PROFILE, POSTGRES_PROFILE, SNOWFLAKE_PROFILE};
use crate::lineage::models::{Lineage, Nullability, RowReads, SourceColumn, TransformKind};
use crate::lineage::types::SharedColumns;

/// A value that reads no column.
pub(crate) fn constant() -> Lineage {
    Lineage {
        sources: SharedColumns::default(),
        kind: TransformKind::Constant,
        resolved: true,
        rows: SharedColumns::default(),
        lost: false,
        rows_lost: false,
        data_type: None,
        nullability: Nullability::Unknown,
    }
}

/// A value whose columns are not traced: a name no relation offers, or an opaque construct.
pub(crate) fn unresolved() -> Lineage {
    Lineage {
        resolved: false,
        lost: true,
        ..constant()
    }
}

/// One source column, unchanged.
pub(crate) fn direct(relation: &str, column: &str, quoted: bool) -> Lineage {
    Lineage {
        sources: Rc::new(BTreeSet::from([SourceColumn {
            relation: relation.to_owned(),
            column: column.to_owned(),
            quoted,
        }])),
        kind: TransformKind::Direct,
        ..constant()
    }
}

/// `lineage`, marked untraced when `traced` is false.
pub(crate) fn traced_if(lineage: Lineage, traced: bool) -> Lineage {
    Lineage {
        resolved: lineage.resolved && traced,
        lost: lineage.lost || !traced,
        ..lineage
    }
}

/// The most significant kind among parts that read a column or aggregate or window; none when none does.
fn strongest_step(parts: &[Lineage]) -> Option<TransformKind> {
    parts
        .iter()
        .filter(|part| !part.sources.is_empty() || part.kind >= TransformKind::Window)
        .map(|part| part.kind)
        .max()
}

/// The parts read together into one value derived at least as `floor`.
pub(crate) fn combined(parts: Vec<Lineage>, floor: TransformKind) -> Lineage {
    let resolved = parts.iter().all(|part| part.resolved);
    let lost: bool = parts.iter().any(|part| part.lost);
    let rows_lost: bool = parts.iter().any(|part| part.rows_lost);
    let strongest: Option<TransformKind> = strongest_step(&parts);
    let (sources, rows) = unions(parts);
    let kind: TransformKind = match (strongest, floor) {
        (_, TransformKind::Aggregation) => TransformKind::Aggregation,
        (None, TransformKind::Window) => TransformKind::Window,
        (None, _) => TransformKind::Constant,
        (Some(strongest), floor) => strongest.max(floor),
    };
    Lineage {
        sources,
        kind,
        resolved,
        rows,
        lost,
        rows_lost,
        data_type: None,
        nullability: Nullability::Unknown,
    }
}

/// Whether every one of `parts` has a type that is `data_type`.
fn all_typed_as(parts: &[&Lineage], data_type: &str, profile: &dyn DialectProfile) -> bool {
    parts.iter().all(|part| typed_as(part, data_type, profile))
}

/// Whether `part` has a type that is `data_type`.
fn typed_as(part: &Lineage, data_type: &str, profile: &dyn DialectProfile) -> bool {
    match &part.data_type {
        Some(own) => profile.same_type(own, data_type),
        None => false,
    }
}

/// The type DuckDB combines every typed branch into, when each branch's type is modelled.
fn duckdb_common_type(parts: &[&Lineage]) -> Option<Rc<str>> {
    let mut combined: Option<DuckType> = None;
    for part in parts {
        let own: DuckType = parsed_type(part.data_type.as_deref()?)?;
        combined = Some(match combined {
            None => own,
            Some(current) => common_type(&current, &own)?,
        });
    }
    combined.map(|combined| Rc::from(combined.spelling()))
}

/// Whether `part` is a bare NULL: a constant of no type that may be NULL.
fn untyped_null(part: &Lineage) -> bool {
    part.kind == TransformKind::Constant
        && part.data_type.is_none()
        && part.nullability == Nullability::Nullable
        && part.sources.is_empty()
}

/// Whether a branch that reads a column has a known type other than the set operation's.
fn implicitly_cast(branch: &Lineage, merged: Option<&str>, profile: &dyn DialectProfile) -> bool {
    let (Some(own), Some(merged)) = (branch.data_type.as_deref(), merged) else {
        return false;
    };
    !branch.sources.is_empty() && !profile.same_type(own, merged)
}

/// A set operation's branches merged, typed as the dialect types set operations.
pub(crate) fn set_merged(parts: Vec<Lineage>, profile: &dyn DialectProfile) -> Lineage {
    let branches: Vec<Lineage> = parts
        .iter()
        .filter(|part| !untyped_null(part))
        .cloned()
        .collect();
    let parts_typed: Vec<Lineage> = parts.clone();
    let mut lineage: Lineage = merged(parts, profile);
    if lineage.data_type.is_none() && profile.name() == SNOWFLAKE_PROFILE {
        let typed: Vec<&Lineage> = branches.iter().collect();
        lineage.data_type = snowflake_set_type(&typed);
    }
    if profile.name() == POSTGRES_PROFILE {
        lineage.data_type = postgres_set_type(&parts_typed).unwrap_or(lineage.data_type);
    }
    let coerced: bool = parts_typed
        .iter()
        .any(|branch| implicitly_cast(branch, lineage.data_type.as_deref(), profile));
    if coerced {
        lineage.kind = lineage.kind.max(TransformKind::Cast);
    }
    lineage
}

/// Alternative branches merged into one column, which takes the most significant branch's kind.
pub(crate) fn merged(parts: Vec<Lineage>, profile: &dyn DialectProfile) -> Lineage {
    let resolved = parts.iter().all(|part| part.resolved);
    let lost: bool = parts.iter().any(|part| part.lost);
    let rows_lost: bool = parts.iter().any(|part| part.rows_lost);
    let typed: Vec<&Lineage> = parts.iter().filter(|part| !untyped_null(part)).collect();
    let first_type: Option<Rc<str>> = typed.first().and_then(|part| part.data_type.clone());
    let data_type: Option<Rc<str>> = match first_type {
        Some(first) if all_typed_as(&typed, &first, profile) => Some(first),
        _ if profile.name() == DUCKDB_PROFILE => duckdb_common_type(&typed),
        _ => None,
    };
    let nullability: Nullability = match (
        parts
            .iter()
            .all(|part| part.nullability == Nullability::NonNull),
        parts
            .iter()
            .any(|part| part.nullability == Nullability::Nullable),
    ) {
        (true, _) if !parts.is_empty() => Nullability::NonNull,
        (_, true) => Nullability::Nullable,
        _ => Nullability::Unknown,
    };
    let kind: TransformKind = strongest_step(&parts).unwrap_or(TransformKind::Constant);
    let (sources, rows) = unions(parts);
    Lineage {
        sources,
        kind,
        resolved,
        rows,
        lost,
        rows_lost,
        data_type,
        nullability,
    }
}

impl RowReads {
    /// Everything `lineage` reads, as columns deciding rows.
    pub(crate) fn read(&mut self, lineage: &Lineage) {
        self.columns.extend(lineage.sources.iter().cloned());
        self.lost |= lineage.lost;
        self.rows_of(lineage);
    }

    /// The columns deciding rows of the queries inside `lineage`.
    pub(crate) fn rows_of(&mut self, lineage: &Lineage) {
        self.columns.extend(lineage.rows.iter().cloned());
        self.lost |= lineage.rows_lost;
    }

    /// Another query's row-deciding columns.
    pub(crate) fn absorb(&mut self, other: &RowReads) {
        self.columns.extend(other.columns.iter().cloned());
        self.lost |= other.lost;
    }
}

fn unions(parts: Vec<Lineage>) -> (SharedColumns, SharedColumns) {
    let mut sources: Vec<SharedColumns> = Vec::with_capacity(parts.len());
    let mut rows: Vec<SharedColumns> = Vec::with_capacity(parts.len());
    for part in parts {
        sources.push(part.sources);
        rows.push(part.rows);
    }
    (shared_union(sources), shared_union(rows))
}

/// The union of `sets`, sharing one of them when it already holds the others.
fn shared_union(sets: Vec<SharedColumns>) -> SharedColumns {
    let mut union: SharedColumns = SharedColumns::default();
    for set in sets {
        if set.is_empty() || Rc::ptr_eq(&union, &set) || set.is_subset(&union) {
            continue;
        }
        if union.is_subset(&set) {
            union = set;
            continue;
        }
        Rc::make_mut(&mut union).extend(set.iter().cloned());
    }
    union
}
