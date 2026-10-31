//! `assumption_id`: the policy divergence a recorded divergence id names.

/// The policy id of a recorded divergence: `declared_offset` for `declared_offset:<column>`.
pub fn assumption_id(recorded: &str) -> &str {
    recorded.split_once(':').map_or(recorded, |(id, _)| id)
}
