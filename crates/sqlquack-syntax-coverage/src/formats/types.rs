//! Syntax-coverage type aliases.

use std::collections::BTreeMap;

use crate::formats::models::{Definition, Form};

/// Each family's forms and definitions, in file order.
pub(crate) type FamilyCatalogue = BTreeMap<String, (Vec<Form>, Vec<Definition>)>;
