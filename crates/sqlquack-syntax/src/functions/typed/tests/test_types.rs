use std::collections::HashSet;

use crate::functions::typed::models::TypedDispatchGroup;
use crate::functions::typed::models::TypedParseKind;

pub(crate) struct RegistryCheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) violations: fn() -> Vec<String>,
    pub(crate) expected_violations: Vec<String>,
}

pub(crate) struct TypedSpecTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_spec: Option<(&'static str, TypedParseKind)>,
}

pub(crate) struct CanonicalNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_canonical: &'static str,
}

pub(crate) struct DispatchGroupTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_group: Option<TypedDispatchGroup>,
}

pub(crate) struct HeavyFamilyTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_names: HashSet<&'static str>,
}
