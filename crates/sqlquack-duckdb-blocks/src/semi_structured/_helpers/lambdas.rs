//! The elements a FILTER or TRANSFORM lambda runs over.

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::JSON_FIDELITY;

/// `body` over `array` bound as `q`; NULL for NULL, raising for neither an array nor an object.
pub(crate) fn raising_non_arrays(array: &str, body: &str) -> String {
    let other = raised(&format!(
        "{JSON_FIDELITY}: a lambda over a value other than an array or object"
    ));
    format!(
        "list_transform([CAST({array} AS JSON)], lambda q: CASE WHEN q IS NULL THEN NULL \
         WHEN json_type(q) NOT IN ('ARRAY', 'OBJECT') THEN CAST({other} AS JSON) ELSE {body} END)[1]"
    )
}

/// The elements of `array`, JSON nulls kept, an object as the only element.
pub(crate) fn elements(array: &str) -> String {
    format!(
        "json_extract(CASE WHEN json_type({array}) = 'OBJECT' THEN CAST(to_json([{array}]) AS JSON) \
         ELSE {array} END, '$[*]')"
    )
}

/// The raise of an array element Snowflake keeps as undefined.
pub(crate) fn undefined_element() -> String {
    format!(
        "CAST({} AS JSON)",
        raised(&format!(
            "{JSON_FIDELITY}: an array element Snowflake keeps as undefined"
        ))
    )
}
