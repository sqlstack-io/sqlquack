//! Function signatures from SQLBuild's function files: a SQL `FUNCTION (...)` header or a Python `@udf(...)`.

use serde_json::{Map, Value, json};

use crate::analysis::constants::{OVERRIDE_ARGUMENTS, OVERRIDE_RETURNS};
use crate::project::_helpers::python_literal::python_keywords;
use crate::project::constants::{
    SQL_FUNCTION_KEYS, TABLE_RETURNS, UDF_DECORATOR, UDF_IMPORTS, UDF_KEYS,
};
use crate::project::main::function_header::function_header;
use crate::project::main::named_types::named_types;

/// The schema declaration of the SQL function file `text`.
pub(crate) fn sql_function(text: &str) -> Result<Value, String> {
    let (properties, _body) = function_header(text)?;
    let mut function: Map<String, Value> = Map::new();
    function.insert("language".to_owned(), json!("sql"));
    for (key, value) in properties {
        match key.as_str() {
            OVERRIDE_ARGUMENTS => {
                function.insert(key, pairs(named_types(&value)?));
            }
            OVERRIDE_RETURNS => {
                let returns = match value.get(..TABLE_RETURNS.len()) {
                    Some(head) if head.eq_ignore_ascii_case(TABLE_RETURNS) => {
                        let columns = value[TABLE_RETURNS.len()..].trim();
                        json!({ "table": pairs(named_types(columns)?) })
                    }
                    _ => json!(value),
                };
                function.insert(key, returns);
            }
            known if SQL_FUNCTION_KEYS.contains(&known) => {}
            other => return Err(format!("the property `{other}` is not understood")),
        }
    }
    if !function.contains_key(OVERRIDE_RETURNS) {
        return Err("needs `returns <TYPE>`".to_owned());
    }
    Ok(Value::Object(function))
}

/// The schema declaration of the Python function file `text`; errors name the decorator's line.
pub(crate) fn python_function(text: &str) -> Result<Value, String> {
    let imported = text.lines().any(|line| {
        let words: Vec<&str> = line.split_whitespace().collect();
        matches!(words.as_slice(), ["from", module, "import", names @ ..]
            if UDF_IMPORTS.contains(module)
                && names.iter().any(|name| name.trim_end_matches(',') == "udf"))
    });
    let Some((line, start)) = text
        .lines()
        .scan(0_usize, |offset, line| {
            let start = *offset;
            *offset += line.len() + 1;
            Some((line, start))
        })
        .enumerate()
        .find(|(_, (line, _))| line.trim_start().starts_with(UDF_DECORATOR))
        .map(|(index, (line, start))| (index + 1, start + line.find('(').unwrap_or_default()))
    else {
        return Err(format!(
            "has no {UDF_DECORATOR}...) decorator; mark the function as SQLBuild does"
        ));
    };
    if !imported {
        return Err(format!(
            "line {line}: import udf from {} first",
            UDF_IMPORTS.join(" or ")
        ));
    }
    let keywords =
        python_keywords(&text[start..]).map_err(|reason| format!("line {line}: {reason}"))?;
    let mut function: Map<String, Value> = Map::new();
    function.insert("language".to_owned(), json!("python"));
    for (key, value) in keywords {
        match (key.as_str(), value) {
            (OVERRIDE_ARGUMENTS, Value::Object(arguments)) => {
                let mut named: Vec<(String, String)> = Vec::new();
                for (name, type_name) in arguments {
                    let Value::String(type_name) = type_name else {
                        return Err(format!(
                            "line {line}: the argument {name}'s type is not text"
                        ));
                    };
                    named.push((name, type_name));
                }
                function.insert(key, pairs(named));
            }
            (OVERRIDE_RETURNS, Value::String(type_name)) => {
                function.insert(key, json!(type_name));
            }
            (OVERRIDE_ARGUMENTS | OVERRIDE_RETURNS, _) => {
                return Err(format!(
                    "line {line}: `{key}` must be a dict of names to types (arguments) or a type \
                     (returns)"
                ));
            }
            (known, _) if UDF_KEYS.contains(&known) => {}
            (other, _) => {
                return Err(format!(
                    "line {line}: the keyword `{other}` is not understood"
                ));
            }
        }
    }
    if !function.contains_key(OVERRIDE_RETURNS) {
        return Err(format!("line {line}: needs returns=\"<TYPE>\""));
    }
    Ok(Value::Object(function))
}

/// Name and type pairs as the schema file writes columns.
fn pairs(named: Vec<(String, String)>) -> Value {
    Value::Array(
        named
            .into_iter()
            .map(|(name, type_name)| json!([name, type_name]))
            .collect(),
    )
}
