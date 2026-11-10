//! Building and combining the lineage of values.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::dialects::main::dialect_rules::dialect_rules;
use crate::dialects::types::DialectProfile;
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
        json_null_free: false,
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

/// The most significant kind among parts that read a column or aggregate; none when none does.
fn strongest_step(parts: &[Lineage]) -> Option<TransformKind> {
    parts
        .iter()
        .filter(|part| !part.sources.is_empty() || part.kind == TransformKind::Aggregation)
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
    let kind = match (strongest, floor) {
        (_, TransformKind::Aggregation) => TransformKind::Aggregation,
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
        json_null_free: false,
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

/// Whether `part` is a bare NULL: a constant of no type that may be NULL.
fn untyped_null(part: &Lineage) -> bool {
    part.kind == TransformKind::Constant
        && part.data_type.is_none()
        && part.nullability == Nullability::Nullable
        && part.sources.is_empty()
}

/// Whether a branch that reads a column has a known type other than the merged one.
fn implicitly_cast(branch: &Lineage, merged: &str, profile: &dyn DialectProfile) -> bool {
    match branch.data_type.as_deref() {
        Some(own) => !branch.sources.is_empty() && !profile.same_type(own, merged),
        None => false,
    }
}

/// `lineage` at least a cast when a branch it merges, reading a column, is implicitly coerced.
pub(crate) fn coerced(
    mut lineage: Lineage,
    branches: &[Lineage],
    profile: &dyn DialectProfile,
) -> Lineage {
    let cast: bool = match lineage.data_type.as_deref() {
        Some(merged) => branches
            .iter()
            .any(|branch| implicitly_cast(branch, merged, profile)),
        None => {
            let read: Vec<&str> = branches
                .iter()
                .filter(|branch| !branch.sources.is_empty())
                .filter_map(|branch| branch.data_type.as_deref())
                .collect();
            read.iter()
                .any(|own| read.iter().any(|other| !profile.same_type(own, other)))
        }
    };
    if cast {
        lineage.kind = lineage.kind.max(TransformKind::Cast);
    }
    lineage
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
    lineage.data_type =
        dialect_rules(profile).set_type(lineage.data_type.take(), &branches, &parts_typed);
    coerced(lineage, &parts_typed, profile)
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
        _ => dialect_rules(profile).merged_type(&typed),
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
        json_null_free: false,
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
