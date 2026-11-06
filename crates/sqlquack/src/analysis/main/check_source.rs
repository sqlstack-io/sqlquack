//! `check_source`: what is wrong with some Snowflake SQL before any conversion.

use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_core::diagnostics::constants::UNKNOWN_FUNCTION;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::{SourceDialect, SourceLines};
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::main::snowflake_rejections::snowflake_rejections;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_semantics::dialects::constants::SNOWFLAKE;
use sqlquack_semantics::lineage::models::{LineageContext, NameFinding, SchemaCatalog};
use sqlquack_semantics::query_name_findings;
use sqlquack_semantics::types::models::Schema;
use sqlquack_syntax::{Dialect, ParseOptions};

use crate::analysis::constants::{
    AMBIGUOUS_COLUMN_FINDING, UNKNOWN_FUNCTION_FINDING, UNKNOWN_TABLE_FINDING,
};

/// The parse failure, what Snowflake would reject and, with `schema`, the names it lacks.
pub fn check_source(sql: &str, schema: Option<&Schema>) -> Result<Vec<Diagnostic>, String> {
    let lines = SourceLines::new(sql);
    let refusals = match parse_source(sql, SourceDialect::Snowflake) {
        Ok(statements) if statements.is_empty() => {
            return Err("the input holds no SQL statement".to_owned());
        }
        Ok(statements) => {
            let mut refusals: Vec<Refusal> = statements
                .iter()
                .flat_map(|statement| snowflake_rejections(statement, (sql, &lines)))
                .collect();
            if let (Some(schema), 1) = (schema, statements.len()) {
                refusals.extend(undeclared_names(sql, schema, &lines));
            }
            refusals
        }
        Err(failure) => vec![refuse_unparsed(&failure, &lines)],
    };
    Ok(diagnose(&refusals, sql))
}

/// The relations and functions the one statement in `sql` reads that `schema` does not declare.
fn undeclared_names(sql: &str, schema: &Schema, lines: &SourceLines) -> Vec<Refusal> {
    let catalog = SchemaCatalog::new(schema);
    let context = LineageContext {
        profile: &SNOWFLAKE,
        schema: &catalog,
        sql: Some(sql),
    };
    query_name_findings(sql, Dialect::Snowflake, &ParseOptions::default(), context)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|finding| undeclared(finding, lines))
        .collect()
}

/// The refusal for an unknown relation or function, or an ambiguous column; other findings are not checked here.
fn undeclared(finding: NameFinding, lines: &SourceLines) -> Option<Refusal> {
    let location = finding
        .span
        .map(|span| lines.location(span.start, span.end));
    let written = finding.written;
    match finding.code {
        UNKNOWN_TABLE_FINDING => Some(Refusal {
            construct: None,
            location,
            category: RefusalCategory::UnresolvedName,
            reason: format!("relation `{written}` is not in the schema"),
        }),
        UNKNOWN_FUNCTION_FINDING => Some(Refusal {
            construct: Some(Construct::new(ConstructFamily::Syntax, UNKNOWN_FUNCTION)),
            location,
            category: RefusalCategory::SnowflakeRejects,
            reason: format!("no Snowflake or schema function is named `{written}`"),
        }),
        AMBIGUOUS_COLUMN_FINDING => Some(Refusal {
            construct: None,
            location,
            category: RefusalCategory::UnresolvedName,
            reason: format!(
                "column `{written}` is ambiguous: more than one relation in scope has it; qualify it"
            ),
        }),
        _ => None,
    }
}
