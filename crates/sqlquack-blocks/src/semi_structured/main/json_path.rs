//! `json_path`: the value at a path into a VARIANT, SQL NULL when the path is absent.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::main::string_literal::string_literal;
use crate::semi_structured::constants::JSON_PATH;
use crate::semi_structured::models::PathStep;

/// `json_extract` along `steps`; keys holding `"` or `\` have no DuckDB path spelling.
pub fn json_path(sql: &str, steps: &[PathStep], log: &ProvenanceLog) -> Result<String, String> {
    log.building_block(JSON_PATH);
    let mut path = String::from("$");
    for step in steps {
        match step {
            PathStep::Key(key) if key.contains(['"', '\\']) => {
                return Err(format!("the path key `{key}` holds a quote or backslash"));
            }
            PathStep::Key(key) => path.push_str(&format!(".\"{key}\"")),
            PathStep::Index(index) => path.push_str(&format!("[{index}]")),
        }
    }
    Ok(format!(
        "json_extract({sql}, {})",
        string_literal(&path, log)
    ))
}
