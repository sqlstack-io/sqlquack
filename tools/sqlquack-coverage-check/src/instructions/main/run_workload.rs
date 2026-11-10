//! `run_workload`: one workload in this process, for callgrind to count.

use std::hint::black_box;

use sqlquack_semantics::dialects::constants::SNOWFLAKE;
use sqlquack_semantics::lineage::models::LineageContext;
use sqlquack_semantics::statement_lineage_and_findings;
use sqlquack_syntax::{Dialect, Expression, ParseOptions};
use sqlquack_syntax_snowflake::parse_strict;

use crate::instructions::_helpers::workload::{Tables, statements, tables};
use crate::instructions::models::Workload;

/// Runs `workload` once; fails when a statement does not parse, so a count never covers less work.
pub fn run_workload(workload: Workload) -> Result<(), String> {
    let texts: Vec<String> = statements();
    let mut parsed: Vec<(&str, Vec<Expression>)> = Vec::with_capacity(texts.len());
    for (position, sql) in texts.iter().enumerate() {
        let statements: Vec<Expression> =
            parse_strict(sql, Dialect::Snowflake, &ParseOptions::default())
                .map_err(|failure| format!("workload statement {position}: {failure:?}"))?;
        parsed.push((sql, black_box(statements)));
    }
    if workload == Workload::Analysis {
        let schema: Tables = tables();
        for (sql, statements) in &parsed {
            let context: LineageContext<'_> = LineageContext {
                profile: &SNOWFLAKE,
                schema: &schema,
                sql: Some(sql),
            };
            for statement in statements {
                black_box(statement_lineage_and_findings(statement, context));
            }
        }
    }
    Ok(())
}
