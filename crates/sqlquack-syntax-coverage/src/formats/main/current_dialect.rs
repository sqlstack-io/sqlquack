//! `current_dialect`: the dialect the loaded root on this thread declared.

use std::rc::Rc;

use crate::formats::_helpers::dialect;
use crate::formats::models::Dialect;

/// The dialect this thread loads and checks under; Snowflake's until a root with a dialect file.
pub fn current_dialect() -> Rc<Dialect> {
    dialect::current_dialect()
}
