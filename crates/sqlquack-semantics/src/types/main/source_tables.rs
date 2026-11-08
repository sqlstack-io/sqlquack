//! `source_tables`: the tables a source YAML file declares.

use crate::types::_helpers::source_file::source_relations;
use crate::types::models::SourceTables;

/// The tables `text` declares, with a warning for each source left out, or why it is not a source file.
pub fn source_tables(text: &str) -> Result<SourceTables, String> {
    source_relations(text)
}
