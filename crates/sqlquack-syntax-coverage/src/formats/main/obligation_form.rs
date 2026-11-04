//! `obligation_form`: the form an obligation id belongs to.

use crate::formats::_helpers::ids::{is_form_id, is_obligation_number};
use crate::formats::constants::OBLIGATION_ZERO;

/// The form of an obligation id `<form>.o<NNN>`, when it is well formed for `family`; `o000` is not.
pub fn obligation_form<'a>(id: &'a str, family: &str) -> Option<&'a str> {
    let (form, number) = id.rsplit_once('.')?;
    (is_obligation_number(number) && &number[1..] != OBLIGATION_ZERO && is_form_id(form, family))
        .then_some(form)
}

/// The order obligation ids are listed in: by form, then by number, so `o999` precedes `o1000`.
pub fn obligation_order(id: &str) -> (&str, usize, &str) {
    let (form, number) = id.rsplit_once('.').unwrap_or((id, ""));
    (form, number.len(), number)
}
