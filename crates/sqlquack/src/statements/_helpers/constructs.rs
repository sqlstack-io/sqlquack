//! The constructs a recorded statement uses, the case's own functions as one project key.

use sqlquack_syntax::Dialect;
use std::collections::BTreeSet;

use sqlquack_diagnostics::classification::constants::PROJECT_FUNCTION_CONSTRUCT;
use sqlquack_diagnostics::classification::main::classify::classify;
use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::snowflake::classification::main::cast_type_name::cast_type_name;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::constants::{INPUT_RELATION, ORDINAL_COLUMN};
use sqlquack_parse::databricks::parse::main::parse::parse as parse_databricks;
use sqlquack_parse::parsing::main::parse_source::parse_source;
use sqlquack_policy::policy::models::EvidenceDialect;
use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::SourceLines;

use crate::statements::_helpers::spellings::{token_spellings, tree_spellings};

/// The upper-case names of the functions the case itself defines.
pub(crate) fn own_functions(case: &LoadedCase) -> BTreeSet<String> {
    case.functions
        .iter()
        .map(|function| function.name.to_ascii_uppercase())
        .collect()
}

/// `key`, or the project function key when it calls one of the case's own functions.
pub(crate) fn project_key(key: String, own: &BTreeSet<String>) -> String {
    let prefix = format!("{}:", ConstructFamily::Function.prefix());
    match key
        .strip_prefix(&prefix)
        .is_some_and(|name| own.contains(name))
    {
        true => Construct::new(ConstructFamily::Function, PROJECT_FUNCTION_CONSTRUCT).key(),
        false => key,
    }
}

/// Every construct and spelling key of the case's statement, own functions as one key; a declared compile error may not parse.
pub(crate) fn constructs(case: &LoadedCase, own: &BTreeSet<String>) -> Result<Vec<String>, String> {
    let sql = measured_sql(case);
    let lines = SourceLines::new(sql);
    let statements = match parsed(sql, case.source_dialect) {
        Ok(statements) => statements,
        Err(_) if case.compile_error || !case.source_dialect.is_snowflake() => Vec::new(),
        Err(failure) => {
            return Err(format!(
                "{}: a recorded statement does not parse: {failure}",
                case.reference
            ));
        }
    };
    let mut keys: BTreeSet<String> = token_spellings(sql);
    for statement in &statements {
        keys.extend(tree_spellings(statement));
        for occurrence in classify(statement, &lines, cast_type_name) {
            keys.insert(project_key(occurrence.construct.key(), own));
        }
    }
    Ok(keys.into_iter().collect())
}

/// The SQL whose constructs count: a Databricks case's own query, without the input fixture every case shares.
fn measured_sql(case: &LoadedCase) -> &str {
    if case.source_dialect.is_snowflake() {
        return &case.source_sql;
    }
    let mut names: Vec<&str> = vec![ORDINAL_COLUMN];
    names.extend(case.inputs.iter().map(|input| input.column.name.as_str()));
    let marker = format!(" AS {INPUT_RELATION}({})) ", names.join(", "));
    case.source_sql
        .split_once(&marker)
        .map_or(case.source_sql.as_str(), |(_, query)| query)
}

/// The statements of `sql` as its source dialect's parser reads them, or why it cannot.
fn parsed(sql: &str, dialect: EvidenceDialect) -> Result<Vec<Expression>, String> {
    match dialect {
        EvidenceDialect::Snowflake => {
            parse_source(sql, Dialect::Snowflake).map_err(|failure| format!("{failure:?}"))
        }
        EvidenceDialect::Databricks => parse_databricks(sql)
            .map(|parsed| parsed.statements)
            .map_err(|error| error.message),
    }
}
