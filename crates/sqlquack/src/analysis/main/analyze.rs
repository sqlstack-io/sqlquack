//! `analyze`: parse, check fidelity, classify and refuse.

use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_core::refusals::main::refuse_infidelity::refuse_infidelity;
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::types::ConstructPolicy;
use sqlquack_core::reparse::main::check_source_fidelity::check_source_fidelity;

use crate::analysis::models::Analysis;

/// Parses, checks print-back fidelity, classifies, and refuses every construct no rule converts.
pub fn analyze(sql: &str, dialect: SourceDialect, policy: &dyn ConstructPolicy) -> Analysis {
    let statements = match parse_source(sql, dialect) {
        Ok(statements) => statements,
        Err(failure) => {
            return Analysis {
                parsed: false,
                statements: 0,
                occurrences: Vec::new(),
                fidelity: Vec::new(),
                refusals: vec![refuse_unparsed(&failure)],
            };
        }
    };
    let mut analysis = Analysis {
        parsed: true,
        statements: statements.len(),
        occurrences: Vec::new(),
        fidelity: Vec::with_capacity(statements.len()),
        refusals: Vec::new(),
    };
    for statement in &statements {
        let outcome = check_source_fidelity(statement, dialect);
        analysis.refusals.extend(refuse_infidelity(&outcome));
        analysis.fidelity.push(outcome);
        let occurrences = classify(statement, sql, dialect);
        analysis
            .refusals
            .extend(refuse_constructs(&occurrences, policy));
        analysis.occurrences.extend(occurrences);
    }
    analysis
}
