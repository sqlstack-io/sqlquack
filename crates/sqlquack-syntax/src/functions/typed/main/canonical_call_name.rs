//! `canonical_call_name`: the name a call is checked under, as polyglot's lowering spells it.

use crate::functions::typed::main::canonical_typed_function_name_upper::canonical_typed_function_name_upper;

/// The lower-case canonical name of a call's last dotted part, `DATE_DIFF` as `datediff`.
pub fn canonical_call_name(name: &str) -> String {
    let upper: String = name
        .rsplit('.')
        .next()
        .unwrap_or(name)
        .trim()
        .to_uppercase();
    canonical_typed_function_name_upper(&upper).to_lowercase()
}
