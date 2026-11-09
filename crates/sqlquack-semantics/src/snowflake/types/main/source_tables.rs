//! `source_tables`: the tables a source YAML file declares.

use crate::snowflake::types::_helpers::source_file::source_relations;
use crate::snowflake::types::models::SourceTables;

/// The tables `text` declares, with a warning for each source left out, or why it is not a source file.
pub fn source_tables(text: &str) -> Result<SourceTables, String> {
    source_relations(text)
}
