//! Syntax-coverage type aliases.

use std::collections::BTreeMap;

use crate::formats::models::{Definition, Form, Receipt};

/// Each family's forms and definitions, in file order.
pub(crate) type FamilyCatalogue = BTreeMap<String, (Vec<Form>, Vec<Definition>)>;

/// The current receipts, the retired receipt line hashes, and the run reports' SQL by its hash.
pub(crate) type LoadedReceipts = (Vec<Receipt>, Vec<String>, BTreeMap<String, String>);
