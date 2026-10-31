//! Building and combining the lineage of values.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::lineage::models::{Lineage, RowReads, SourceColumn, TransformKind};
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

/// The parts read together into one value derived at least as `floor`.
pub(crate) fn combined(parts: Vec<Lineage>, floor: TransformKind) -> Lineage {
    let resolved = parts.iter().all(|part| part.resolved);
    let lost: bool = parts.iter().any(|part| part.lost);
    let rows_lost: bool = parts.iter().any(|part| part.rows_lost);
    let strongest = parts
        .iter()
        .filter(|part| !part.sources.is_empty() || part.kind == TransformKind::Aggregation)
        .map(|part| part.kind)
        .max();
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
    }
}

/// Alternative branches merged into one column: direct only when each is one same source unchanged.
pub(crate) fn merged(parts: Vec<Lineage>) -> Lineage {
    let resolved = parts.iter().all(|part| part.resolved);
    let lost: bool = parts.iter().any(|part| part.lost);
    let rows_lost: bool = parts.iter().any(|part| part.rows_lost);
    let aggregated = parts
        .iter()
        .any(|part| part.kind == TransformKind::Aggregation);
    let all_direct = parts.iter().all(|part| part.kind == TransformKind::Direct);
    let strongest = parts.iter().map(|part| part.kind).max();
    let (sources, rows) = unions(parts);
    let kind = match (sources.len(), all_direct, aggregated, strongest) {
        (_, _, true, _) => TransformKind::Aggregation,
        (0, _, _, _) => TransformKind::Constant,
        (1, true, _, _) => TransformKind::Direct,
        (_, _, _, strongest) => strongest
            .unwrap_or(TransformKind::Expression)
            .max(TransformKind::Expression),
    };
    Lineage {
        sources,
        kind,
        resolved,
        rows,
        lost,
        rows_lost,
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
