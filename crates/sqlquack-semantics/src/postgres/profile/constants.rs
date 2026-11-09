//! The values PostgreSQL's profile and lineage rules read.

/// PostgreSQL's system columns, which every stored table has and no star expands.
pub(crate) const POSTGRES_SYSTEM_COLUMNS: [&str; 6] =
    ["tableoid", "xmin", "cmin", "xmax", "cmax", "ctid"];
