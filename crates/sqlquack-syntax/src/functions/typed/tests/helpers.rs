use std::collections::HashSet;

use crate::functions::typed::_helpers::lookups::typed_function_spec_by_name_upper;
use crate::functions::typed::_helpers::lookups::TYPED_FUNCTION_ALIAS_TO_CANONICAL;
use crate::functions::typed::constants::TYPED_DISPATCH_GROUP_SPECS;
use crate::functions::typed::constants::TYPED_FUNCTION_SPECS;
use crate::functions::typed::main::typed_dispatch_group_by_name_upper::typed_dispatch_group_by_name_upper;
use crate::functions::typed::main::typed_function_spec_by_canonical_upper::typed_function_spec_by_canonical_upper;
use crate::functions::typed::models::TypedParseKind;

pub(crate) fn repeated_names() -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    TYPED_FUNCTION_SPECS
        .iter()
        .flat_map(|spec| std::iter::once(spec.canonical_name).chain(spec.aliases.iter().copied()))
        .filter(|name| !seen.insert(name))
        .map(str::to_owned)
        .collect()
}

pub(crate) fn unresolved_names() -> Vec<String> {
    let by_name = TYPED_FUNCTION_SPECS.iter().flat_map(|spec| {
        std::iter::once(spec.canonical_name)
            .chain(spec.aliases.iter().copied())
            .filter(|name| {
                typed_function_spec_by_name_upper(name)
                    .is_none_or(|found| found.canonical_name != spec.canonical_name)
            })
    });
    let by_canonical = TYPED_FUNCTION_SPECS
        .iter()
        .map(|spec| spec.canonical_name)
        .filter(|name| {
            typed_function_spec_by_canonical_upper(name)
                .is_none_or(|found| found.canonical_name != *name)
        });
    by_name.chain(by_canonical).map(str::to_owned).collect()
}

pub(crate) fn stray_aliases() -> Vec<String> {
    TYPED_FUNCTION_ALIAS_TO_CANONICAL
        .iter()
        .filter(|(alias, canonical)| {
            typed_function_spec_by_canonical_upper(canonical)
                .is_none_or(|spec| !spec.aliases.contains(alias))
        })
        .map(|(alias, _)| (*alias).to_owned())
        .collect()
}

pub(crate) fn misgrouped_names() -> Vec<String> {
    TYPED_DISPATCH_GROUP_SPECS
        .iter()
        .filter(|spec| {
            typed_function_spec_by_canonical_upper(spec.name).is_none()
                || typed_dispatch_group_by_name_upper(spec.name) != Some(spec.group)
        })
        .map(|spec| spec.name.to_owned())
        .collect()
}

pub(crate) fn dispatch_group_names() -> HashSet<&'static str> {
    TYPED_DISPATCH_GROUP_SPECS
        .iter()
        .map(|spec| spec.name)
        .collect()
}

pub(crate) fn resolved_spec(name: &str) -> Option<(&'static str, TypedParseKind)> {
    typed_function_spec_by_name_upper(name).map(|spec| (spec.canonical_name, spec.parse_kind))
}
