use std::collections::HashSet;

use crate::functions::names::_helpers::lookups::FUNCTION_SPECS;

pub(crate) fn duplicates(names: &[&'static str]) -> Vec<&'static str> {
    let mut seen: HashSet<&str> = HashSet::new();
    names
        .iter()
        .copied()
        .filter(|name| !seen.insert(name))
        .collect()
}

pub(crate) fn shared(left: &[&'static str], right: &[&'static str]) -> Vec<&'static str> {
    left.iter()
        .copied()
        .filter(|name| right.contains(name))
        .collect()
}

pub(crate) fn not_no_paren(names: &[&'static str]) -> Vec<&'static str> {
    names
        .iter()
        .copied()
        .filter(|name| !FUNCTION_SPECS.get(name).is_some_and(|spec| spec.no_paren))
        .collect()
}

pub(crate) fn not_aggregate(names: &[&'static str]) -> Vec<&'static str> {
    names
        .iter()
        .copied()
        .filter(|name| !FUNCTION_SPECS.get(name).is_some_and(|spec| spec.aggregate))
        .collect()
}
