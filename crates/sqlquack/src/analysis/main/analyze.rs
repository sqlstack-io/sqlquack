//! `analyze`: parse, classify, convert or refuse.

use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::parsing::main::parse_source::parse_source;
use std::time::{Duration, Instant};

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::types::ConstructPolicy;

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_rules::conversion::main::convert_statement::convert_statement;
use sqlquack_rules::conversion::models::{ConversionOptions, Converted, Schema};

use crate::analysis::models::Analysis;

/// Parses, classifies, and converts each statement or refuses it.
pub fn analyze(
    sql: &str,
    dialect: SourceDialect,
    policy: &dyn ConstructPolicy,
    (schema, options): (&Schema, &ConversionOptions),
) -> Analysis {
    let statements = match parse_source(sql, dialect) {
        Ok(statements) => statements,
        Err(failure) => {
            return Analysis {
                parsed: false,
                statements: 0,
                occurrences: Vec::new(),
                refusals: vec![refuse_unparsed(&failure)],
                conversions: Vec::new(),
            };
        }
    };
    let mut analysis = Analysis {
        parsed: true,
        statements: statements.len(),
        occurrences: Vec::new(),
        refusals: Vec::new(),
        conversions: Vec::new(),
    };
    for statement in &statements {
        let occurrences = classify(statement, sql, dialect);
        let declared: Vec<Refusal> = refuse_constructs(&occurrences, policy)
            .into_iter()
            .filter(|refusal| refusal.category == RefusalCategory::NotExpressible)
            .collect();
        let refused = !declared.is_empty();
        analysis.refusals.extend(declared);
        analysis.occurrences.extend(occurrences);
        if !refused {
            let started = Instant::now();
            let converted =
                convert_statement(statement, (schema, options), ProvenanceLog::default());
            let elapsed = started.elapsed();
            match converted.and_then(|converted| within_budget(converted, (sql, elapsed), options))
            {
                Ok(converted) => analysis.conversions.push(converted),
                Err(refusal) => analysis.refusals.push(refusal),
            }
        }
    }
    analysis
}

/// `converted`, or a refusal when converting it outran the time or size budget.
fn within_budget(
    converted: Converted,
    (source, elapsed): (&str, Duration),
    options: &ConversionOptions,
) -> Result<Converted, Refusal> {
    let budget = options.budget;
    let emitted = converted.sql.len() as u64;
    let allowed = budget
        .minimum_bytes
        .max((source.len() as u64).saturating_mul(budget.size_ratio));
    let reason = if elapsed > Duration::from_millis(budget.milliseconds) {
        format!(
            "converting the statement took {} ms, beyond the budget of {} ms",
            elapsed.as_millis(),
            budget.milliseconds
        )
    } else if emitted > allowed {
        format!(
            "the DuckDB SQL is {emitted} bytes for {} bytes of source, beyond the budget of \
             {allowed} bytes ({} times the source, at least {})",
            source.len(),
            budget.size_ratio,
            budget.minimum_bytes
        )
    } else {
        return Ok(converted);
    };
    Err(Refusal {
        construct: None,
        location: None,
        category: RefusalCategory::ConversionBudget,
        reason,
    })
}
