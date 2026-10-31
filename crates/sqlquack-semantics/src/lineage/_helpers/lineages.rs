//! Building and combining the lineage of values.

use std::collections::BTreeSet;

use crate::lineage::models::{Lineage, SourceColumn, TransformKind};

/// A value that reads no column.
pub(crate) fn constant() -> Lineage {
    Lineage {
        sources: BTreeSet::new(),
        kind: TransformKind::Constant,
        resolved: true,
        rows: BTreeSet::new(),
    }
}

/// A value whose columns are not traced: a name no relation offers, or an opaque construct.
pub(crate) fn unresolved() -> Lineage {
    Lineage {
        resolved: false,
        ..constant()
    }
}

/// One source column, unchanged.
pub(crate) fn direct(relation: &str, column: &str, quoted: bool) -> Lineage {
    Lineage {
        sources: BTreeSet::from([SourceColumn {
            relation: relation.to_owned(),
            column: column.to_owned(),
            quoted,
        }]),
        kind: TransformKind::Direct,
        ..constant()
    }
}

/// `lineage`, marked untraced when `traced` is false.
pub(crate) fn traced_if(lineage: Lineage, traced: bool) -> Lineage {
    Lineage {
        resolved: lineage.resolved && traced,
        ..lineage
    }
}

/// The parts read together into one value derived at least as `floor`.
pub(crate) fn combined(parts: Vec<Lineage>, floor: TransformKind) -> Lineage {
    let resolved = parts.iter().all(|part| part.resolved);
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
    }
}

/// Alternative branches merged into one column: direct only when each is one same source unchanged.
pub(crate) fn merged(parts: Vec<Lineage>) -> Lineage {
    let resolved = parts.iter().all(|part| part.resolved);
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
    }
}

/// Every column a value reads, its sources and row dependencies together.
pub(crate) fn everything_read(lineage: &Lineage) -> BTreeSet<SourceColumn> {
    lineage.sources.union(&lineage.rows).cloned().collect()
}

fn unions(parts: Vec<Lineage>) -> (BTreeSet<SourceColumn>, BTreeSet<SourceColumn>) {
    let mut sources: BTreeSet<SourceColumn> = BTreeSet::new();
    let mut rows: BTreeSet<SourceColumn> = BTreeSet::new();
    for part in parts {
        sources.extend(part.sources);
        rows.extend(part.rows);
    }
    (sources, rows)
}
