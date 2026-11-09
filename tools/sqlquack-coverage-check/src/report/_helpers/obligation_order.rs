//! Obligation ids in numeric order, so `o999` precedes `o1000`.

use std::collections::BTreeSet;

use serde::Serializer;

/// The order obligation ids are listed in: by form, then by number, so `o999` precedes `o1000`.
pub(crate) fn obligation_order(id: &str) -> (&str, usize, &str) {
    let (form, number) = id.rsplit_once('.').unwrap_or((id, ""));
    (form, number.len(), number)
}

/// The ids in obligation order, `o999` before `o1000`.
pub(crate) fn sorted_obligations<'a>(ids: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut sorted: Vec<&String> = ids.into_iter().collect();
    sorted.sort_by(|left, right| obligation_order(left).cmp(&obligation_order(right)));
    sorted.into_iter().cloned().collect()
}

pub(crate) fn in_obligation_order<S: Serializer>(
    ids: &BTreeSet<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(sorted_obligations(ids))
}
