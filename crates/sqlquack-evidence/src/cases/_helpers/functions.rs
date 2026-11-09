//! The project functions a case defines: their checks and the Snowflake statements creating them.

use std::collections::BTreeSet;

use crate::cases::_helpers::loading::is_identifier;
use crate::cases::models::{CaseDefinition, CaseFunction, FunctionBodyLanguage, InputColumn};
use crate::constants::{FUNCTION_BODY_QUOTE, PYTHON_RUNTIME_VERSIONS};

/// `CREATE FUNCTION` for `function`, its body dollar-quoted.
pub(crate) fn create_function(function: &CaseFunction) -> String {
    let returns = match (&function.returns, &function.returns_table) {
        (_, Some(columns)) => format!("TABLE ({})", typed_list(columns)),
        (Some(data_type), None) => data_type.clone(),
        (None, None) => String::new(),
    };
    let language = match (
        function.language,
        &function.handler,
        &function.runtime_version,
    ) {
        (FunctionBodyLanguage::Python, Some(handler), Some(version)) => {
            format!("\nLANGUAGE PYTHON\nRUNTIME_VERSION = '{version}'\nHANDLER = '{handler}'")
        }
        _ => String::new(),
    };
    format!(
        "CREATE FUNCTION {}({})\nRETURNS {returns}{language}\nAS {FUNCTION_BODY_QUOTE}\n{}\n{FUNCTION_BODY_QUOTE}",
        function.name,
        typed_list(&function.arguments),
        function.body.trim()
    )
}

fn typed_list(columns: &[InputColumn]) -> String {
    columns
        .iter()
        .map(|column| format!("{} {}", column.name, column.data_type))
        .collect::<Vec<String>>()
        .join(", ")
}

/// Problems with a case's functions: names, one return form, and a body the quoting keeps intact.
pub(crate) fn function_problems(label: &str, case: &CaseDefinition) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let names: BTreeSet<&str> = case
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    if names.len() != case.functions.len() {
        problems.push(format!("{label}: repeats a function name"));
    }
    for function in &case.functions {
        let named = format!("{label}: function `{}`", function.name);
        let parameters_named = function
            .arguments
            .iter()
            .chain(function.returns_table.iter().flatten())
            .all(|column| is_identifier(&column.name));
        if !is_identifier(&function.name) || !parameters_named {
            problems.push(format!(
                "{named}: names, parameters and columns must be lower-case identifiers"
            ));
        }
        if function.returns.is_some() == function.returns_table.is_some() {
            problems.push(format!(
                "{named}: needs exactly one of `returns` and `returns_table`"
            ));
        }
        if function.body.trim().is_empty() || function.body.contains(FUNCTION_BODY_QUOTE) {
            problems.push(format!(
                "{named}: needs a body without `{FUNCTION_BODY_QUOTE}`"
            ));
        }
        problems.extend(python_problems(&named, function));
    }
    problems
}

/// A Python function needs a handler, a known runtime version and a scalar return; SQL neither.
fn python_problems(named: &str, function: &CaseFunction) -> Vec<String> {
    let python = function.language == FunctionBodyLanguage::Python;
    let handler_named = function.handler.as_deref().is_some_and(is_identifier);
    let runtime_known = function
        .runtime_version
        .as_deref()
        .is_some_and(|version| PYTHON_RUNTIME_VERSIONS.contains(&version));
    let mut problems: Vec<String> = Vec::new();
    if python && !(handler_named && runtime_known && function.returns_table.is_none()) {
        problems.push(format!(
            "{named}: a Python function needs a lower-case `handler`, a `runtime_version` of {} \
             and a scalar `returns`",
            PYTHON_RUNTIME_VERSIONS.join(", ")
        ));
    }
    if !python && (function.handler.is_some() || function.runtime_version.is_some()) {
        problems.push(format!(
            "{named}: only a Python function takes `handler` and `runtime_version`"
        ));
    }
    problems
}
